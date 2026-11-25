//! Phase 2（`kind = "platform"` 平台包）的**独立**验收测试。
//!
//! 本文件只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod packs_qa;` 挂进来。
//! `packs::install/uninstall` 是 `pub(crate)`，外部 integration test 调不到，
//! 所以用例只能住在 crate 内。
//!
//! 这里的夹具刻意**不复用** `packs.rs` 自带测试的辅助函数：那套断言和实现出自
//! 同一人之手，同一个误读会同时出现在两边而互相印证。本文件的包全部用下面这个
//! 自己写的"存储式 ZIP"构造器现场拼；唯一例外是
//! [`the_python_builder_output_installs_activates_and_scans`]——它用的是仓库里
//! 真实的 `build/pack_lsp.py`，为的是钉住"脚本打出来的包 app 真的能装"。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};
use sha2::Digest;

use crate::packs::{install, installed, parse_manifest, uninstall};
use slot_store::{
    available_options, platform_packs, scan_quiet, Core, Platform,
};

const PACK: &str = "arcade-neogeo";
const SO: &str = "System/linkSlot/cores/fbneo/fbneo_libretro.so";
const PLATFORM_ROOT: &str = "System/linkSlot/extensions/platforms";
const RECORD_DIR: &str = "linkSlot/Config/packs";
const RECORD_FILE: &str = "installed.json";
const MIB: usize = 1024 * 1024;

// ---------------------------------------------------------------- 小工具

fn sha256(data: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(data))
}

fn dat_path(pack: &str, platform: &str) -> String {
    format!("{PLATFORM_ROOT}/{pack}/metadata/{platform}.dat.json")
}

fn write_file(root: &Path, relative: &str, data: &[u8]) {
    let target = root.join(relative);
    std::fs::create_dir_all(target.parent().expect("文件必须有父目录")).unwrap();
    std::fs::write(&target, data).unwrap();
}

fn record_path(root: &Path) -> PathBuf {
    root.join(RECORD_DIR).join(RECORD_FILE)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/slot 之上是仓库根")
        .to_path_buf()
}

