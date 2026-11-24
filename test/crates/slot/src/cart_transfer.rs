//! Cartridge-to-cartridge transfer mode.
//!
//! The ROM and the cartridge's own side files (cover label, intro text/video,
//! shell choice) move to the other device.  Battery saves, resume states,
//! rewind data and playtime stay where they were: those are the *game's* save
//! data, not the cartridge itself.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use slot_retro::LinkChannel;
use slot_store::{Cart, Core, Outline, Platform, ShellChoice, ShellFinish};
use crate::rom_id::{self, Digest};

use crate::link_radio::LinkRole;
use crate::link_start::{LinkFail, LinkProgress, LinkStarter, LinkTransport};

const MAGIC: &[u8; 4] = b"SCX4";
const KIND_OFFER: u8 = 1;
const KIND_ACCEPT: u8 = 2;
const KIND_REJECT: u8 = 3;
const KIND_DATA: u8 = 4;
const KIND_DONE: u8 = 5;
const KIND_ACK: u8 = 6;
const KIND_CANCEL: u8 = 7;
const KIND_DUPLICATE: u8 = 8;
const CHUNK: usize = 32 * 1024;

const ASSET_ROM: u8 = 0;
const ASSET_LABEL: u8 = 1;
const ASSET_INTRO: u8 = 2;
const ASSET_VIDEO: u8 = 3;
const ASSET_SAV: u8 = 4;
const ASSET_STATE: u8 = 5;
const ASSET_THUMB: u8 = 6;

/// What one transfer carries.
///
/// The cartridge is the ROM plus its side material (cover, intro, video,
/// shell).  The *game's* save data — battery save, resume state and its
/// thumbnail — is a separate group the player can send on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransferMode {
    /// ROM, cover, intro, video and the shell colour.  What every build did
    /// before this choice existed, so it stays the default.
    #[default]
    CartOnly,
    /// Only the game's save data.
    SaveOnly,
    /// Everything above.
    All,
}

impl TransferMode {
    pub fn code(self) -> u8 {
        match self {
            TransferMode::CartOnly => 0,
            TransferMode::SaveOnly => 1,
            TransferMode::All => 2,
        }
    }

    pub fn from_code(code: u8) -> TransferMode {
        match code {
            1 => TransferMode::SaveOnly,
            2 => TransferMode::All,
            _ => TransferMode::CartOnly,
        }
    }

    /// Whether this mode moves the cartridge itself.
    pub fn sends_cart(self) -> bool {
        !matches!(self, TransferMode::SaveOnly)
    }

