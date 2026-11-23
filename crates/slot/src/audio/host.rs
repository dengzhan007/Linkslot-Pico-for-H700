use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::ring::Ring;
use super::sink::{AudioError, AudioSink};

pub struct HostAudio {
    ring: Arc<Ring>,
    stream: Option<u64>,
}

enum Cmd {
    Open {
        ring: Arc<Ring>,
        sample_rate: u32,
        reply: mpsc::Sender<Result<u64, AudioError>>,
    },
    Close {
        id: u64,
        reply: mpsc::Sender<()>,
    },
}

/// cpal caches the WASAPI device enumerator in a process-wide static, and that
/// enumerator belongs to the COM apartment of the thread that created it. A
/// fresh "slot-audio" thread per session therefore reused a dead apartment and
/// crashed inside `GetDefaultAudioEndpoint` on the second session. One
/// long-lived thread now owns every stream, so the enumerator's apartment
/// lives as long as the process.
fn worker() -> &'static mpsc::Sender<Cmd> {
    static WORKER: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Cmd>();
        std::thread::Builder::new()
            .name("slot-audio".into())
            .spawn(move || audio_worker(rx))
            .expect("spawn the audio thread");
        tx
    })
}

fn audio_worker(rx: mpsc::Receiver<Cmd>) {
    let mut streams: HashMap<u64, cpal::Stream> = HashMap::new();
    let mut next_id = 0u64;
    for cmd in rx {
        match cmd {
            Cmd::Open {
                ring,
                sample_rate,
                reply,
            } => {
                let opened =
                    std::panic::catch_unwind(AssertUnwindSafe(|| open_stream(&ring, sample_rate)));
                let result = match opened {
                    Ok(Ok(stream)) => {
                        let id = next_id;
                        next_id += 1;
                        streams.insert(id, stream);
                        Ok(id)
                    }
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(AudioError::Device("the audio host panicked".into())),
                };
                let _ = reply.send(result);
            }
            Cmd::Close { id, reply } => {
                streams.remove(&id);
                let _ = reply.send(());
            }
        }
    }
}

impl HostAudio {
    pub fn new() -> Self {
        HostAudio {
            ring: Arc::new(Ring::new(0)),
            stream: None,
        }
    }

    fn close(&mut self) {
        let Some(id) = self.stream.take() else {
            return;
        };
        let (reply, done) = mpsc::channel();
        if worker().send(Cmd::Close { id, reply }).is_ok() {
            let _ = done.recv();
        }
    }
}

impl Default for HostAudio {
    fn default() -> Self {
        HostAudio::new()
    }
}

impl Drop for HostAudio {
    fn drop(&mut self) {
        self.close();
    }
}

impl AudioSink for HostAudio {
    fn open(&mut self, sample_rate: u32) -> Result<(), AudioError> {
        self.close();
        let (reply, ready) = mpsc::channel();
        worker()
            .send(Cmd::Open {
                ring: self.ring.clone(),
                sample_rate,
                reply,
            })
            .map_err(|_| AudioError::Device("the audio thread is gone".into()))?;
        match ready.recv() {
            Ok(Ok(id)) => {
                self.stream = Some(id);
                Ok(())
            }
            Ok(Err(error)) => Err(error),
            Err(_) => Err(AudioError::Device("the audio thread is gone".into())),
        }
    }

    fn ring(&self) -> Arc<Ring> {
        self.ring.clone()
    }
}

fn open_stream(ring: &Arc<Ring>, sample_rate: u32) -> Result<cpal::Stream, AudioError> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
    let fallback = device
        .default_output_config()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    let mut last = None;
    for config in [exact_rate(&device, sample_rate), Some(fallback)]
        .into_iter()
        .flatten()
    {
        match build(&device, config, ring) {
            Ok(stream) => return Ok(stream),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or(AudioError::NoDevice))
}

fn exact_rate(device: &cpal::Device, sample_rate: u32) -> Option<cpal::SupportedStreamConfig> {
    let want = cpal::SampleRate(sample_rate);
    device.supported_output_configs().ok()?.find_map(|r| {
        let usable = r.channels() == 2
            && r.sample_format() == cpal::SampleFormat::F32
            && r.min_sample_rate() <= want
            && want <= r.max_sample_rate();
        usable.then(|| r.with_sample_rate(want))
    })
}

const PERIOD_FRAMES: u32 = 512;

fn build(
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    ring: &Arc<Ring>,
) -> Result<cpal::Stream, AudioError> {
    if config.channels() != 2 || config.sample_format() != cpal::SampleFormat::F32 {
        return Err(AudioError::Config(format!(
            "{} channels, {}",
            config.channels(),
            config.sample_format()
        )));
    }
    ring.reopen(config.sample_rate().0);
    let mut cfg = config.config();
    let fixed = match *config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => Some(PERIOD_FRAMES.clamp(min, max)),
        cpal::SupportedBufferSize::Unknown => None,
    };
    let mut last = None;
    for size in [
        fixed.map(cpal::BufferSize::Fixed),
        Some(cpal::BufferSize::Default),
    ]
    .into_iter()
    .flatten()
    {
        cfg.buffer_size = size;
        match start(device, &cfg, ring) {
            Ok(s) => return Ok(s),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or(AudioError::NoDevice))
}

fn start(
    device: &cpal::Device,
    cfg: &cpal::StreamConfig,
    ring: &Arc<Ring>,
) -> Result<cpal::Stream, AudioError> {
    let ring = ring.clone();
    let mut scratch: Vec<i16> = Vec::new();
    let stream = device
        .build_output_stream(
            cfg,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if scratch.len() < out.len() {
                    scratch.resize(out.len(), 0);
                }
                let taken = &mut scratch[..out.len()];
                ring.fill(taken);
                for (o, s) in out.iter_mut().zip(taken.iter()) {
                    *o = f32::from(*s) / 32768.0;
                }
            },
            |e| eprintln!("slot: audio: {e}"),
            None,
        )
        .map_err(|e| AudioError::Device(e.to_string()))?;
    stream
        .play()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    Ok(stream)
}