/// 只写"存储"条目（method 0）的最小 ZIP 构造器。
///
/// 每个字节都由本文件算，不经过 `zip` crate；这样"包能不能被读进来"就不会被
/// 同一个库的同一处误解同时盖住。
fn zip_stored(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32fast::hash(data);
        let name_len = name.len() as u16;

        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 名字
        out.extend_from_slice(&0u16.to_le_bytes()); // method = store
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // method
        central.extend_from_slice(&0u16.to_le_bytes()); // time
        central.extend_from_slice(&0x21u16.to_le_bytes()); // date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u32;
    out.extend_from_slice(&central);
    let count = entries.len() as u16;
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

// ---------------------------------------------------------------- 夹具

fn platform_decl(id: &str) -> Value {
    let (folder, profile, bios) = match id {
        "neogeo" => ("Games/NeoGeo", "neogeo", json!(["neogeo.zip"])),
        _ => ("Games/Arcade", "arcade", json!([])),
    };
    json!({
        "id": id,
        "name": id,
        "folder": folder,
        "rom_extensions": ["zip"],
        "scanner": "arcade_zip_v1",
        "ui": "arcade_cabinet_v1",
        "ui_profile": profile,
        "input": "arcade_6button_v1",
        "core_ids": ["fbneo"],
        "default_core": "fbneo",
        "bios": bios,
    })
}

fn core_decl() -> Value {
    json!({
        "id": "fbneo",
        "dylib": "fbneo_libretro.so",
        "display": "FBNeo",
        "license": "Non-commercial",
        "source": "https://example.invalid/fbneo",
        "system_dir": "BIOS/fbneo",
    })
}

/// 几百字节的 DAT，形状与 Phase 3 生成器一致（set → title…）。
fn dat_json(platform: &str) -> Vec<u8> {
    let sets: Vec<Value> = (0..12)
        .map(|index| {
            json!({
                "set": format!("{platform}-set-{index:02}"),
                "title": format!("QA {platform} title {index}"),
                "system": platform,
                "year": "1992",
                "players": 2,
                "orientation": "horizontal",
                "parent": Value::Null,
                "clone": Value::Null,
                "bios": Value::Null,
            })
        })
        .collect();
    serde_json::to_vec(&json!({"schema": 1, "platform": platform, "sets": sets})).unwrap()
}

/// 一份合法平台包草稿：核心 + 两份 DAT。测试只改其中一个字段再断言。
struct Draft {
    meta: Value,
    files: Vec<(String, Vec<u8>)>,
}

impl Draft {
    fn valid(core: &[u8]) -> Draft {
        let meta = json!({
            "schema": 2,
            "kind": "platform",
            "id": PACK,
            "name": "街机 & NeoGeo · FBNeo",
            "version": "1.0.0",
            "min_app": "1.1.0",
            "extension_api": 1,
            "license": "Non-commercial",
            "source": "https://example.invalid/fbneo",
            "platforms": [platform_decl("arcade"), platform_decl("neogeo")],
            "cores": [core_decl()],
            "files": [],
        });
        Draft {
            meta,
            files: vec![
                (SO.to_string(), core.to_vec()),
                (dat_path(PACK, "arcade"), dat_json("arcade")),
                (dat_path(PACK, "neogeo"), dat_json("neogeo")),
            ],
        }
    }

    fn with_file(mut self, path: &str, data: &[u8]) -> Draft {
        self.files.push((path.to_string(), data.to_vec()));
        self
    }

    fn file_index(&self, path: &str) -> usize {
        self.files
            .iter()
            .position(|(candidate, _)| candidate == path)
            .unwrap_or_else(|| panic!("夹具里没有 {path}"))
    }

    /// `pack.json` 里的 `files`（按真实字节算 size / sha256）。
    fn declared_files(&self) -> Value {
        Value::Array(
            self.files
                .iter()
                .map(|(path, data)| {
                    json!({"path": path, "size": data.len(), "sha256": sha256(data)})
                })
                .collect(),
        )
    }

    fn manifest(&self) -> Vec<u8> {
        let mut meta = self.meta.clone();
        meta["files"] = self.declared_files();
        serde_json::to_vec(&meta).unwrap()
    }

    fn bytes(&self) -> Vec<u8> {
        let mut entries: Vec<(String, Vec<u8>)> =
            vec![("pack.json".to_string(), self.manifest())];
        entries.extend(self.files.iter().cloned());
        zip_stored(&entries)
    }

    /// 实际数据很小，但 `pack.json` 声明的体积是 `sizes`——用来测体积门。
    fn bytes_declaring(&self, sizes: &[usize]) -> Vec<u8> {
        let mut entries: Vec<(String, Vec<u8>)> =
            vec![("pack.json".to_string(), self.manifest_declaring(sizes))];
        entries.extend(self.files.iter().cloned());
        zip_stored(&entries)
    }

    fn manifest_declaring(&self, sizes: &[usize]) -> Vec<u8> {
        let mut meta = self.meta.clone();
        meta["files"] = Value::Array(
            self.files
                .iter()
                .zip(sizes.iter())
                .map(|((path, data), size)| {
                    json!({"path": path, "size": size, "sha256": sha256(data)})
                })
                .collect(),
        );
        serde_json::to_vec(&meta).unwrap()
    }

    /// 数据不动，只把某个文件的**声明 sha256** 写错（全 0）。
    fn bytes_corrupting_hash_of(&self, path: &str) -> Vec<u8> {
        let mut meta = self.meta.clone();
        meta["files"] = Value::Array(
            self.files
                .iter()
                .map(|(candidate, data)| {
                    let digest = if candidate == path {
                        "0".repeat(64)
                    } else {
                        sha256(data)
                    };
                    json!({"path": candidate, "size": data.len(), "sha256": digest})
                })
                .collect(),
        );
        let mut entries: Vec<(String, Vec<u8>)> =
            vec![("pack.json".to_string(), serde_json::to_vec(&meta).unwrap())];
        entries.extend(self.files.iter().cloned());
        zip_stored(&entries)
    }
}

fn core_pack(id: &str, data: &[u8], version: &str) -> Vec<u8> {
    let path = format!("System/linkSlot/cores/{id}/{id}_libretro.so");
    let meta = json!({
        "schema": 1,
        "kind": "core",
        "id": id,
        "name": format!("QA {id}"),
        "version": version,
        "files": [{"path": path, "size": data.len(), "sha256": sha256(data)}],
    });
    zip_stored(&[
        ("pack.json".to_string(), serde_json::to_vec(&meta).unwrap()),
        (path, data.to_vec()),
    ])
}

// ---------------------------------------------------------------- 断言助手

/// 被拒绝的包不许在卡上留下任何痕迹：没有记录，连 `System/` `linkSlot/` 都不该出现。
fn assert_nothing_written(root: &Path) {
    assert!(
        installed(root).is_empty(),
        "被拒绝的包写了安装记录：{:?}",
        installed(root).iter().map(|pack| pack.id.clone()).collect::<Vec<_>>()
    );
    for dir in ["System", "linkSlot"] {
        assert!(
            !root.join(dir).exists(),
            "被拒绝的包创建了 {dir}/，这些文件以后再也找不到归属"
        );
    }
}

fn expect_rejected(draft: &Draft, label: &str) -> String {
    let card = tempfile::tempdir().unwrap();
    let bytes = draft.bytes();
    let error = match install(card.path(), &bytes) {
        Ok(pack) => panic!("{label} 应当被拒绝，却装成了 {}", pack.id),
        Err(error) => error,
    };
    assert_nothing_written(card.path());
    error
}

fn boot(card: &Path) -> Vec<(Platform, String)> {
    let mut carts: Vec<(Platform, String)> = scan_quiet(card)
        .expect("扫描不该失败")
        .into_iter()
        .map(|cart| (cart.platform, cart.stem))
        .collect();
    carts.sort_by_key(|(platform, _)| *platform as u8);
    carts
}

fn backups(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join(RECORD_DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("installed.json.bad"))
        })
        .collect();
    out.sort();
    out
}

/// 目录树下的所有普通文件（包括子目录里的）。
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => out.extend(files_under(&path)),
            Ok(kind) if kind.is_file() => out.push(path),
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------- 1. 脚本 ↔ app 端到端

fn find_python() -> Option<&'static str> {
    for candidate in ["python3", "python", "py"] {
        let ran = Command::new(candidate)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if ran {
            return Some(candidate);
        }
    }
    None
}