    /// Whether this mode moves `.sav` / `resume.state` / the thumbnail.
    pub fn sends_save(self) -> bool {
        !matches!(self, TransferMode::CartOnly)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    Connecting,
    Shelf,
    OfferSent,
    Incoming,
    /// The ROM is already local; only some parts (side material and/or save
    /// data) are coming.
    IncomingPartial,
    Sending,
    Receiving,
    Duplicate,
    Done,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Offer {
    pub platform: Platform,
    pub extension: String,
    pub name: String,
    pub size: u64,
    /// Full SHA-256 of the source ROM.
    pub hash: Digest,
    pub assets: Vec<AssetMeta>,
    pub shell: Option<ShellChoice>,
    /// What the sender chose to include.
    pub mode: TransferMode,
    /// The core that wrote the resume state, so the receiver can put it back
    /// under the same `States/<platform>/<core>/<stem>/` directory.  A state
    /// written by one core will not load in another.
    pub core: Core,
}

impl Offer {
    /// The asset kinds the local cart still needs: side material it does not
    /// have locally, plus every save kind the offer carries (the player chose
    /// to overwrite this device's save data, so the confirm step is the guard).
    /// The ROM itself is never part of a partial transfer.
    pub fn parts_missing(&self, root: &Path, cart: &Cart) -> Vec<u8> {
        let labels = root.join("Labels").join(cart.platform.dir_name());
        let info = root.join("Info").join(cart.platform.dir_name());
        let mut missing = Vec::new();
        for asset in &self.assets {
            let present = match asset.kind {
                ASSET_LABEL => cart
                    .label
                    .as_ref()
                    .is_some_and(|path| path.is_file())
                    || labels.join(format!("{}.png", cart.stem)).is_file(),
                ASSET_INTRO => info.join(format!("{}.txt", cart.stem)).is_file(),
                ASSET_VIDEO => info.join(format!("{}.mp4", cart.stem)).is_file(),
                ASSET_SAV | ASSET_STATE | ASSET_THUMB => false,
                _ => true,
            };
            if !present && asset.size > 0 {
                missing.push(asset.kind);
            }
        }
        missing
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AssetMeta {
    pub kind: u8,
    pub size: u64,
}

#[derive(Clone, Debug)]
struct SourceAsset {
    meta: AssetMeta,
    path: PathBuf,
}

struct Selected {
    offer: Offer,
    assets: Vec<SourceAsset>,
}

struct IncomingAsset {
    meta: AssetMeta,
    temp: PathBuf,
    target: PathBuf,
    file: File,
    received: u64,
}

pub struct Transfer {
    pub host: bool,
    pub stage: Stage,
    /// What this device sends when the player picks a cartridge.
    pub mode: TransferMode,
    starter: Option<LinkStarter>,
    wire: Option<Box<dyn LinkChannel>>,
    selected: Option<Selected>,
    pub incoming: Option<Offer>,
    pub progress: f32,
    pub peer: Option<String>,
    pub received: bool,
    pub duplicate_receiver: bool,
    /// Asset kinds the receiver asked for when the ROM is already local.
    pub parts: Vec<u8>,
    pub parts_only: bool,
    parts_stem: Option<String>,
    /// Old files copied aside before an incoming save replaced them.
    pub backups: Vec<PathBuf>,
    pub error: Option<String>,
    sending_index: usize,
    sending_at: u64,
    receiving: Vec<IncomingAsset>,
    receiving_total: u64,
    received_total: u64,
    target_stem: Option<String>,
    pub done_processed: bool,
}

impl Transfer {
    pub fn start(host: bool, port: u16, transport: LinkTransport, mode: TransferMode) -> Transfer {
        let role = if host { LinkRole::Host } else { LinkRole::Join };
        Transfer {
            host,
            stage: Stage::Connecting,
            mode,
            starter: Some(LinkStarter::spawn_transport(role, port, transport)),
            wire: None,
            selected: None,
            incoming: None,
            progress: 0.0,
            peer: None,
            received: false,
            duplicate_receiver: false,
            parts: Vec::new(),
            parts_only: false,
            parts_stem: None,
            backups: Vec::new(),
            error: None,
            sending_index: 0,
            sending_at: 0,
            receiving: Vec::new(),
            receiving_total: 0,
            received_total: 0,
            target_stem: None,
            done_processed: false,
        }
    }

    /// Change what the next cartridge sends.  Only meaningful in the room,
    /// before a cartridge is picked; a change mid-transfer would be a lie.
    pub fn set_mode(&mut self, mode: TransferMode) {
        if self.stage == Stage::Shelf {
            self.mode = mode;
        }
    }

    pub fn cancel(&mut self) {
        if let Some(starter) = &mut self.starter {
            starter.cancel();
        }
        if let Some(wire) = self.wire.as_mut() {
            wire.send(0, &message(KIND_CANCEL, &[]));
            wire.send_end();
        }
        self.abort_receive();
    }

    /// One frame of matching/transport upkeep.  `root` is only used by the
    /// receiver when it accepts an offer.
    pub fn update(&mut self, root: &Path) {
        if let Some(starter) = &mut self.starter {
            match starter.poll() {
                None => return,
                Some(LinkProgress::At(_)) => return,
                Some(LinkProgress::Ready(wire)) => {
                    self.wire = Some(Box::new(wire));
                    self.starter = None;
                    self.stage = Stage::Shelf;
                    crate::link_log::debug("transfer: transport matched");
                }
                Some(LinkProgress::Failed(fail)) => {
                    self.starter = None;
                    self.stage = Stage::Failed;
                    self.error = Some(match fail {
                        LinkFail::Cancelled => "已取消".into(),
                        LinkFail::NobodyCame => "没有找到另一台掌机".into(),
                        LinkFail::PeerVanished => "对方已断开".into(),
                        other => format!("匹配失败：{}", other.line()),
                    });
                }
            }
        }
        // Drain first: the receiver may send KIND_DUPLICATE and close in the
        // same tick.  Checking is_closed before try_recv would turn that
        // explicit refusal into a generic "对方已断开" and lose the reason.
        let mut frames = Vec::new();
        if let Some(wire) = self.wire.as_mut() {
            while let Some(frame) = wire.try_recv() {
                frames.push(frame);
            }
        }
        for frame in frames {
            self.handle_frame(root, &frame);
        }
        let closed = self.wire.as_ref().is_some_and(|wire| wire.is_closed());
        if closed
            && !matches!(
                self.stage,
                Stage::Shelf | Stage::Duplicate | Stage::Done
            )
        {
            self.stage = Stage::Failed;
            self.error = Some("对方已断开".into());
            self.abort_receive();
            return;
        }
        if self.stage == Stage::Sending {
            let mut wire = self.wire.take();
            if let Some(wire) = wire.as_mut() {
                self.pump_send(wire);
            }
            self.wire = wire;
        }
    }

    /// Asset labels for the transfer overlay.  Partial transfers show only the
    /// parts that are actually coming, never the ROM.
    pub fn display_assets(&self) -> Vec<&'static str> {
        let kinds: Vec<u8> = if !self.parts.is_empty() {
            self.parts.clone()
        } else if let Some(selected) = &self.selected {
            selected.assets.iter().map(|asset| asset.meta.kind).collect()
        } else if let Some(offer) = &self.incoming {
            offer.assets.iter().map(|asset| asset.kind).collect()
        } else {
            Vec::new()
        };
        kinds.into_iter().map(asset_label).collect()
    }

    pub fn select(&mut self, root: &Path, cart: &Cart, hash: Digest) -> Result<(), String> {
        if self.stage != Stage::Shelf || self.wire.is_none() {
            return Err("对方还没有就绪".into());
        }
        let extension = cart
            .rom
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| "这张卡没有可传输的扩展名".to_string())?
            .to_ascii_lowercase();
        if !cart.platform.extensions().contains(&extension.as_str()) {
            return Err("不支持这张卡的格式".into());
        }
        let assets = source_assets(root, cart, self.mode)?;
        if assets.is_empty() {
            return Err(if self.mode.sends_save() {
                "这张卡还没有存档可发送".into()
            } else {
                "这张卡还没有可发送的内容".into()
            });
        }
        let size = assets.iter().map(|asset| asset.meta.size).sum::<u64>();
        // Only the cartridge carries a shell colour; a save-only transfer must
        // not restyle whatever copy of the game the other device already has.
        let shell = self.mode.sends_cart().then(|| {
            cart.shell.unwrap_or_else(|| {
                let effective = slot_ui::shell_for(cart);
                ShellChoice {
                    outline: Outline::Auto,
                    colour: effective.colour,
                    finish: match effective.finish {
                        slot_ui::Finish::Solid => ShellFinish::Solid,
                        slot_ui::Finish::Translucent => ShellFinish::Clear,
                        slot_ui::Finish::Glitter => ShellFinish::Glitter,
                    },
                }
            })
        });
        let offer = Offer {
            platform: cart.platform,
            extension,
            name: cart.stem.clone(),
            size,
            hash,
            assets: assets.iter().map(|asset| asset.meta).collect(),
            shell,
            mode: self.mode,
            core: slot_store::core_for_platform(root, &cart.stem, cart.platform),
        };
        self.send_offer(&offer)?;
        self.selected = Some(Selected { offer, assets });
        self.stage = Stage::OfferSent;
        self.sending_index = 0;
        self.sending_at = 0;
        self.progress = 0.0;
        Ok(())
    }

    pub fn accept(&mut self, root: &Path) -> Result<(), String> {
        let result = self.accept_inner(root);
        if result.is_err() {
            // The sender stays in OfferSent until it hears something; a local
            // error must reject the offer instead of leaving it waiting.
            self.send_message(KIND_REJECT);
        }
        result
    }

    fn accept_inner(&mut self, root: &Path) -> Result<(), String> {
        self.parts_only = false;
        self.parts_stem = None;
        self.parts.clear();
        self.backups.clear();
        let offer = self
            .incoming
            .take()
            .ok_or_else(|| "没有待接收的卡带".to_string())?;
        if offer.size > 64 * 1024 * 1024 {
            return Err("卡带超过 64MB 上限".into());
        }
        if !offer
            .platform
            .extensions()
            .contains(&offer.extension.as_str())
            || !offer
                .extension
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err("对方发来的卡带格式不受支持".into());
        }
        if offer.assets.is_empty() {
            return Err("对方没有发送任何内容".into());
        }
        // A save-only offer carries no ROM by design.  Everything else must
        // carry one, because the receiver needs a cartridge to seat the save
        // data against.
        if offer.mode.sends_cart() && !offer.assets.iter().any(|a| a.kind == ASSET_ROM) {
            return Err("对方没有发送 ROM".into());
        }
        let stem = unique_stem(root, offer.platform, &offer.name);
        let mut receiving = Vec::new();
        for asset in &offer.assets {
            let (temp, target) = asset_paths(
                root,
                offer.platform,
                &stem,
                &offer.extension,
                asset.kind,
                offer.core,
            )?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败：{e}"))?;
            }
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| format!("创建临时卡带文件失败：{e}"))?;
            receiving.push(IncomingAsset {
                meta: *asset,
                temp,
                target,
                file,
                received: 0,
            });
        }
        self.receiving_total = offer.size;
        self.received_total = 0;
        self.receiving = receiving;
        self.received = true;
        self.target_stem = Some(stem);
        self.incoming = Some(offer.clone());
        self.stage = Stage::Receiving;
        self.progress = 0.0;
        self.send_message(KIND_ACCEPT);
        Ok(())
    }

    /// The receiver found the same ROM: do not copy it, only ask for the parts
    /// it still needs (side material it lacks, save data the player chose to
    /// overwrite).
    pub fn offer_parts(&mut self, stem: String, parts: Vec<u8>) {
        if self.stage != Stage::Incoming || parts.is_empty() {
            return;
        }
        self.parts_stem = Some(stem);
        self.parts = parts;
        self.stage = Stage::IncomingPartial;
    }

    pub fn accept_parts(&mut self, root: &Path) -> Result<(), String> {
        let offer = self
            .incoming
            .take()
            .ok_or_else(|| "没有待接收的内容".to_string())?;
        let stem = self
            .parts_stem
            .clone()
            .ok_or_else(|| "没有本地卡带目标".to_string())?;
        let parts = self.parts.clone();
        if parts.is_empty() {
            return Err("没有待接收的内容".to_string());
        }
        let mut mask = 0u8;
        let mut assets = Vec::new();
        for asset in &offer.assets {
            let wanted = parts.contains(&asset.kind) && asset.kind != ASSET_ROM;
            if wanted {
                mask |= 1u8 << asset.kind;
                assets.push(*asset);
            }
        }
        if mask == 0 {
            return Err("对方没有可补充的内容".to_string());
        }
        let total = assets.iter().map(|asset| asset.size).sum::<u64>();
        if total > 64 * 1024 * 1024 {
            return Err("补充内容超过 64MB 上限".into());
        }
        let mut receiving = Vec::new();
        for asset in &assets {
            let (temp, target) = asset_paths(
                root,
                offer.platform,
                &stem,
                &offer.extension,
                asset.kind,
                offer.core,
            )?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("创建目录失败：{error}"))?;
            }
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| format!("创建临时资料文件失败：{error}"))?;
            receiving.push(IncomingAsset {
                meta: *asset,
                temp,
                target,
                file,
                received: 0,
            });
        }
        self.receiving_total = total;
        self.received_total = 0;
        self.receiving = receiving;
        self.received = true;
        self.parts_only = true;
        self.target_stem = Some(stem);
        self.incoming = Some(offer);
        self.stage = Stage::Receiving;
        self.progress = 0.0;
        self.send_payload(KIND_ACCEPT, &[mask]);
        Ok(())
    }

    pub fn reject(&mut self) {
        self.incoming = None;
        self.stage = Stage::Shelf;
        self.send_message(KIND_REJECT);
    }

    pub fn cancel_offer(&mut self) {
        if self.stage == Stage::OfferSent {
            self.send_message(KIND_CANCEL);
            self.selected = None;
            self.stage = Stage::Shelf;
            self.progress = 0.0;
        }
    }

    /// Final on-disk stem chosen by the receiver (may have a collision suffix).
    pub fn target_stem(&self) -> Option<&str> {
        self.target_stem.as_deref()
    }

    pub fn reset_after_done(&mut self) {
        if self.stage == Stage::Done {
            self.selected = None;
            self.incoming = None;
            self.parts_only = false;
            self.parts_stem = None;
            self.parts.clear();
            self.progress = 0.0;
            self.stage = Stage::Shelf;
            self.done_processed = false;
        }
    }

    /// Refuse an incoming offer the player cannot act on ("this device does
    /// not own that cartridge"), so the sender stops waiting instead of
    /// sitting in `OfferSent` until it times out.
    pub fn fail_with(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.stage = Stage::Failed;
        self.send_message(KIND_REJECT);
    }

    /// The App found the offered hash in its precomputed local index.  The
    /// receiver sends this before asking the player to accept anything.
    pub fn mark_duplicate(&mut self, receiver: bool) {
        self.error = Some(if receiver {
            "本机已拥有当前卡带".into()
        } else {
            "对方已拥有当前卡带".into()
        });
        self.duplicate_receiver = receiver;
        self.stage = Stage::Duplicate;
        self.send_message(KIND_DUPLICATE);
    }

    pub fn reset_after_duplicate(&mut self) {
        if self.stage == Stage::Duplicate {
            self.selected = None;
            self.incoming = None;
            self.progress = 0.0;
            self.error = None;
            self.duplicate_receiver = false;
            self.stage = Stage::Shelf;
        }
    }

    pub fn take_done(&mut self) -> bool {
        std::mem::take(&mut self.done_processed)
    }

    fn send_message(&mut self, kind: u8) {
        self.send_payload(kind, &[]);
    }

    fn send_payload(&mut self, kind: u8, payload: &[u8]) {
        if let Some(wire) = self.wire.as_mut() {
            wire.send(0, &message(kind, payload));
        }
    }

    fn send_offer(&mut self, offer: &Offer) -> Result<(), String> {
        let mut payload = Vec::new();
        payload.push(offer.platform.index() as u8);
        let ext = offer.extension.as_bytes();
        payload.push(ext.len().min(255) as u8);
        payload.extend_from_slice(&ext[..ext.len().min(255)]);
        let name = offer.name.as_bytes();
        payload.extend_from_slice(&(name.len().min(u16::MAX as usize) as u16).to_le_bytes());
        payload.extend_from_slice(&name[..name.len().min(u16::MAX as usize)]);
        payload.extend_from_slice(&offer.size.to_le_bytes());
        payload.extend_from_slice(&offer.hash);
        payload.push(offer.assets.len().min(255) as u8);
        for asset in &offer.assets {
            payload.push(asset.kind);
            payload.extend_from_slice(&asset.size.to_le_bytes());
        }
        payload.push(u8::from(offer.shell.is_some()));
        if let Some(shell) = offer.shell {
            payload.extend_from_slice(&shell.colour);
            payload.push(outline_code(shell.outline));
            payload.push(finish_code(shell.finish));
        }
        // Appended after the shell so a build that predates the content choice
        // simply ignores the tail and treats the offer as a plain cartridge.
        payload.push(offer.mode.code());
        payload.push(core_code(offer.core));
        let wire = self.wire.as_mut().ok_or("对方还没有就绪")?;
        wire.send(0, &message(KIND_OFFER, &payload));
        Ok(())
    }

    fn handle_frame(&mut self, root: &Path, frame: &[u8]) {
        let Some((kind, payload)) = parse(frame) else {
            return;
        };
        match kind {
            KIND_OFFER => {
                if self.stage == Stage::Shelf {
                    if let Some(offer) = parse_offer(payload) {
                        self.peer = Some(offer.name.clone());
                        self.incoming = Some(offer);
                        self.stage = Stage::Incoming;
                    }
                }
            }
            KIND_ACCEPT => {
                if self.stage == Stage::OfferSent {
                    let mask = payload.first().copied().unwrap_or(0);
                    if let Some(selected) = self.selected.as_mut() {
                        if mask != 0 {
                            selected.assets.retain(|asset| {
                                asset.meta.kind != ASSET_ROM
                                    && (mask & (1u8 << asset.meta.kind)) != 0
                            });
                            selected.offer.assets =
                                selected.assets.iter().map(|asset| asset.meta).collect();
                            selected.offer.size =
                                selected.assets.iter().map(|asset| asset.meta.size).sum();
                            if selected.assets.is_empty() {
                                self.error = Some("对方请求的内容为空".into());
                                self.stage = Stage::Failed;
                                return;
                            }
                            self.parts_only = true;
                        }
                    }
                    self.sending_index = 0;
                    self.sending_at = 0;
                    self.progress = 0.0;
                    self.stage = Stage::Sending;
                }
            }
            KIND_REJECT => {
                if self.stage == Stage::OfferSent {
                    self.stage = Stage::Shelf;
                    self.selected = None;
                    self.progress = 0.0;
                }
            }
            KIND_DATA => {
                if self.stage == Stage::Receiving {
                    self.receive_chunk(payload);
                }
            }
            KIND_DONE => {
                if self.stage == Stage::Receiving {
                    if self.finish_receive(root) {
                        self.send_message(KIND_ACK);
                        self.stage = Stage::Done;
                        self.done_processed = true;
                    }
                } else if self.stage == Stage::Sending {
                    self.stage = Stage::Done;
                    self.done_processed = true;
                }
            }
            KIND_ACK => {
                if self.stage == Stage::Done {
                    self.done_processed = true;
                }
            }
            KIND_DUPLICATE => {
                self.error = Some("对方已拥有当前卡带".into());
                self.duplicate_receiver = false;
                self.stage = Stage::Duplicate;
                self.selected = None;
            }
            KIND_CANCEL => {
                // Leaving the room after a finished transfer is not a failure:
                // the ROM is already committed, so keep the Done/Shelf result.
                if matches!(self.stage, Stage::Done | Stage::Shelf) {
                    return;
                }
                self.error = Some("对方取消了传输".into());
                self.stage = Stage::Failed;
                self.abort_receive();
            }
            _ => {}
        }
    }

    fn receive_chunk(&mut self, payload: &[u8]) {
        if payload.len() < 9 {
            return;
        }
        let kind = payload[0];
        let offset = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let Some(asset) = self.receiving.iter_mut().find(|asset| asset.meta.kind == kind) else {
            return;
        };
        if offset != asset.received {
            self.error = Some("传输数据乱序".into());
            self.stage = Stage::Failed;
            self.abort_receive();
            return;
        }
        if asset
            .file
            .write_all(&payload[9..])
            .is_err()
        {
            self.error = Some("写入卡带文件失败".into());
            self.stage = Stage::Failed;
            self.abort_receive();
            return;
        }
        asset.received += (payload.len() - 9) as u64;
        self.received_total += (payload.len() - 9) as u64;
        self.progress = (self.received_total as f32 / self.receiving_total.max(1) as f32)
            .clamp(0.0, 1.0);
    }

    fn finish_receive(&mut self, root: &Path) -> bool {
        if self.receiving.is_empty() {
            return false;
        }
        if self
            .receiving
            .iter()
            .any(|asset| asset.received != asset.meta.size)
        {
            self.error = Some("传输数据不完整".into());
            self.stage = Stage::Failed;
            self.abort_receive();
            return false;
        }
        for asset in &mut self.receiving {
            if asset.file.flush().is_err() {
                self.error = Some("保存卡带文件失败".into());
                self.stage = Stage::Failed;
                self.abort_receive();
                return false;
            }
        }
        for asset in &self.receiving {
            // Save data replaces whatever is already there, so the old file is
            // copied aside first — the player gets one undo, and a failed
            // rename leaves the original untouched.
            if matches!(asset.meta.kind, ASSET_SAV | ASSET_STATE | ASSET_THUMB) {
                if let Some(backup) = backup_target(&asset.target) {
                    self.backups.push(backup);
                }
            }
            if std::fs::rename(&asset.temp, &asset.target).is_err() {
                self.error = Some("保存卡带文件失败".into());
                self.stage = Stage::Failed;
                return false;
            }
        }
        if let (Some(stem), Some(offer)) = (&self.target_stem, &self.incoming) {
            if !self.parts_only {
                if let Some(shell) = offer.shell {
                    let _ = slot_store::ini::write(
                    root,
                    "linkSlot/Config/cart_shell.txt",
                        stem,
                        &shell.to_value(),
                    );
                }
            }
            let rom = self
                .receiving
                .iter()
                .find(|asset| asset.meta.kind == ASSET_ROM)
                .map(|asset| asset.target.clone());
            let label = self
                .receiving
                .iter()
                .find(|asset| asset.meta.kind == ASSET_LABEL)
                .map(|asset| asset.target.clone());
            if let Some(rom) = rom {
                let cart = Cart {
                    platform: offer.platform,
                    stem: stem.clone(),
                    rom,
                    label,
                    title: String::new(),
                    code: String::new(),
                    shell: offer.shell,
                model: Default::default(),
                front: None,
};
                let _ = slot_store::cart_meta::update(root, &cart, |meta| {
                    meta.sha256 = Some(rom_id::digest_hex(&offer.hash));
                });
            }
        }
        self.receiving.clear();
        self.progress = 1.0;
        true
    }

    fn abort_receive(&mut self) {
        for asset in &mut self.receiving {
            let _ = std::fs::remove_file(&asset.temp);
        }
        self.receiving.clear();
    }

    fn pump_send(&mut self, wire: &mut Box<dyn LinkChannel>) {
        let Some(selected) = &mut self.selected else {
            return;
        };
        if self.sending_index >= selected.assets.len() {
            wire.send(0, &message(KIND_DONE, &[]));
            self.stage = Stage::Done;
            self.progress = 1.0;
            self.done_processed = true;
            return;
        }
        let asset = &selected.assets[self.sending_index];
        let mut file = match File::open(&asset.path) {
            Ok(file) => file,
            Err(error) => {
                self.error = Some(format!("读取卡带文件失败：{error}"));
                self.stage = Stage::Failed;
                return;
            }
        };
        if self.sending_at >= asset.meta.size {
            self.sending_index += 1;
            self.sending_at = 0;
            return;
        }
        if file.seek(std::io::SeekFrom::Start(self.sending_at)).is_err() {
            self.error = Some("读取卡带文件失败".into());
            self.stage = Stage::Failed;
            return;
        }
        let remaining = (asset.meta.size - self.sending_at).min(CHUNK as u64) as usize;
        let mut buf = vec![0u8; remaining];
        match file.read_exact(&mut buf) {
            Ok(()) => {
                let mut payload = Vec::with_capacity(9 + buf.len());
                payload.push(asset.meta.kind);
                payload.extend_from_slice(&self.sending_at.to_le_bytes());
                payload.extend_from_slice(&buf);
                wire.send(0, &message(KIND_DATA, &payload));
                self.sending_at += buf.len() as u64;
                self.progress = if selected.offer.size == 0 {
                    1.0
                } else {
                    let done: u64 = selected.assets[..self.sending_index]
                        .iter()
                        .map(|asset| asset.meta.size)
                        .sum::<u64>()
                        + self.sending_at;
                    (done as f32 / selected.offer.size as f32).clamp(0.0, 1.0)
                };
                if self.sending_at >= asset.meta.size {
                    self.sending_index += 1;
                    self.sending_at = 0;
                }
            }
            Err(error) => {
                self.error = Some(format!("读取卡带文件失败：{error}"));
                self.stage = Stage::Failed;
            }
        }
    }
}

