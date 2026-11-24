//! Compiles the vendored rcheevos runtime (MIT) into a static library.
//!
//! Only the runtime and utility sources are needed: no HTTP client, no hash
//! code and no client library. The rcheevos headers are pinned in
//! `third_party/rcheevos`.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let root = manifest.join("../../third_party/rcheevos");
    let include = root.join("include");
    let src = root.join("src");

    let mut files = Vec::new();
    for dir in ["runtime", "util"] {
        let path = src.join(dir);
        for entry in std::fs::read_dir(&path).expect("rcheevos source directory") {
            let entry = entry.expect("rcheevos source entry");
            let file = entry.path();
            if file.extension().and_then(|ext| ext.to_str()) == Some("c") {
                files.push(file);
            }
        }
    }
    files.sort();

    let mut build = cc::Build::new();
    build
        .include(&include)
        .include(&src)
        .files(&files)
        .define("_LARGEFILE64_SOURCE", None)
        .define("_FILE_OFFSET_BITS", "64")
        .warnings(false);
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        build.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    build.compile("rcheevos");

    println!("cargo:rerun-if-changed={}", include.display());
    println!("cargo:rerun-if-changed={}", src.display());
    rerun_tree(&include);
    rerun_tree(&src);
}

fn rerun_tree(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rerun_tree(&path);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("h")
            || path.extension().and_then(|ext| ext.to_str()) == Some("c")
        {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