/// 用仓库真实的 `build/pack_lsp.py` 打一个 `.lsp`，再交给 `packs::install`。
///
/// 这条同时钉住两件事：打包脚本产出的形状（schema 2 + 多 payload + deflate）能被
/// app 读进来；装完之后激活门、核心选择器、扫描器都真的认到两台机种。
#[test]
fn the_python_builder_output_installs_activates_and_scans() {
    let Some(python) = find_python() else {
        eprintln!("SKIP the_python_builder_output_installs_activates_and_scans: 没有 python3");
        return;
    };

    let work = tempfile::tempdir().unwrap();
    // 64 KiB 伪随机核心（确定性，便于复现；内容不重要，字节一致才是断言点）。
    let core: Vec<u8> = (0..64 * 1024)
        .map(|index| ((index * 37 + 11) % 251) as u8)
        .collect();
    let core_file = work.path().join("fbneo_libretro.so");
    std::fs::write(&core_file, &core).unwrap();
    let arcade_file = work.path().join("arcade.dat.json");
    std::fs::write(&arcade_file, dat_json("arcade")).unwrap();
    let neogeo_file = work.path().join("neogeo.dat.json");
    std::fs::write(&neogeo_file, dat_json("neogeo")).unwrap();
    let out = work.path().join("linkslot-pack-arcade-neogeo-v1.0.0.lsp");

    let repo = repo_root();
    let output = Command::new(python)
        .current_dir(&repo)
        .arg("build/pack_lsp.py")
        .arg("--meta")
        .arg("packs/arcade-neogeo/pack.json")
        .arg("--payload")
        .arg(format!("{SO}={}", core_file.display()))
        .arg("--payload")
        .arg(format!(
            "{}={}",
            dat_path(PACK, "arcade"),
            arcade_file.display()
        ))
        .arg("--payload")
        .arg(format!(
            "{}={}",
            dat_path(PACK, "neogeo"),
            neogeo_file.display()
        ))
        .arg("--out")
        .arg(&out)
        .output()
        .expect("spawn build/pack_lsp.py");
    assert!(
        output.status.success(),
        "打包脚本失败：\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "pack_lsp.py: {}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
    let bytes = std::fs::read(&out).expect("脚本说成功但 .lsp 不在");

    let card = tempfile::tempdir().unwrap();
    write_file(card.path(), "Games/Arcade/a.zip", b"PK\x03\x04 arcade rom");
    write_file(card.path(), "Games/NeoGeo/b.zip", b"PK\x03\x04 neogeo rom");

    // 装包之前：目录里有 ROM 也不算数（不装包就不识别）。
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert_eq!(platform_packs::available_platforms(card.path()).len(), 6);
    assert!(available_options(card.path(), Platform::Arcade).is_empty());
    assert!(boot(card.path()).is_empty(), "没装包却扫出了街机 Cart");

    let installed_pack = install(card.path(), &bytes).expect("脚本打出来的 .lsp 必须能被安装");
    assert_eq!(installed_pack.kind, "platform");
    assert_eq!(installed_pack.id, PACK);
    assert_eq!(
        installed_pack.platforms,
        vec!["arcade".to_string(), "neogeo".to_string()]
    );
    assert_eq!(installed_pack.cores, vec!["fbneo".to_string()]);
    assert_eq!(installed_pack.version, "1.0.0");
    assert_eq!(installed_pack.min_app, "1.1.0");

    // 落盘位置与字节。
    assert_eq!(
        std::fs::read(card.path().join(SO)).unwrap(),
        core,
        "核心字节与打包前不一致"
    );
    for platform in ["arcade", "neogeo"] {
        assert_eq!(
            std::fs::read(card.path().join(dat_path(PACK, platform))).unwrap(),
            dat_json(platform),
            "{platform}.dat.json 字节与打包前不一致"
        );
    }

    // 记录。
    let listed = installed(card.path());
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, PACK);
    assert_eq!(
        listed[0].platforms,
        vec!["arcade".to_string(), "neogeo".to_string()]
    );
    assert_eq!(listed[0].cores, vec!["fbneo".to_string()]);

    // 激活门 + 选择器 + 扫描器。
    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    let available = platform_packs::available_platforms(card.path());
    assert_eq!(available.len(), 8, "内置 6 台 + 激活 2 台：{available:?}");
    assert!(available.contains(&Platform::Arcade));
    assert!(available.contains(&Platform::Neogeo));
    assert_eq!(
        available_options(card.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );
    assert_eq!(
        available_options(card.path(), Platform::Neogeo),
        vec![Core::Fbneo]
    );
    assert_eq!(
        boot(card.path()),
        vec![
            (Platform::Arcade, "a".to_string()),
            (Platform::Neogeo, "b".to_string())
        ],
        "装包后应恰好扫出这两张卡"
    );
}

// ---------------------------------------------------------------- 2. 卸载保留用户数据

#[test]
fn uninstall_keeps_user_data_and_only_removes_pack_files() {
    let card = tempfile::tempdir().unwrap();
    let core = b"\x7fELF fbneo for the device".to_vec();
    install(card.path(), &Draft::valid(&core).bytes()).expect("合法平台包");

    let keepers: [(&str, &[u8]); 6] = [
        ("Games/Arcade/a.zip", b"PK\x03\x04 arcade rom".as_slice()),
        ("BIOS/fbneo/neogeo.zip", b"PK\x03\x04 neogeo bios".as_slice()),
        ("Saves/Arcade/fbneo/x.sav", b"save bytes".as_slice()),
        ("States/Arcade/fbneo/y.state", b"state bytes".as_slice()),
        ("Cheats/Arcade/z.cht", b"cheat bytes".as_slice()),
        (
            "linkSlot/Config/selected_core.txt",
            b"[a]\ncore=fbneo\n".as_slice(),
        ),
    ];
    for (path, data) in keepers {
        write_file(card.path(), path, data);
    }

    uninstall(card.path(), PACK).expect("卸载");

    for (path, data) in keepers {
        assert_eq!(
            std::fs::read(card.path().join(path)).unwrap(),
            data,
            "卸载动了用户数据：{path}"
        );
    }
    assert!(!card.path().join(SO).exists(), "核心没删掉");
    for platform in ["arcade", "neogeo"] {
        assert!(!card.path().join(dat_path(PACK, platform)).exists());
    }
    assert!(
        !card.path().join(PLATFORM_ROOT).join(PACK).exists(),
        "extension 目录没清理：重装会被「目录已存在」的假象干扰"
    );
    assert!(installed(card.path()).is_empty());
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
    assert!(boot(card.path()).is_empty(), "卸载后还在扫街机");

    // 再装回来：核心与激活状态都恢复，用户数据仍逐字节都在。
    install(card.path(), &Draft::valid(&core).bytes()).expect("重装");
    assert!(card.path().join(SO).is_file());
    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    for (path, data) in keepers {
        assert_eq!(std::fs::read(card.path().join(path)).unwrap(), data);
    }
    assert_eq!(
        boot(card.path()),
        vec![(Platform::Arcade, "a".to_string())],
        "重装后只该看见 Arcade 那张卡（b.zip 属于 NeoGeo）"
    );
}

// ---------------------------------------------------------------- 3. 拒绝矩阵

#[test]
fn rejects_unknown_and_duplicate_platforms() {
    let mut unknown = Draft::valid(b"core");
    unknown.meta["platforms"][1] = json!({
        "id": "snes", "name": "SNES", "folder": "Games/Snes",
        "rom_extensions": ["zip"], "scanner": "arcade_zip_v1",
        "ui": "arcade_cabinet_v1", "ui_profile": "snes",
        "input": "arcade_6button_v1", "core_ids": ["fbneo"],
        "default_core": "fbneo", "bios": [],
    });
    let error = expect_rejected(&unknown, "未知平台 snes");
    assert!(error.contains("未知平台"), "{error}");

    let mut duplicate = Draft::valid(b"core");
    duplicate.meta["platforms"][1] = platform_decl("arcade");
    let error = expect_rejected(&duplicate, "platforms 重复");
    assert!(error.contains("重复"), "{error}");
}

#[test]
fn rejects_unknown_scanner_ui_input_and_ui_profile() {
    for (field, value) in [
        ("scanner", json!("arcade_zip_v9")),
        ("ui", json!("arcade_cabinet_v9")),
        ("input", json!("arcade_4button_v1")),
        ("ui_profile", json!("neogeo")),
    ] {
        let mut draft = Draft::valid(b"core");
        draft.meta["platforms"][0][field] = value;
        let error = expect_rejected(&draft, &format!("platforms[0].{field}"));
        assert!(
            error.contains(field),
            "{field} 的错误信息没有点名字段：{error}"
        );
    }
}

#[test]
fn rejects_a_pack_meant_for_another_app_version() {
    let mut api = Draft::valid(b"core");
    api.meta["extension_api"] = json!(2);
    let error = expect_rejected(&api, "extension_api=2");
    assert!(error.contains("extension_api"), "{error}");

    let mut missing = Draft::valid(b"core");
    missing.meta.as_object_mut().unwrap().remove("min_app");
    let error = expect_rejected(&missing, "缺 min_app");
    assert!(error.contains("min_app"), "{error}");

    let mut future = Draft::valid(b"core");
    future.meta["min_app"] = json!("99.0.0");
    let error = expect_rejected(&future, "min_app=99.0.0");
    assert!(error.contains("99.0.0"), "{error}");
}

#[test]
fn rejects_a_core_that_cannot_run_arcade() {
    let mut draft = Draft::valid(b"core");
    // 换成已知但跑不了街机的 mgba，并把路径 / dylib / core_ids 一起改到位，
    // 否则会先在"引用了未声明的核心"上被拒，测不到"跑不了"这条规则。
    let index = draft.file_index(SO);
    draft.files[index].0 = "System/linkSlot/cores/mgba/mgba_libretro.so".to_string();
    draft.meta["cores"][0]["id"] = json!("mgba");
    draft.meta["cores"][0]["dylib"] = json!("mgba_libretro.so");
    for decl in [0usize, 1] {
        draft.meta["platforms"][decl]["core_ids"] = json!(["mgba"]);
        draft.meta["platforms"][decl]["default_core"] = json!("mgba");
    }
    let error = expect_rejected(&draft, "核心跑不了街机");
    assert!(error.contains("跑不了"), "{error}");
}

#[test]
fn rejects_a_second_core_and_a_pack_without_its_core() {
    let mut two = Draft::valid(b"core");
    two.meta["cores"] = json!([
        core_decl(),
        {
            "id": "mame2003_plus",
            "dylib": "mame2003_plus_libretro.so",
            "display": "MAME 2003-Plus",
            "license": "Non-commercial",
            "source": "https://example.invalid/mame",
            "system_dir": "BIOS/mame",
        }
    ]);
    let error = expect_rejected(&two, "两个核心");
    assert!(error.contains("一个核心"), "{error}");

    let mut missing = Draft::valid(b"core");
    missing.files.retain(|(path, _)| !path.ends_with("_libretro.so"));
    let error = expect_rejected(&missing, "缺 .so");
    assert!(error.contains("必须带核心"), "{error}");
}

#[test]
fn rejects_oversized_single_files_and_payloads() {
    let draft = Draft::valid(b"core");

    // 单文件 100 MiB > 96 MiB。
    let card = tempfile::tempdir().unwrap();
    let error = install(card.path(), &draft.bytes_declaring(&[100 * MIB, 200, 200])).unwrap_err();
    assert!(error.contains("文件大小"), "{error}");
    assert_nothing_written(card.path());

    // 每个都合规（70 / 60 MiB），但总量 130 MiB > 128 MiB。
    let card = tempfile::tempdir().unwrap();
    let error = install(card.path(), &draft.bytes_declaring(&[70 * MIB, 60 * MIB, 200])).unwrap_err();
    assert!(error.contains("128 MiB"), "{error}");
    assert_nothing_written(card.path());
}

#[test]
fn rejects_paths_outside_the_platform_whitelist() {
    let cases = [
        "System/linkSlot/bin/linkslot",
        "linkSlot/Config/x.json",
        "System/linkSlot/cores/mgba/mgba_libretro.so",
        "System/linkSlot/extensions/platforms/other-pack/metadata/arcade.dat.json",
        "System/linkSlot/extensions/platforms/arcade-neogeo/assets/run.sh",
        "System/linkSlot/extensions/platforms/arcade-neogeo/metadata/other.dat.json",
        "System/linkSlot/extensions/platforms/arcade-neogeo/assets/../../escape.png",
    ];
    for path in cases {
        let draft = Draft::valid(b"core").with_file(path, b"payload");
        let error = expect_rejected(&draft, path);
        assert!(
            !error.trim().is_empty(),
            "{path} 被拒绝了，但没给出理由"
        );
    }
}

// ---------------------------------------------------------------- 4. 重复平台：一个字节都不写

#[test]
fn a_second_pack_claiming_arcade_writes_nothing() {
    let card = tempfile::tempdir().unwrap();
    let original = b"\x7fELF the original fbneo".to_vec();
    install(card.path(), &Draft::valid(&original).bytes()).expect("第一个包");
    let before = installed(card.path());
    assert_eq!(before.len(), 1);
    let record_before = std::fs::read(record_path(card.path())).unwrap();

    // 另一个 id，但同样点名 arcade：两包共用同一个 fbneo `.so`，
    // 卸载任一个都会删掉对方的文件。
    let mut rival = Draft::valid(b"\x7fELF the rival fbneo");
    rival.meta["id"] = json!("arcade-solo");
    rival.meta["name"] = json!("Arcade only");
    rival.meta["platforms"] = json!([platform_decl("arcade")]);
    rival
        .files
        .retain(|(path, _)| path.as_str() != dat_path(PACK, "neogeo").as_str());
    let old_dat = dat_path(PACK, "arcade");
    let new_dat = dat_path("arcade-solo", "arcade");
    for (path, _) in rival.files.iter_mut() {
        if path.as_str() == old_dat.as_str() {
            *path = new_dat.clone();
        }
    }
    let error = install(card.path(), &rival.bytes()).expect_err("第二个包必须被拒");
    assert!(error.contains("已由扩展包"), "{error}");

    assert_eq!(installed(card.path()), before, "记录被动了");
    assert_eq!(
        std::fs::read(record_path(card.path())).unwrap(),
        record_before,
        "installed.json 被改写了"
    );
    assert_eq!(
        std::fs::read(card.path().join(SO)).unwrap(),
        original,
        "原来的核心被覆盖了"
    );
    assert!(
        !card.path().join(PLATFORM_ROOT).join("arcade-solo").exists(),
        "被拒的包建了目录"
    );
    // 暂存目录 `packs/stage/` 本身由**第一次成功安装**留下（空目录，见报告 QA-2），
    // 这里要断言的是：被拒的这一轮没有往里写任何东西。
    let stage = card.path().join(RECORD_DIR).join("stage");
    assert!(
        !stage.join("arcade-solo").exists(),
        "被拒的包留下了自己的暂存目录"
    );
    assert!(
        files_under(&stage).is_empty(),
        "被拒的包在暂存目录里写了文件：{:?}",
        files_under(&stage)
    );
    assert!(platform_packs::active(card.path(), Platform::Arcade));
}

// ---------------------------------------------------------------- 5. NOTE-2 回归

#[test]
fn note_2_a_core_pack_does_not_clobber_the_platform_record() {
    let card = tempfile::tempdir().unwrap();
    install(card.path(), &Draft::valid(b"\x7fELF fbneo").bytes()).expect("平台包");

    let supafaust = b"\x7fELF supafaust".to_vec();
    let core_pack = core_pack("supafaust", &supafaust, "1.0.0");
    install(card.path(), &core_pack).expect("核心包");

    let listed = installed(card.path());
    assert_eq!(
        listed.len(),
        2,
        "两条记录：{:?}",
        listed.iter().map(|pack| pack.id.clone()).collect::<Vec<_>>()
    );
    let platform = listed
        .iter()
        .find(|pack| pack.id == PACK)
        .expect("核心包把平台记录挤掉了");
    assert_eq!(
        platform.platforms,
        vec!["arcade".to_string(), "neogeo".to_string()]
    );
    assert_eq!(platform.cores, vec!["fbneo".to_string()]);

    // 盘上的 JSON 也必须还有这两个键（结构体漏声明一个字段就会在这一轮丢掉）。
    let raw = std::fs::read_to_string(record_path(card.path())).unwrap();
    assert!(raw.contains("\"platforms\""), "盘上记录没有 platforms：{raw}");
    assert!(raw.contains("\"cores\""), "盘上记录没有 cores：{raw}");
    assert!(platform_packs::active(card.path(), Platform::Arcade));

    // 卸核心包：平台仍然活着。
    uninstall(card.path(), "supafaust").unwrap();
    assert_eq!(installed(card.path()).len(), 1);
    assert!(
        platform_packs::active(card.path(), Platform::Arcade),
        "卸核心包把平台带走了"
    );
    assert!(platform_packs::active(card.path(), Platform::Neogeo));

    // 反过来：装回核心包，再卸平台包，核心记录仍应在。
    install(card.path(), &core_pack).unwrap();
    uninstall(card.path(), PACK).unwrap();
    let left = installed(card.path());
    assert_eq!(left.len(), 1, "{:?}", left.iter().map(|p| p.id.clone()).collect::<Vec<_>>());
    assert_eq!(left[0].id, "supafaust");
    assert_eq!(
        std::fs::read(card.path().join("System/linkSlot/cores/supafaust/supafaust_libretro.so"))
            .unwrap(),
        supafaust,
        "卸平台包动了核心包的文件"
    );
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
}

// ---------------------------------------------------------------- 6. 坏记录备份

#[test]
fn a_corrupt_record_is_backed_up_and_a_second_one_does_not_overwrite_it() {
    let card = tempfile::tempdir().unwrap();
    let record = record_path(card.path());
    std::fs::create_dir_all(record.parent().unwrap()).unwrap();
    let first = b"{ this is not json".to_vec();
    let second = b"[1,2,3,".to_vec();

    std::fs::write(&record, &first).unwrap();
    assert!(installed(card.path()).is_empty(), "坏记录必须当空表处理");
    let found = backups(card.path());
    assert_eq!(found.len(), 1, "没有留下备份：{found:?}");
    assert_eq!(
        std::fs::read(&found[0]).unwrap(),
        first,
        ".bad 的内容必须逐字节等于坏字节"
    );
    assert!(!record.exists(), "坏文件还留在原位，下次安装会把它覆盖掉");

    std::fs::write(&record, &second).unwrap();
    assert!(installed(card.path()).is_empty());
    let found = backups(card.path());
    assert_eq!(found.len(), 2, "第二次损坏覆盖了第一次的证据：{found:?}");
    let contents: Vec<Vec<u8>> = found
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect();
    assert!(contents.contains(&first), "第一份证据被覆盖了：{found:?}");
    assert!(contents.contains(&second), "第二份证据没留下：{found:?}");
}

// ---------------------------------------------------------------- 7. 宿主语义

#[test]
fn a_device_so_alone_activates_the_platform() {
    let card = tempfile::tempdir().unwrap();
    install(card.path(), &Draft::valid(b"\x7fELF device fbneo").bytes()).expect("平台包");

    // 包里的核心是字面量 `.so`（设备卡格式）。桌面上它不是"本机可加载的核心"，
    // 但平台仍必须亮：这是刻意分开的两条判断。
    assert!(card.path().join(SO).is_file());
    assert!(platform_packs::active(card.path(), Platform::Arcade));
    assert!(platform_packs::active(card.path(), Platform::Neogeo));
    assert_eq!(
        available_options(card.path(), Platform::Arcade),
        vec![Core::Fbneo]
    );

    #[cfg(windows)]
    {
        // 只有 `gpsp_libretro.so` 的根目录：Windows 上宿主扩展名是 .dll，
        // 因此 `Core::is_available` 必须为 false，GBA 选择器不该多出 gpSP。
        let other = tempfile::tempdir().unwrap();
        write_file(
            other.path(),
            "System/linkSlot/cores/gpsp/gpsp_libretro.so",
            b"\x7fELF gpSP (device build)",
        );
        assert!(
            !Core::Gpsp.is_available(other.path()),
            "桌面板把设备用的 .so 当成了可加载核心"
        );
        assert!(
            !available_options(other.path(), Platform::Gba).contains(&Core::Gpsp),
            "GBA 选择器多出了桌面加载不了的 gpSP"
        );
    }

    #[cfg(unix)]
    {
        let other = tempfile::tempdir().unwrap();
        write_file(
            other.path(),
            "System/linkSlot/cores/gpsp/gpsp_libretro.so",
            b"\x7fELF gpSP",
        );
        if std::env::consts::DLL_EXTENSION == "so" {
            assert!(Core::Gpsp.is_available(other.path()));
        } else {
            assert!(!Core::Gpsp.is_available(other.path()));
        }
    }
}

// ---------------------------------------------------------------- 8. 刷新信号

fn http_post(port: u16, path: &str, content_type: &str, body: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("连接本地 Web 服务");
    stream
        .set_read_timeout(Some(Duration::from_secs(120)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(120)))
        .unwrap();
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: {content_type}\r\n\
         X-Slot-Web: 1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw).to_string();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, text)
}