pub fn asset_label(kind: u8) -> &'static str {
    match kind {
        ASSET_ROM => "ROM",
        ASSET_LABEL => "封面",
        ASSET_INTRO => "介绍",
        ASSET_VIDEO => "视频",
        ASSET_SAV => "存档",
        ASSET_STATE => "状态",
        ASSET_THUMB => "缩略图",
        _ => "资料",
    }
}

fn message(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + payload.len());
    out.extend_from_slice(MAGIC);
    out.push(kind);
    out.extend_from_slice(payload);
    out
}

fn parse(frame: &[u8]) -> Option<(u8, &[u8])> {
    (frame.len() >= 5 && &frame[..4] == MAGIC).then(|| (frame[4], &frame[5..]))
}

fn parse_offer(payload: &[u8]) -> Option<Offer> {
    let first = *payload.first()?;
    let platform = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Fc,
        Platform::Sfc,
    ]
    .into_iter()
    .find(|p| p.index() as u8 == first)?;
    let mut at = 1;
    let ext_len = *payload.get(at)? as usize;
    at += 1;
    let extension = std::str::from_utf8(payload.get(at..at + ext_len)?)
        .ok()?
        .to_string();
    at += ext_len;
    let name_len = u16::from_le_bytes(payload.get(at..at + 2)?.try_into().ok()?) as usize;
    at += 2;
    let name = String::from_utf8_lossy(payload.get(at..at + name_len)?).into_owned();
    at += name_len;
    let size = u64::from_le_bytes(payload.get(at..at + 8)?.try_into().ok()?);
    at += 8;
    let hash: Digest = payload.get(at..at + 32)?.try_into().ok()?;
    at += 32;
    let count = *payload.get(at)? as usize;
    at += 1;
    let mut assets = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = *payload.get(at)?;
        at += 1;
        let size = u64::from_le_bytes(payload.get(at..at + 8)?.try_into().ok()?);
        at += 8;
        assets.push(AssetMeta { kind, size });
    }
    let has_shell = *payload.get(at)? != 0;
    at += 1;
    let shell = if has_shell {
        let colour = payload.get(at..at + 3)?.try_into().ok()?;
        at += 3;
        let outline = outline_from(*payload.get(at)?);
        at += 1;
        let finish = finish_from(*payload.get(at)?);
        Some(ShellChoice {
            outline,
            colour,
            finish,
        })
    } else {
        None
    };
    // Older builds end the offer right after the shell: a missing tail means
    // "cart only", which is exactly what they sent.
    let mode = payload
        .get(at)
        .copied()
        .map(TransferMode::from_code)
        .unwrap_or_default();
    at += 1;
    let core = payload
        .get(at)
        .copied()
        .map(core_from)
        .unwrap_or_default();
    Some(Offer {
        platform,
        extension,
        name,
        size,
        hash,
        assets,
        shell,
        mode,
        core,
    })
}

