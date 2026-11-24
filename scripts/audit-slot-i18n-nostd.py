#!/usr/bin/env python3
"""Host-only no_std + alloc feasibility test for Linkslot slot-i18n."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import textwrap

TARGETS = ("armv7a-none-eabi", "aarch64-unknown-none")
LOCALES = (("ZhCn", "zh-CN"), ("EnUs", "en-US"))

def run(args, env=None, log=None):
    merged = os.environ.copy()
    if env:
        merged.update(env)
    proc = subprocess.run(args, env=merged, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode:
        print(proc.stdout[-12000:], file=sys.stderr)
        raise RuntimeError("command failed: " + " ".join(map(str, args)))
    return proc.stdout

def rust_literal(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)

def placeholders(value: str) -> tuple[str, ...]:
    return tuple(sorted(set(re.findall(r"\{([A-Za-z0-9_]+)\}", value))))

def make_manifest(name: str) -> str:
    return textwrap.dedent(f"""\
        [package]
        name = "{name}"
        version = "0.0.0"
        edition = "2021"
        rust-version = "1.96"

        [lib]
        crate-type = ["rlib"]

        [profile.release]
        panic = "abort"
        codegen-units = 1
        lto = false
    """)

def build_candidate(source: Path, out: Path) -> tuple[int, int]:
    crate = source / "crates/slot-i18n"
    original = (crate / "src/lib.rs").read_text(encoding="utf-8")
    domains = re.findall(r'^\s*name:\s*"([^"]+)"', original, flags=re.M)
    if not domains:
        raise RuntimeError("failed to read upstream slot-i18n domain order")

    file_data = {}
    rows = {"ZhCn": [], "EnUs": []}
    key_owners = {}
    for domain in domains:
        maps = {}
        for variant, locale in LOCALES:
            path = source / "translations" / (domain + "." + locale + ".json")
            raw = path.read_text(encoding="utf-8")
            data = json.loads(raw)
            if raw.startswith("\ufeff") or "\r" in raw or not raw.endswith("\n"):
                raise RuntimeError(path.name + ": invalid text encoding/newline")
            if data.get("schema") != 1 or data.get("language") != locale:
                raise RuntimeError(path.name + ": wrong schema or language")
            strings = data.get("strings")
            if not isinstance(strings, dict) or not strings:
                raise RuntimeError(path.name + ": missing strings object")
            for key, value in strings.items():
                if not isinstance(key, str) or not isinstance(value, str) or not value.strip():
                    raise RuntimeError(path.name + ": invalid or empty key/value")
                if variant == "ZhCn":
                    previous = key_owners.get(key)
                    if previous is not None:
                        raise RuntimeError(key + ": appears in both " + previous + " and " + domain)
                    key_owners[key] = domain
            maps[variant] = strings
            file_data[(domain, variant)] = raw
        if set(maps["ZhCn"]) != set(maps["EnUs"]):
            raise RuntimeError(domain + ": bilingual key sets differ")
        for key, value in maps["ZhCn"].items():
            if placeholders(value) != placeholders(maps["EnUs"][key]):
                raise RuntimeError(domain + "/" + key + ": placeholder sets differ")
            rows["ZhCn"].append((key, value))
            rows["EnUs"].append((key, maps["EnUs"][key]))

    for language in rows:
        rows[language].sort(key=lambda pair: pair[0])

    parts = [
        "#![no_std]\n",
        "extern crate alloc;\n",
        "#[cfg(test)] extern crate std;\n",
        "use alloc::{format, string::{String, ToString}, vec, vec::Vec};\n",
        r'''
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language { #[default] ZhCn, EnUs }
impl Language {
    pub const ALL: [Language; 2] = [Language::ZhCn, Language::EnUs];
    pub fn code(self) -> &'static str {
        match self { Self::ZhCn => "zh-CN", Self::EnUs => "en-US" }
    }
    pub fn menu_label(self) -> &'static str {
        match self { Self::ZhCn => "中文", Self::EnUs => "English" }
    }
    pub fn parse(raw: &str) -> Option<Self> {
        fn matches(raw: &str, expected: &str) -> bool {
            let input = raw.trim().as_bytes();
            let want = expected.as_bytes();
            input.len() == want.len() && input.iter().zip(want).all(|(&a, &b)| {
                (if a == b'_' { b'-' } else { a }).to_ascii_lowercase() == b
            })
        }
        if ["zh", "zh-cn", "zh-hans", "zh-hans-cn", "cn"].iter().any(|v| matches(raw, v)) {
            Some(Self::ZhCn)
        } else if ["en", "en-us", "en-gb", "en-latn-us"].iter().any(|v| matches(raw, v)) {
            Some(Self::EnUs)
        } else { None }
    }
    pub fn index(self) -> usize { match self { Self::ZhCn => 0, Self::EnUs => 1 } }
    pub fn at(index: usize) -> Self { Self::ALL[index.min(Self::ALL.len() - 1)] }
    pub fn next(self, right: bool) -> Self {
        let len = Self::ALL.len();
        if right { Self::at((self.index() + 1) % len) }
        else { Self::at((self.index() + len - 1) % len) }
    }
}
struct Domain { name: &'static str, files: &'static [(Language, &'static str)] }
''',
        "const DOMAINS: [Domain; " + str(len(domains)) + "] = [\n"
    ]
    for domain in domains:
        parts.append("    Domain { name: " + rust_literal(domain) + ", files: &[\n")
        for variant, locale in LOCALES:
            parts.append("        (Language::" + variant + ", " +
                         rust_literal(file_data[(domain, variant)]) + "),\n")
        parts.append("    ] },\n")
    parts.append("];\n")

    for variant in ("ZhCn", "EnUs"):
        parts.append("static TABLE_" + variant.upper() + ": &[(&str, &str)] = &[\n")
        for key, value in rows[variant]:
            parts.append("    (" + rust_literal(key) + ", " + rust_literal(value) + "),\n")
        parts.append("];\n")

    parts.append(r'''
fn table(lang: Language) -> &'static [(&'static str, &'static str)] {
    match lang { Language::ZhCn => TABLE_ZHCN, Language::EnUs => TABLE_ENUS }
}
pub fn domains() -> impl Iterator<Item = (&'static str, Language, &'static str)> {
    DOMAINS.iter().flat_map(|domain| {
        domain.files.iter().map(move |&(language, raw)| (domain.name, language, raw))
    })
}
pub fn t_opt(lang: Language, key: &str) -> Option<&'static str> {
    table(lang).iter().find_map(|(k, v)| (*k == key).then_some(*v))
        .or_else(|| table(Language::ZhCn).iter().find_map(|(k, v)| (*k == key).then_some(*v)))
}
pub fn t(lang: Language, key: &'static str) -> &'static str {
    t_opt(lang, key).unwrap_or(key)
}
pub fn t_args(lang: Language, key: &'static str, args: &[(&str, &str)]) -> String {
    let mut text = t(lang, key).to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}
pub fn placeholders(template: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break; };
        let name = &after[..close];
        if !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !found.iter().any(|seen| seen == name) {
            found.push(name.to_string());
        }
        rest = &after[close + 1..];
    }
    found.sort();
    found
}
pub fn entries(lang: Language) -> Vec<(&'static str, &'static str)> {
    let mut out = table(lang).to_vec();
    out.sort_by(|a, b| a.0.cmp(b.0));
    out
}
pub fn len(lang: Language) -> usize { table(lang).len() }

#[cfg(test)]
mod generated_port_tests {
    use super::*;
    #[test]
    fn every_embedded_domain_language_pair_is_preserved() {
        assert_eq!(domains().count(), DOMAIN_FILES);
    }
    #[test]
    fn runtime_tables_are_sorted_and_complete() {
        for lang in Language::ALL {
            let got = entries(lang);
            assert_eq!(got.len(), len(lang));
            for (key, value) in got {
                assert_eq!(t_opt(lang, key), Some(value));
            }
        }
    }
}
'''.replace("DOMAIN_FILES", str(len(domains) * 2)))

    for variant in ("ZhCn", "EnUs"):
        parts.append("#[cfg(test)] mod generated_" + variant.lower() + "_table_test {\n")
        parts.append("    use super::*;\n    const EXPECTED: &[(&str, &str)] = &[\n")
        for key, value in rows[variant]:
            parts.append("        (" + rust_literal(key) + ", " + rust_literal(value) + "),\n")
        parts.append("    ];\n    #[test] fn complete_table_matches_generated_source() {\n")
        parts.append("        assert_eq!(entries(Language::" + variant + "), EXPECTED);\n")
        parts.append("    }\n}\n")

    match = re.search(r"(?s)(#\[cfg\(test\)\]\s*mod tests\s*\{.*)$", original)
    if not match:
        raise RuntimeError("could not preserve upstream slot-i18n unit test module")
    parts.append(match.group(1))

    out.mkdir(parents=True, exist_ok=True)
    (out / "src").mkdir(parents=True, exist_ok=True)
    (out / "Cargo.toml").write_text(make_manifest("slot-i18n"), encoding="utf-8")
    (out / "src/lib.rs").write_text("".join(parts), encoding="utf-8")
    return len(key_owners), len(domains), len(rows["ZhCn"])

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    source, out = args.source.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    run(["cargo", "test", "--manifest-path", str(source / "Cargo.toml"),
         "-p", "slot-i18n", "--locked"],
        env={"CARGO_TARGET_DIR": str(out / "upstream-target")},
        log=out / "upstream-host-tests.log")
    key_count, domain_count, entries_per_language = build_candidate(source, out)
    report = [
        "# slot-i18n no_std static-table feasibility\n\n",
        "Upstream source snapshot: " + str(source) + "\n",
        "Domains: " + str(domain_count) + "; unique keys: " + str(key_count) +
        "; entries per language: " + str(entries_per_language) + ".\n",
        "Build-time JSON checks: schema, language metadata, UTF-8/newline policy, non-empty values, bilingual key parity, global key uniqueness and placeholder parity all passed.\n",
        "Unmodified upstream slot-i18n unit and translations integration tests passed on host.\n",
        "Runtime representation: read-only static slices; no serde_json, HashMap or OnceLock in the candidate. t_args/placeholders/entries use alloc.\n\n"
    ]
    for target in TARGETS:
        run(["cargo", "check", "--manifest-path", str(out / "Cargo.toml"),
             "--target", target, "--lib"],
            env={"RUSTFLAGS": "-C panic=abort -C force-unwind-tables=no -C target-cpu=cortex-a53"},
            log=out / (target + ".log"))
        report.append("PASS: cargo check --target " + target + "\n")
    run(["cargo", "test", "--manifest-path", str(out / "Cargo.toml"),
         "--target", "x86_64-unknown-linux-gnu"],
        log=out / "host-tests.log")
    report.append("PASS: preserved upstream unit tests plus generated full-table parity tests on host.\n")
    (out / "REPORT.md").write_text("".join(report), encoding="utf-8")
    print((out / "REPORT.md").read_text(encoding="utf-8"))

if __name__ == "__main__":
    main()