/// 真的起一个 Web 服务，用 HTTP 打 `/api/packs/install` 与 `/api/packs/remove`。
///
/// 只断言"成功才置位、失败不置位"：这就是书架上街机能否立刻出现/消失的全部条件。
#[test]
fn install_and_remove_stamp_the_library_changed() {
    let card = tempfile::tempdir().unwrap();
    let mut server = match crate::web::WebServer::start(card.path().to_path_buf()) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("SKIP install_and_remove_stamp_the_library_changed: Web 服务起不来：{error}");
            return;
        }
    };
    let port = server.port();
    eprintln!("Web 服务端口 {port}");

    let body = Draft::valid(b"\x7fELF fbneo").bytes();
    let (status, text) = http_post(port, "/api/packs/install", "application/octet-stream", &body);
    assert_eq!(status, 200, "安装应成功：{text}");
    assert!(
        server.take_library_changed(),
        "安装成功后没有置位 library_changed，街机要重启才出现"
    );

    // 失败路径不得置位：一个坏包不该让整台机器重扫游戏库。
    let (status, _) = http_post(
        port,
        "/api/packs/install",
        "application/octet-stream",
        b"not a zip at all",
    );
    assert_eq!(status, 400);
    assert!(
        !server.take_library_changed(),
        "安装失败却置位了 library_changed"
    );

    let (status, text) = http_post(
        port,
        "/api/packs/remove",
        "application/json",
        br#"{"id":"arcade-neogeo"}"#,
    );
    assert_eq!(status, 200, "卸载应成功：{text}");
    assert!(
        server.take_library_changed(),
        "卸载成功后没有置位 library_changed，货架会留着点不开的条目"
    );

    let (status, _) = http_post(
        port,
        "/api/packs/remove",
        "application/json",
        br#"{"id":"nope"}"#,
    );
    assert_eq!(status, 400);
    assert!(
        !server.take_library_changed(),
        "卸载失败却置位了 library_changed"
    );

    server.stop();
    assert!(!platform_packs::active(card.path(), Platform::Arcade));
}