/// Stable wire id for [`Core`].  Only needs to agree across linkSlot builds,
/// so it follows `Core::EVERY` rather than any on-disk string.
fn core_code(core: Core) -> u8 {
    Core::EVERY
        .iter()
        .position(|known| *known == core)
        .unwrap_or(0) as u8
}

fn core_from(code: u8) -> Core {
    Core::EVERY.get(code as usize).copied().unwrap_or_default()
}

/// `States/<platform>/<core>/<stem>/`, the directory `resume.state` lives in.
fn state_dir(root: &Path, platform: Platform, core: Core, stem: &str) -> PathBuf {
    root.join("States")
        .join(platform.dir_name())
        .join(core.as_str())
        .join(stem)
}

/// Copy a file that is about to be replaced.  The stamp keeps a second
/// overwrite from clobbering the first backup.
fn backup_target(target: &Path) -> Option<PathBuf> {
    if !target.is_file() {
        return None;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())?;
    let parent = target.parent()?;
    let name = target.file_name()?.to_str()?;
    let backup = match name.rsplit_once('.') {
        Some((stem, ext)) => parent.join(format!("{stem}.conflict.{stamp}.{ext}")),
        None => parent.join(format!("{name}.conflict.{stamp}")),
    };
    std::fs::copy(target, &backup).ok()?;
    Some(backup)
}

