use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use slot_gfx::{Compositor, TexId};
use slot_store::Cart;
use slot_ui::{cart_face_with, CartFace};

use crate::app::{App, FaceWant};
use crate::label_cache::label_art;

const WORKERS: usize = 2;

type Key = (usize, usize);

#[derive(Default)]
struct Queue {
    jobs: VecDeque<(Key, Cart, u64)>,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

pub struct CartFaces {
    queue: Shared,
    built: Receiver<(Key, u64, CartFace)>,
    pending: HashSet<Key>,
    asked: Vec<FaceWant>,
    free: Vec<TexId>,
    generation: u64,
}

impl CartFaces {
    pub fn spawn(root: PathBuf) -> Self {
        let queue: Shared = Arc::default();
        let (outbox, built) = mpsc::channel();
        for n in 0..WORKERS {
            let (queue, outbox, root) = (queue.clone(), outbox.clone(), root.clone());
            let spawned = thread::Builder::new()
                .name(format!("slot-carts-{n}"))
                .spawn(move || work(&queue, &outbox, &root));
            if let Err(e) = spawned {
                eprintln!("slot: cart faces: worker thread failed to start: {e}");
            }
        }
        CartFaces {
            queue,
            built,
            pending: HashSet::new(),
            asked: Vec::new(),
            free: Vec::new(),
            generation: 0,
        }
    }

    pub fn sync(&mut self, app: &mut App, compositor: &mut Compositor) {
        let generation = app.face_generation();
        if generation != self.generation {
            // A folder switch or a cover refresh retired every in-flight job.
            // Clearing "asked" too makes the new wants schedule immediately
            // instead of waiting for a key that may now mean another cart.
            self.generation = generation;
            self.pending.clear();
            self.asked.clear();
            let (lock, _) = &*self.queue;
            lock.lock()
                .unwrap_or_else(|e| e.into_inner())
                .jobs
                .clear();
        }
        self.free.extend(app.shed_faces());
        let invalidated = app.take_invalidated_faces();
        if !invalidated.is_empty() {
            self.free.extend(invalidated);
            // Drop both queued work and the "asked" snapshot: every face must
            // be rebuilt from the new label, and a worker already inside the
            // old job must not satisfy the new generation.
            self.pending.clear();
            self.asked.clear();
        }
        while let Ok((key, generation, face)) = self.built.try_recv() {
            self.place(app, compositor, key, generation, face);
        }
        let wants = app.face_wants();
        if wants != self.asked {
            self.requeue(app, &wants);
            self.asked = wants;
        }
        let mut waiting: Vec<Key> = self
            .asked
            .iter()
            .filter(|w| w.on_screen)
            .map(|w| (w.shelf, w.index))
            .collect();
        while !waiting.is_empty() {
            let Ok((key, generation, face)) = self.built.recv() else {
                return;
            };
            waiting.retain(|k| *k != key);
            self.place(app, compositor, key, generation, face);
        }
    }

    fn requeue(&mut self, app: &App, wants: &[FaceWant]) {
        let (lock, wake) = &*self.queue;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        for (key, _, _) in queue.jobs.drain(..) {
            self.pending.remove(&key);
        }
        let generation = app.face_generation();
        for w in wants {
            let key = (w.shelf, w.index);
            if self.pending.contains(&key) {
                continue;
            }
            if let Some(cart) = app.face_cart(w.shelf, w.index) {
                let cart = cart.clone();
                queue.jobs.push_back((key, cart, generation));
                self.pending.insert(key);
            }
        }
        wake.notify_all();
    }

    fn place(
        &mut self,
        app: &mut App,
        compositor: &mut Compositor,
        key: Key,
        generation: u64,
        face: CartFace,
    ) {
        if generation != app.face_generation() {
            return;
        }
        self.pending.remove(&key);
        self.asked.retain(|w| (w.shelf, w.index) != key);
        let tex = match self.free.pop() {
            Some(tex) => {
                compositor.update_texture(tex, face.w, face.h, &face.rgba);
                tex
            }
            None => compositor.create_texture(face.w, face.h, &face.rgba),
        };
        self.free.extend(app.set_cart_face(key.0, key.1, tex));
    }
}

impl Drop for CartFaces {
    fn drop(&mut self) {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(queue: &Shared, outbox: &Sender<(Key, u64, CartFace)>, root: &Path) {
    let (lock, wake) = &**queue;
    loop {
        let (key, cart, generation) = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if q.closed {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    break job;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let face = cart_face_with(&cart, label_art(root, &cart));
        if outbox.send((key, generation, face)).is_err() {
            return;
        }
    }
}