// ---------------------------------------------------------------- 9. 常量与模板

/// 文档写的体积门（96 / 128 MiB）必须就是代码里的门。
#[test]
fn the_documented_limits_are_96_and_128_mib() {
    let draft = Draft::valid(b"core");

    // 单文件 96 MiB 恰好通过；96 MiB + 1 被拒。
    assert!(
        parse_manifest(&draft.manifest_declaring(&[96 * MIB, 200, 200])).is_ok(),
        "文档写单文件 ≤ 96 MiB，96 MiB 却装不了"
    );
    let error = parse_manifest(&draft.manifest_declaring(&[96 * MIB + 1, 200, 200])).unwrap_err();
    assert!(error.contains("文件大小"), "{error}");

    // 总量 128 MiB 恰好通过；多 200 字节就被拒。
    assert!(
        parse_manifest(&draft.manifest_declaring(&[64 * MIB, 64 * MIB - 200, 200])).is_ok(),
        "文档写总量 ≤ 128 MiB，128 MiB 却装不了"
    );
    let error = parse_manifest(&draft.manifest_declaring(&[64 * MIB, 64 * MIB, 200])).unwrap_err();
    assert!(error.contains("128 MiB"), "{error}");
}

/// QA-1 回归：`packs.rs` 声明的 96 MiB 单文件门必须**真的可达**。
///
/// 历史：`packs::install` 曾经直接调 `upgrade::parse_zip`，而那里的单条目上限是
/// 64 MiB，于是 96 MiB 的门形同虚设——FBNeo 的 aarch64 `.so` 有 79.6 MiB，会在
/// 解压阶段就被判 `ZIP 单文件超过 64 MiB`，唯一的官方平台包装不进卡。
/// 现在 packs 走自己的条目上限（`parse_zip_with_entry_limit`，64 MiB 那条留给
/// 升级包），所以 65 MiB 的核心必须能装、且落盘字节完整。
#[test]
fn a_65_mib_core_installs_now_that_packs_have_their_own_zip_entry_limit() {
    let card = tempfile::tempdir().unwrap();
    let core = vec![0xA5u8; 65 * MIB];
    let bytes = Draft::valid(&core).bytes();
    install(card.path(), &bytes).expect("96 MiB 单文件门之内的核心必须装得进去");

    let landed = std::fs::metadata(card.path().join(SO)).expect("核心没落盘");
    assert_eq!(landed.len() as usize, core.len(), "落盘长度与核心不一致");
    assert_eq!(
        std::fs::read(card.path().join(SO)).unwrap(),
        core,
        "落盘字节与核心不一致"
    );
    assert_eq!(installed(card.path()).len(), 1);
    assert!(platform_packs::active(card.path(), Platform::Arcade));
}