fn outline_code(outline: Outline) -> u8 {
    match outline {
        Outline::Auto => 0,
        Outline::Notched => 1,
        Outline::Rounded => 2,
    }
}

fn outline_from(code: u8) -> Outline {
    match code {
        1 => Outline::Notched,
        2 => Outline::Rounded,
        _ => Outline::Auto,
    }
}

fn finish_code(finish: ShellFinish) -> u8 {
    match finish {
        ShellFinish::Solid => 0,
        ShellFinish::Clear => 1,
        ShellFinish::Glitter => 2,
    }
}

fn finish_from(code: u8) -> ShellFinish {
    match code {
        1 => ShellFinish::Clear,
        2 => ShellFinish::Glitter,
        _ => ShellFinish::Solid,
    }
}

fn source_assets(
    root: &Path,
    cart: &Cart,
    mode: TransferMode,
) -> Result<Vec<SourceAsset>, String> {
    let mut assets = Vec::new();
    if mode.sends_cart() {
        let size = std::fs::metadata(&cart.rom)
            .map_err(|e| format!("读取卡带失败：{e}"))?
            .len();
        assets.push(SourceAsset {
            meta: AssetMeta {
                kind: ASSET_ROM,
                size,
            },
            path: cart.rom.clone(),
        });
        if let Some(label) = &cart.label {
            if let Ok(meta) = std::fs::metadata(label) {
                assets.push(SourceAsset {
                    meta: AssetMeta {
                        kind: ASSET_LABEL,
                        size: meta.len(),
                    },
                    path: label.clone(),
                });
            }
        }
        let info = root.join("Info").join(cart.platform.dir_name());
        for (kind, extension) in [(ASSET_INTRO, "txt"), (ASSET_VIDEO, "mp4")] {
            let path = info.join(format!("{}.{}", cart.stem, extension));
            if let Ok(meta) = std::fs::metadata(&path) {
                assets.push(SourceAsset {
                    meta: AssetMeta {
                        kind,
                        size: meta.len(),
                    },
                    path,
                });
            }
        }
    }
    if mode.sends_save() {
        let saves = crate::root::saves_dir(root).join(cart.platform.dir_name());
        // `.srm` is the legacy name older cores wrote; send whichever exists.
        let sav = [format!("{}.sav", cart.stem), format!("{}.srm", cart.stem)]
            .into_iter()
            .map(|name| saves.join(name))
            .find(|path| path.is_file());
        if let Some(path) = sav {
            if let Ok(meta) = std::fs::metadata(&path) {
                assets.push(SourceAsset {
                    meta: AssetMeta {
                        kind: ASSET_SAV,
                        size: meta.len(),
                    },
                    path,
                });
            }
        }
        let core = slot_store::core_for_platform(root, &cart.stem, cart.platform);
        let dir = state_dir(root, cart.platform, core, &cart.stem);
        for (kind, name) in [(ASSET_STATE, "resume.state"), (ASSET_THUMB, "resume.png")] {
            let path = dir.join(name);
            if let Ok(meta) = std::fs::metadata(&path) {
                assets.push(SourceAsset {
                    meta: AssetMeta {
                        kind,
                        size: meta.len(),
                    },
                    path,
                });
            }
        }
    }
    Ok(assets)
}