/// 修 QA-1 时不能顺手把**升级包**的门也放宽。
///
/// `parse_zip` 仍按 64 MiB 拒单条目（升级包路径的峰值内存与语义不变），只有
/// packs 走的那个入口按 96 MiB 放行。同一个 65 MiB 包喂给两个入口，结论必须不同。
#[test]
fn the_upgrade_pack_entry_cap_stays_at_64_mib() {
    let bytes = Draft::valid(&vec![0xA5u8; 65 * MIB]).bytes();

    let error = crate::upgrade::parse_zip(&bytes).expect_err("升级包路径不该被放宽到 96 MiB");
    assert!(
        error.contains("64 MiB"),
        "升级包的单条目上限变了：{error}"
    );
    assert!(
        crate::upgrade::parse_zip_with_entry_limit(&bytes, 96 * MIB).is_ok(),
        "packs 用的入口应当接受 96 MiB 之内的条目"
    );
}

/// QA-2 回归：成功安装不在卡上留下 `linkSlot/Config/packs/stage/`。
///
/// 暂存目录只该是安装过程中的临时落脚点；安装结束（无论成败）卡上都不该多出
/// 一个空目录。旧实现只删了 `stage/<id>`，父目录 `stage/` 会一直留着。
#[test]
fn a_successful_install_leaves_no_stage_directory() {
    let card = tempfile::tempdir().unwrap();
    install(card.path(), &Draft::valid(b"\x7fELF fbneo").bytes()).expect("合法平台包");
    let stage = card.path().join(RECORD_DIR).join("stage");
    assert!(
        !stage.exists(),
        "安装结束后还留着暂存目录：{}",
        stage.display()
    );
    assert!(files_under(&stage).is_empty());
}

/// QA-4 回归：坏包先报**文件校验**，而不是抢着报"平台已被占用"。
///
/// 一个既声称了已占用平台、sha256 又对不上的包，用户最需要知道的是"你这个包
/// 坏了"，而不是"arcade 已经有人占了"。两条都在落盘之前，语义不变。
#[test]
fn a_corrupt_pack_that_claims_a_taken_platform_reports_the_file_error_first() {
    let card = tempfile::tempdir().unwrap();
    let original = b"\x7fELF the original fbneo".to_vec();
    install(card.path(), &Draft::valid(&original).bytes()).expect("第一个包");
    let before = installed(card.path());

    let mut rival = Draft::valid(b"\x7fELF the rival fbneo");
    rival.meta["id"] = json!("arcade-solo");
    rival.meta["platforms"] = json!([platform_decl("arcade")]);
    rival
        .files
        .retain(|(path, _)| path.as_str() != dat_path(PACK, "neogeo").as_str());
    let old_dat = dat_path(PACK, "arcade");
    let new_dat = dat_path("arcade-solo", "arcade");
    for (path, _) in rival.files.iter_mut() {
        if path.as_str() == old_dat.as_str() {
            *path = new_dat.clone();
        }
    }
    // 故意把声明的 sha256 写错（数据本身不动，所以是"声明的哈希对不上"）。
    let bytes = rival.bytes_corrupting_hash_of(SO);

    let error = install(card.path(), &bytes).expect_err("哈希对不上的包必须被拒");
    eprintln!("坏包 + 已占用平台的报错：{error}");
    assert!(
        !error.contains("已由扩展包"),
        "坏包抢先报了平台占用，用户会以为是自己装错了包：{error}"
    );
    assert!(
        error.contains("sha256") || error.contains("哈希") || error.contains("校验"),
        "报错没指向文件完整性：{error}"
    );
    assert_eq!(installed(card.path()), before, "记录被动了");
    assert_eq!(
        std::fs::read(card.path().join(SO)).unwrap(),
        original,
        "原核心被覆盖了"
    );
    assert!(!card.path().join(PLATFORM_ROOT).join("arcade-solo").exists());
}