fn unique_stem(root: &Path, platform: Platform, stem: &str) -> String {
    let dir = root.join("Games").join(platform.dir_name());
    for index in 1..=9999u32 {
        let candidate = if index == 1 {
            stem.to_string()
        } else {
            format!("{stem} ({index})")
        };
        // A stem is the save/state/cover key, so it must be unique across
        // every extension the platform accepts: Contra.smc and Contra.sfc
        // would otherwise share Saves/SFC/Contra.sav and all state paths.
        let occupied = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .any(|name| {
                let path = Path::new(&name);
                path.file_stem()
                    .is_some_and(|other| other.eq_ignore_ascii_case(candidate.as_str()))
                    && path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| {
                            platform
                                .extensions()
                                .iter()
                                .any(|known| extension.eq_ignore_ascii_case(known))
                        })
            });
        if !occupied {
            return candidate;
        }
    }
    format!("{stem} (copy)")
}

fn asset_paths(
    root: &Path,
    platform: Platform,
    stem: &str,
    extension: &str,
    kind: u8,
    core: Core,
) -> Result<(PathBuf, PathBuf), String> {
    let (dir, name) = match kind {
        ASSET_ROM => (
            root.join("Games").join(platform.dir_name()),
            format!("{stem}.{extension}"),
        ),
        ASSET_LABEL => (
            root.join("Labels").join(platform.dir_name()),
            format!("{stem}.png"),
        ),
        ASSET_INTRO => (
            root.join("Info").join(platform.dir_name()),
            format!("{stem}.txt"),
        ),
        ASSET_VIDEO => (
            root.join("Info").join(platform.dir_name()),
            format!("{stem}.mp4"),
        ),
        ASSET_SAV => (
            crate::root::saves_dir(root).join(platform.dir_name()),
            format!("{stem}.sav"),
        ),
        ASSET_STATE => (state_dir(root, platform, core, stem), "resume.state".to_string()),
        ASSET_THUMB => (state_dir(root, platform, core, stem), "resume.png".to_string()),
        _ => return Err("对方发来了未知的卡带内容".into()),
    };
    let target = dir.join(name);
    let temp = dir.join(format!(".{}.part", target.file_name().unwrap_or_default().to_string_lossy()));
    let _ = std::fs::remove_file(&temp);
    Ok((temp, target))
}