/// 仓库里那份 `packs/arcade-neogeo/pack.json` 必须与设计稿 §5.2 一致，且能过校验。
#[test]
fn the_shipped_template_matches_the_design() {
    let template = include_str!("../../../packs/arcade-neogeo/pack.json");
    let meta: Value = serde_json::from_str(template).expect("模板必须是合法 JSON");

    assert_eq!(meta["schema"], json!(2));
    assert_eq!(meta["kind"], json!("platform"));
    assert_eq!(meta["id"], json!(PACK));
    assert_eq!(meta["extension_api"], json!(1));
    assert!(
        !meta["min_app"].as_str().unwrap_or_default().is_empty(),
        "平台包必须写 min_app"
    );

    let platforms = meta["platforms"].as_array().expect("platforms 必须是数组");
    assert_eq!(platforms.len(), 2);
    let ids: Vec<&str> = platforms
        .iter()
        .map(|decl| decl["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["arcade", "neogeo"]);
    // §5.2：arcade 不需要 BIOS，neogeo 需要 neogeo.zip；folder / profile 一一对应。
    assert_eq!(platforms[0]["folder"], json!("Games/Arcade"));
    assert_eq!(platforms[0]["ui_profile"], json!("arcade"));
    assert_eq!(platforms[0]["bios"], json!([]));
    assert_eq!(platforms[1]["folder"], json!("Games/NeoGeo"));
    assert_eq!(platforms[1]["ui_profile"], json!("neogeo"));
    assert_eq!(platforms[1]["bios"], json!(["neogeo.zip"]));
    for decl in platforms {
        assert_eq!(decl["scanner"], json!("arcade_zip_v1"));
        assert_eq!(decl["ui"], json!("arcade_cabinet_v1"));
        assert_eq!(decl["input"], json!("arcade_6button_v1"));
        assert_eq!(decl["rom_extensions"], json!(["zip"]));
        assert_eq!(decl["core_ids"], json!(["fbneo"]));
        assert_eq!(decl["default_core"], json!("fbneo"));
    }

    let cores = meta["cores"].as_array().expect("cores 必须是数组");
    assert_eq!(cores.len(), 1, "v1.1.0 的平台包只允许一个核心");
    assert_eq!(cores[0]["id"], json!("fbneo"));
    assert_eq!(cores[0]["dylib"], json!("fbneo_libretro.so"));
    assert!(!cores[0]["license"].as_str().unwrap_or_default().is_empty());
    assert_eq!(cores[0]["system_dir"], json!("BIOS/fbneo"));
    assert_eq!(meta["files"], json!([]), "模板的 files 由打包脚本填");

    // 模板本身（files 补齐后）必须能过 app 的校验。
    let mut with_files = meta.clone();
    with_files["files"] = Draft::valid(b"core").declared_files();
    let parsed = parse_manifest(&serde_json::to_vec(&with_files).unwrap())
        .expect("仓库模板必须能过 app 的校验");
    assert_eq!(parsed.id, PACK);
    assert_eq!(parsed.platforms.len(), 2);
    assert_eq!(parsed.cores.len(), 1);
    assert_eq!(parsed.cores[0].license, "Non-commercial");
}