#[cfg(test)]
impl Transfer {
    /// A transport-free transfer for UI state tests (side + stage only).
    pub(crate) fn stub(host: bool, stage: Stage) -> Transfer {
        Transfer {
            host,
            stage,
            mode: TransferMode::CartOnly,
            starter: None,
            wire: None,
            selected: None,
            incoming: None,
            progress: 0.0,
            peer: None,
            received: false,
            duplicate_receiver: false,
            parts: Vec::new(),
            parts_only: false,
            parts_stem: None,
            backups: Vec::new(),
            error: None,
            sending_index: 0,
            sending_at: 0,
            receiving: Vec::new(),
            receiving_total: 0,
            received_total: 0,
            target_stem: None,
            done_processed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_net::TcpLink;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    fn attached(wire: TcpLink) -> Transfer {
        Transfer {
            host: true,
            stage: Stage::Shelf,
            mode: TransferMode::CartOnly,
            starter: None,
            wire: Some(Box::new(wire)),
            selected: None,
            incoming: None,
            progress: 0.0,
            peer: None,
            received: false,
            duplicate_receiver: false,
            parts: Vec::new(),
            parts_only: false,
            parts_stem: None,
            backups: Vec::new(),
            error: None,
            sending_index: 0,
            sending_at: 0,
            receiving: Vec::new(),
            receiving_total: 0,
            received_total: 0,
            target_stem: None,
            done_processed: false,
        }
    }

    fn pair() -> (Transfer, Transfer) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let host = std::thread::spawn(move || TcpLink::host("127.0.0.1", port));
        std::thread::sleep(Duration::from_millis(80));
        let client = TcpLink::join("127.0.0.1", port).unwrap();
        let host = host.join().unwrap().unwrap();
        (attached(host), attached(client))
    }

    fn source_cart(root: &Path) -> Cart {
        let dir = root.join("Games/GBA");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Source.gba"), vec![0x41; 150_000]).unwrap();
        std::fs::create_dir_all(root.join("Labels/GBA")).unwrap();
        std::fs::create_dir_all(root.join("Info/GBA")).unwrap();
        std::fs::write(root.join("Labels/GBA/Source.png"), b"cover").unwrap();
        std::fs::write(root.join("Info/GBA/Source.txt"), b"intro").unwrap();
        Cart {
            platform: Platform::Gba,
            stem: "Source".into(),
            rom: dir.join("Source.gba"),
            label: Some(root.join("Labels/GBA/Source.png")),
            title: String::new(),
            code: String::new(),
            shell: Some(ShellChoice {
                outline: Outline::Auto,
                colour: [0x12, 0x34, 0x56],
                finish: ShellFinish::Solid,
            }),
        model: Default::default(),
        front: None,
}
    }

    #[test]
    fn an_automatic_shell_colour_is_carried_by_the_offer() {
        let root = tempfile::tempdir().unwrap();
        let mut cart = source_cart(root.path());
        cart.shell = None;
        cart.code = "AXV".into();
        let (mut sender, _receiver) = pair();
        sender.select(root.path(), &cart, [0u8; 32]).unwrap();
        let shell = sender
            .selected
            .as_ref()
            .expect("selected offer")
            .offer
            .shell
            .expect("effective shell");
        assert_eq!(shell.colour, [0xc2, 0x33, 0x2e]);
    }

    #[test]
    fn a_cartridge_moves_its_rom_cover_and_intro_but_not_its_save() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        std::fs::create_dir_all(sender_root.path().join("Saves/GBA")).unwrap();
        std::fs::write(sender_root.path().join("Saves/GBA/Source.sav"), b"save").unwrap();
        let (mut sender, mut receiver) = pair();

        sender.select(sender_root.path(), &cart, rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32])).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(receiver.stage, Stage::Incoming);
        let incoming = receiver.incoming.as_ref().expect("incoming offer");
        assert_eq!(incoming.name, "Source");
        assert_eq!(
            incoming.shell.map(|shell| shell.colour),
            Some([0x12, 0x34, 0x56])
        );
        receiver.accept(receiver_root.path()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while (sender.stage != Stage::Done || receiver.stage != Stage::Done)
            && Instant::now() < deadline
        {
            sender.update(sender_root.path());
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(sender.stage, Stage::Done, "{:?}", sender.error);
        assert_eq!(receiver.stage, Stage::Done, "{:?}", receiver.error);
        assert_eq!(
            std::fs::read(receiver_root.path().join("Games/GBA/Source.gba")).unwrap(),
            vec![0x41; 150_000]
        );
        assert!(receiver_root.path().join("Labels/GBA/Source.png").is_file());
        assert!(receiver_root.path().join("Info/GBA/Source.txt").is_file());
        assert!(!receiver_root.path().join("Saves/GBA/Source.sav").exists());
        assert!(receiver_root.path().join("linkSlot/Config/cart_shell.txt").is_file());
    }

    #[test]
    fn an_offer_can_be_rejected_without_writing_a_file() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        let (mut sender, mut receiver) = pair();
        sender.select(sender_root.path(), &cart, rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32])).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        receiver.reject();
        let deadline = Instant::now() + Duration::from_secs(5);
        while sender.stage != Stage::Shelf && Instant::now() < deadline {
            sender.update(sender_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!receiver_root.path().join("Games/GBA/Source.gba").exists());
    }

    #[test]
    fn a_duplicate_rom_can_receive_only_its_missing_side_assets() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        std::fs::write(
            sender_root.path().join("Info/GBA/Source.mp4"),
            b"video",
        )
        .unwrap();

        let receiver_labels = receiver_root.path().join("Labels/GBA");
        let receiver_info = receiver_root.path().join("Info/GBA");
        std::fs::create_dir_all(&receiver_labels).unwrap();
        std::fs::create_dir_all(&receiver_info).unwrap();
        std::fs::create_dir_all(receiver_root.path().join("Games/GBA")).unwrap();
        std::fs::write(
            receiver_root.path().join("Games/GBA/Source.gba"),
            vec![0x41; 150_000],
        )
        .unwrap();
        std::fs::write(receiver_info.join("Source.txt"), b"local intro").unwrap();
        let receiver_cart = Cart {
            platform: Platform::Gba,
            stem: "Source".into(),
            rom: receiver_root.path().join("Games/GBA/Source.gba"),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: Some(ShellChoice {
                outline: Outline::Auto,
                colour: [1, 2, 3],
                finish: ShellFinish::Solid,
            }),
        model: Default::default(),
        front: None,
};

        let (mut sender, mut receiver) = pair();
        sender
            .select(sender_root.path(), &cart, rom_id::digest_file(&cart.rom).unwrap())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(receiver.stage, Stage::Incoming);
        let offer = receiver.incoming.as_ref().unwrap();
        let missing = offer.parts_missing(receiver_root.path(), &receiver_cart);
        assert_eq!(missing, vec![ASSET_LABEL, ASSET_VIDEO]);
        receiver.offer_parts("Source".into(), missing);
        assert_eq!(receiver.stage, Stage::IncomingPartial);
        assert_eq!(receiver.display_assets(), vec!["封面", "视频"]);
        receiver.accept_parts(receiver_root.path()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        while (sender.stage != Stage::Done || receiver.stage != Stage::Done)
            && Instant::now() < deadline
        {
            sender.update(sender_root.path());
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(sender.stage, Stage::Done, "{:?}", sender.error);
        assert_eq!(receiver.stage, Stage::Done, "{:?}", receiver.error);
        assert_eq!(
            std::fs::read(receiver_info.join("Source.txt")).unwrap(),
            b"local intro"
        );
        assert_eq!(
            std::fs::read(receiver_labels.join("Source.png")).unwrap(),
            b"cover"
        );
        assert_eq!(
            std::fs::read(receiver_info.join("Source.mp4")).unwrap(),
            b"video"
        );
        assert!(!receiver_root
            .path()
            .join("Games/GBA/Source (2).gba")
            .exists());
        assert!(!receiver_root
            .path()
            .join("linkSlot/Config/cart_shell.txt")
            .exists());
    }

    #[test]
    fn a_stem_is_unique_across_every_rom_extension() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("Games/SFC");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Contra.smc"), b"rom").unwrap();

        assert_eq!(
            unique_stem(root.path(), Platform::Sfc, "Contra"),
            "Contra (2)",
            "an alternate extension did not reserve the stem"
        );
        assert_eq!(
            unique_stem(root.path(), Platform::Sfc, "Other"),
            "Other"
        );
    }

    #[test]
    fn a_cancel_after_done_keeps_the_transfer_result() {
        let (mut sender, mut receiver) = pair();
        sender.stage = Stage::Done;
        receiver.stage = Stage::Done;

        sender.cancel();
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline {
            receiver.update(Path::new("."));
            std::thread::sleep(Duration::from_millis(1));
            if receiver.stage != Stage::Done {
                break;
            }
        }
        assert_eq!(receiver.stage, Stage::Done, "a late cancel failed a finished transfer");
        assert!(receiver.error.is_none(), "{:?}", receiver.error);
    }

    #[test]
    fn a_rejected_offer_returns_the_sender_to_the_shelf() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        let hash = rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32]);
        let (mut sender, mut receiver) = pair();
        sender.select(sender_root.path(), &cart, hash).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(receiver.stage, Stage::Incoming);
        receiver.incoming.as_mut().unwrap().size = 65 * 1024 * 1024;
        assert!(receiver.accept(receiver_root.path()).is_err());
        assert_eq!(sender.stage, Stage::OfferSent);

        let deadline = Instant::now() + Duration::from_secs(5);
        while sender.stage == Stage::OfferSent && Instant::now() < deadline {
            sender.update(sender_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(sender.stage, Stage::Shelf, "{:?}", sender.error);
    }

    #[test]
    fn a_full_accept_clears_a_previous_parts_only_transfer() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        let hash = rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32]);
        let (mut sender, mut receiver) = pair();
        sender.select(sender_root.path(), &cart, hash).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(receiver.stage, Stage::Incoming);
        receiver.parts_only = true;
        receiver.accept(receiver_root.path()).unwrap();
        assert!(!receiver.parts_only);
        receiver.cancel();
        sender.cancel();
    }

    #[test]
    fn a_duplicate_frame_is_read_even_when_the_receiver_closes() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        let (mut sender, mut receiver) = pair();
        sender
            .select(sender_root.path(), &cart, rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32]))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        receiver.mark_duplicate(true);
        if let Some(wire) = receiver.wire.as_mut() {
            wire.send_end();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while sender.stage != Stage::Duplicate && Instant::now() < deadline {
            sender.update(sender_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(sender.stage, Stage::Duplicate, "{:?}", sender.error);
    }

    #[test]
    fn a_receiver_that_already_has_the_hash_refuses_without_writing() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        // Same bytes, same platform and therefore the same ROM hash.
        source_cart(receiver_root.path());
        let (mut sender, mut receiver) = pair();
        sender.select(sender_root.path(), &cart, rom_id::digest_file(&cart.rom).unwrap_or([0u8; 32])).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(receiver_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        receiver.mark_duplicate(true);
        assert_eq!(receiver.stage, Stage::Duplicate);
        assert!(receiver.duplicate_receiver);
        let deadline = Instant::now() + Duration::from_secs(5);
        while sender.stage != Stage::Duplicate && Instant::now() < deadline {
            sender.update(sender_root.path());
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(sender.stage, Stage::Duplicate);
        assert!(!sender.duplicate_receiver);
        assert!(!receiver_root.path().join("Games/GBA/Source (2).gba").exists());
        assert_eq!(
            std::fs::read(receiver_root.path().join("Games/GBA/Source.gba")).unwrap(),
            vec![0x41; 150_000]
        );
    }

    /// Drive both sides until the transfer settles.
    fn settle(sender: &mut Transfer, receiver: &mut Transfer, s: &Path, r: &Path) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while (sender.stage != Stage::Done || receiver.stage != Stage::Done)
            && Instant::now() < deadline
        {
            sender.update(s);
            receiver.update(r);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn wait_incoming(receiver: &mut Transfer, r: &Path) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stage != Stage::Incoming && Instant::now() < deadline {
            receiver.update(r);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(receiver.stage, Stage::Incoming, "{:?}", receiver.error);
    }

    fn write_save(root: &Path, city: &[u8]) {
        let dir = root.join("Saves/GBA");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Source.sav"), city).unwrap();
    }

    fn write_state(root: &Path) {
        let dir = root.join("States/GBA/mgba/Source");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("resume.state"), b"resume").unwrap();
        std::fs::write(dir.join("resume.png"), b"png").unwrap();
    }

    #[test]
    fn a_save_only_transfer_moves_the_save_and_the_state_but_no_cart() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        write_save(sender_root.path(), b"battery");
        write_state(sender_root.path());

        let (mut sender, mut receiver) = pair();
        sender.mode = TransferMode::SaveOnly;
        sender
            .select(
                sender_root.path(),
                &cart,
                rom_id::digest_file(&cart.rom).unwrap(),
            )
            .unwrap();
        wait_incoming(&mut receiver, receiver_root.path());
        let offer = receiver.incoming.as_ref().expect("no offer");
        assert_eq!(offer.mode, TransferMode::SaveOnly);
        assert_eq!(offer.core, Core::Mgba);
        assert!(
            !offer.assets.iter().any(|asset| asset.kind == ASSET_ROM),
            "a save-only offer carried the ROM"
        );
        receiver.accept(receiver_root.path()).unwrap();
        settle(
            &mut sender,
            &mut receiver,
            sender_root.path(),
            receiver_root.path(),
        );

        assert_eq!(sender.stage, Stage::Done, "{:?}", sender.error);
        assert_eq!(receiver.stage, Stage::Done, "{:?}", receiver.error);
        assert_eq!(
            std::fs::read(receiver_root.path().join("Saves/GBA/Source.sav")).unwrap(),
            b"battery"
        );
        assert_eq!(
            std::fs::read(
                receiver_root
                    .path()
                    .join("States/GBA/mgba/Source/resume.state")
            )
            .unwrap(),
            b"resume"
        );
        assert_eq!(
            std::fs::read(receiver_root.path().join("States/GBA/mgba/Source/resume.png"))
                .unwrap(),
            b"png"
        );
        assert!(
            !receiver_root.path().join("Games/GBA/Source.gba").exists(),
            "a save-only transfer copied the ROM"
        );
    }

    #[test]
    fn a_replaced_save_is_backed_up_next_to_it() {
        let sender_root = tempfile::tempdir().unwrap();
        let receiver_root = tempfile::tempdir().unwrap();
        let cart = source_cart(sender_root.path());
        write_save(sender_root.path(), b"new");
        write_save(receiver_root.path(), b"old");

        let (mut sender, mut receiver) = pair();
        sender.mode = TransferMode::SaveOnly;
        sender
            .select(
                sender_root.path(),
                &cart,
                rom_id::digest_file(&cart.rom).unwrap(),
            )
            .unwrap();
        wait_incoming(&mut receiver, receiver_root.path());
        receiver.accept(receiver_root.path()).unwrap();
        settle(
            &mut sender,
            &mut receiver,
            sender_root.path(),
            receiver_root.path(),
        );
        assert_eq!(receiver.stage, Stage::Done, "{:?}", receiver.error);

        let saves = receiver_root.path().join("Saves/GBA");
        assert_eq!(std::fs::read(saves.join("Source.sav")).unwrap(), b"new");
        let backup = std::fs::read_dir(&saves)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".conflict."))
            })
            .expect("no conflict backup of the replaced save");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        assert_eq!(receiver.backups.len(), 1);
        assert_eq!(receiver.backups[0], backup);
    }

    #[test]
    fn parts_cover_every_save_kind_the_offer_carries() {
        let root = tempfile::tempdir().unwrap();
        let cart = source_cart(root.path());
        let offer = Offer {
            platform: Platform::Gba,
            extension: "gba".into(),
            name: "Source".into(),
            size: 13,
            hash: [7u8; 32],
            assets: vec![
                AssetMeta {
                    kind: ASSET_ROM,
                    size: 150_000,
                },
                AssetMeta {
                    kind: ASSET_LABEL,
                    size: 5,
                },
                AssetMeta {
                    kind: ASSET_SAV,
                    size: 3,
                },
                AssetMeta {
                    kind: ASSET_STATE,
                    size: 5,
                },
            ],
            shell: None,
            mode: TransferMode::All,
            core: Core::Mgba,
        };
        // The offer's cover is already on this card; the saves always come.
        assert_eq!(offer.parts_missing(root.path(), &cart), vec![ASSET_SAV, ASSET_STATE]);
    }

    #[test]
    fn an_offer_without_the_content_tail_reads_as_cart_only() {
        let mut payload = Vec::new();
        payload.push(Platform::Gba.index() as u8);
        payload.push(3);
        payload.extend_from_slice(b"gba");
        payload.extend_from_slice(&(6u16).to_le_bytes());
        payload.extend_from_slice(b"Source");
        payload.extend_from_slice(&150_000u64.to_le_bytes());
        payload.extend_from_slice(&[7u8; 32]);
        payload.push(1);
        payload.push(ASSET_ROM);
        payload.extend_from_slice(&150_000u64.to_le_bytes());
        payload.push(0);
        // No mode / core bytes: exactly what a build before this change sent.
        let offer = parse_offer(&payload).expect("old offer did not parse");
        assert_eq!(offer.mode, TransferMode::CartOnly);
        assert_eq!(offer.core, Core::Mgba);
    }

    #[test]
    fn a_save_only_cartridge_without_a_save_is_refused_locally() {
        let root = tempfile::tempdir().unwrap();
        let cart = source_cart(root.path());
        let (mut sender, _receiver) = pair();
        sender.mode = TransferMode::SaveOnly;
        let error = sender
            .select(root.path(), &cart, [0u8; 32])
            .expect_err("an empty save-only transfer was accepted");
        assert!(error.contains("存档"), "{error}");
    }
}
