//! Generate the embedded mod-docs page from the tracked Markdown sources.
//!
//! The device page and the "export docs" archive used to be maintained as
//! separate HTML and Markdown copies.  The Markdown under
//! `src/web/doc_page/content/` is the only source now: this build step renders
//! the page shell and the export includes the same files.  A deliberately
//! small Markdown subset is implemented here so the build has no extra
//! dependency and the generated page stays offline.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct Section {
    lang: String,
    mode: String,
    id: String,
    title: String,
    file: String,
    open: bool,
    pre_id: String,
}

struct TocEntry {
    lang: String,
    label: String,
    href: String,
}

pub fn generate() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let page_dir = manifest_dir.join("src/web/doc_page");
    let content_dir = page_dir.join("content");
    let shell_path = page_dir.join("mod-docs.shell.html");
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("mod-docs.html");

    println!("cargo:rerun-if-changed={}", content_dir.display());
    println!("cargo:rerun-if-changed={}", shell_path.display());
    println!("cargo:rerun-if-changed=build_support/docs.rs");

    let sections = read_sections(&content_dir.join("manifest.tsv"));
    let toc = read_toc(&content_dir.join("toc.tsv"));

    let mut zh_sections = String::new();
    let mut en_sections = String::new();
    let mut zh_quick = String::new();
    let mut en_quick = String::new();

    for section in &sections {
        let file = content_dir.join(&section.lang).join(&section.file);
        println!("cargo:rerun-if-changed={}", file.display());
        let body = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let open = if section.open { " open" } else { "" };
        match section.mode.as_str() {
            "hidden-pre" => {
                let pre = pre_element(&section.pre_id, &body, "hidden");
                if section.lang == "en" {
                    en_quick.push_str(&pre);
                } else {
                    zh_quick.push_str(&pre);
                }
            }
            "pre" | "pre-section" => {
                let pre_class = if section.mode == "pre-section" { "md-code" } else { "" };
                let rendered = format!(
                    "<details class=\"doc-section\" id=\"{}\"{}><summary>{}</summary>{}</details>\n",
                    escape_attr(&section.id),
                    open,
                    escape_html(&section.title),
                    pre_element(&section.pre_id, &body, pre_class)
                );
                if section.lang == "en" {
                    en_sections.push_str(&rendered);
                } else {
                    zh_sections.push_str(&rendered);
                }
            }
            _ => {
                let body = strip_first_h1(&body);
                let mut rendered = render_markdown(&body);
                if section.id == "doc-api" {
                    inject_alias(&mut rendered, "doc-layout", "<h2 id=\"1-");
                    inject_alias(&mut rendered, "doc-lua", "<h2 id=\"6-");
                    inject_alias(&mut rendered, "doc-theme", "<h2 id=\"9-");
                    inject_alias(&mut rendered, "doc-limits", "<h2 id=\"10-");
                } else if section.id == "doc-api-en" {
                    inject_alias(&mut rendered, "doc-limits-en", "<h2 id=\"limits-");
                }
                rendered = format!(
                    "<details class=\"doc-section\" id=\"{}\"{}><summary>{}</summary>{}</details>\n",
                    escape_attr(&section.id),
                    open,
                    escape_html(&section.title),
                    rendered
                );
                if section.lang == "en" {
                    en_sections.push_str(&rendered);
                } else {
                    zh_sections.push_str(&rendered);
                }
            }
        }
    }

    let zh_toc = build_toc(&toc, "zh", "mod 文档目录");
    let en_toc = build_toc(&toc, "en", "Mod docs contents");
    let shell = fs::read_to_string(&shell_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", shell_path.display()));
    let page = shell
        .replace("{{ZH_TOC}}", &zh_toc)
        .replace("{{ZH_SECTIONS}}", &zh_sections)
        .replace("{{ZH_QUICK}}", &zh_quick)
        .replace("{{EN_TOC}}", &en_toc)
        .replace("{{EN_SECTIONS}}", &en_sections)
        .replace("{{EN_QUICK}}", &en_quick);
    fs::write(&out_path, page)
        .unwrap_or_else(|error| panic!("cannot write {}: {error}", out_path.display()));
}

fn read_sections(path: &Path) -> Vec<Section> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut sections = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 6 {
            panic!("bad manifest row: {line}");
        }
        sections.push(Section {
            lang: fields[0].to_string(),
            mode: fields[1].to_string(),
            id: fields[2].to_string(),
            title: fields[3].to_string(),
            file: fields[4].to_string(),
            open: fields[5] == "1",
            pre_id: fields.get(6).copied().unwrap_or("").to_string(),
        });
    }
    sections
}

fn read_toc(path: &Path) -> Vec<TocEntry> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            panic!("bad toc row: {line}");
        }
        entries.push(TocEntry {
            lang: fields[0].to_string(),
            label: fields[1].to_string(),
            href: fields[2].to_string(),
        });
    }
    entries
}

fn build_toc(entries: &[TocEntry], lang: &str, label: &str) -> String {
    let mut html = format!(
        "<nav class=\"docs-toc\" aria-label=\"{}\">\n",
        escape_attr(label)
    );
    for entry in entries.iter().filter(|entry| entry.lang == lang) {
        html.push_str(&format!(
            "  <a href=\"{}\">{}</a>\n",
            escape_attr(&entry.href),
            escape_html(&entry.label)
        ));
    }
    html.push_str("</nav>");
    html
}

fn pre_element(id: &str, body: &str, class: &str) -> String {
    let class = if class.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", escape_attr(class))
    };
    format!(
        "<pre id=\"{}\"{}>{}</pre>\n",
        escape_attr(id),
        class,
        escape_html(body)
    )
}

fn inject_alias(html: &mut String, id: &str, needle: &str) {
    if let Some(position) = html.find(needle) {
        html.insert_str(position, &format!("<span id=\"{}\"></span>", escape_attr(id)));
    }
}

/// Each page section already has a `<summary>` title; the source file's first
/// H1 would duplicate it as a second heading.
fn strip_first_h1(markdown: &str) -> String {
    let mut out = String::new();
    let mut seen_content = false;
    let mut skipped = false;
    for line in markdown.replace("\r\n", "\n").lines() {
        let trimmed = line.trim_start();
        if !seen_content && trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            skipped = true;
            seen_content = true;
            continue;
        }
        if !trimmed.is_empty() {
            seen_content = true;
        }
        if skipped && out.is_empty() && line.trim().is_empty() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn render_markdown(markdown: &str) -> String {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut html = String::new();
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if let Some(language) = trimmed.strip_prefix("```") {
            let language = language.trim();
            index += 1;
            let mut code = String::new();
            while index < lines.len() && !lines[index].trim_start().starts_with("```") {
                code.push_str(lines[index]);
                code.push('\n');
                index += 1;
            }
            if index < lines.len() {
                index += 1;
            }
            let class = if language.is_empty() {
                String::new()
            } else {
                format!(" class=\"language-{}\"", escape_attr(language))
            };
            html.push_str(&format!(
                "<pre class=\"md-code\"><code{}>{}</code></pre>\n",
                class,
                escape_html(&code)
            ));
            continue;
        }

        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|ch| *ch == '#').count().min(6);
            let text = trimmed[level..].trim();
            let raw_slug = slug(text);
            let count = ids.entry(raw_slug.clone()).or_insert(0);
            let id = if *count == 0 {
                raw_slug
            } else {
                format!("{}-{}", raw_slug, *count)
            };
            *count += 1;
            html.push_str(&format!(
                "<h{level} id=\"{}\">{}</h{level}>\n",
                escape_attr(&id),
                inline(text)
            ));
            index += 1;
            continue;
        }

        if is_table_start(&lines, index) {
            html.push_str(&render_table(&lines, &mut index));
            continue;
        }

        if is_list_item(trimmed) {
            html.push_str(&render_list(&lines, &mut index));
            continue;
        }

        if trimmed.starts_with('>') {
            let mut quote = String::new();
            while index < lines.len() {
                let current = lines[index].trim();
                if !current.starts_with('>') {
                    break;
                }
                quote.push_str(current.trim_start_matches('>').trim());
                quote.push(' ');
                index += 1;
            }
            html.push_str(&format!("<blockquote>{}</blockquote>\n", inline(quote.trim())));
            continue;
        }

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            html.push_str("<hr>\n");
            index += 1;
            continue;
        }

        let mut paragraph = String::new();
        while index < lines.len() {
            let current = lines[index];
            let current_trimmed = current.trim();
            if current_trimmed.is_empty()
                || current_trimmed.starts_with('#')
                || current_trimmed.starts_with("```")
                || current_trimmed.starts_with('>')
                || is_list_item(current_trimmed)
                || is_table_start(&lines, index)
            {
                break;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(current_trimmed);
            index += 1;
        }
        html.push_str(&format!("<p>{}</p>\n", inline(&paragraph)));
    }
    html
}

fn is_table_start(lines: &[&str], index: usize) -> bool {
    if index + 1 >= lines.len() {
        return false;
    }
    let line = lines[index].trim();
    let next = lines[index + 1].trim();
    line.starts_with('|') && next.starts_with('|') && is_table_separator(next)
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim_matches('|').trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| ch == '-' || ch == ':' || ch == ' ' || ch == '|')
        && trimmed.contains('-')
}

fn render_table(lines: &[&str], index: &mut usize) -> String {
    let header = split_cells(lines[*index]);
    *index += 2; // header + separator
    let mut rows = Vec::new();
    while *index < lines.len() {
        let line = lines[*index].trim();
        if !line.starts_with('|') {
            break;
        }
        rows.push(split_cells(line));
        *index += 1;
    }
    let mut html = String::from("<div class=\"table-wrap\"><table class=\"api-table\"><thead><tr>");
    for cell in &header {
        html.push_str(&format!("<th>{}</th>", inline(cell)));
    }
    html.push_str("</tr></thead><tbody>");
    for row in rows {
        html.push_str("<tr>");
        for cell in &row {
            html.push_str(&format!("<td>{}</td>", inline(cell)));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div>\n");
    html
}

fn split_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || ordered_marker(line).is_some()
}

fn ordered_marker(line: &str) -> Option<usize> {
    let mut digits = 0;
    for ch in line.chars() {
        if ch.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits == 0 || !line[digits..].starts_with(". ") {
        return None;
    }
    Some(digits)
}

fn render_list(lines: &[&str], index: &mut usize) -> String {
    let first = lines[*index].trim();
    let ordered = ordered_marker(first).is_some();
    let tag = if ordered { "ol" } else { "ul" };
    let mut html = format!("<{tag}>\n");
    while *index < lines.len() {
        let line = lines[*index].trim();
        if !is_list_item(line) || ordered_marker(line).is_some() != ordered {
            break;
        }
        let body = if let Some(digits) = ordered_marker(line) {
            &line[digits + 2..]
        } else {
            &line[2..]
        };
        html.push_str(&format!("<li>{}</li>\n", inline(body.trim())));
        *index += 1;
    }
    html.push_str(&format!("</{tag}>\n"));
    html
}

fn inline(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let code_pos = rest.find('`');
        let bold_pos = rest.find("**");
        let link_pos = rest.find('[');
        let marker = [code_pos, bold_pos, link_pos]
            .into_iter()
            .flatten()
            .min();
        let Some(position) = marker else {
            out.push_str(&escape_html(rest));
            break;
        };
        out.push_str(&escape_html(&rest[..position]));
        let tail = &rest[position..];
        if tail.starts_with('`') {
            if let Some(end) = tail[1..].find('`') {
                out.push_str("<code>");
                out.push_str(&escape_html(&tail[1..1 + end]));
                out.push_str("</code>");
                rest = &tail[2 + end..];
                continue;
            }
        } else if tail.starts_with("**") {
            if let Some(end) = tail[2..].find("**") {
                out.push_str("<strong>");
                out.push_str(&inline(&tail[2..2 + end]));
                out.push_str("</strong>");
                rest = &tail[4 + end..];
                continue;
            }
        } else if tail.starts_with('[') {
            if let Some(mid) = tail.find("](") {
                if let Some(end) = tail[mid + 2..].find(')') {
                    let label = &tail[1..mid];
                    let url = &tail[mid + 2..mid + 2 + end];
                    out.push_str(&format!(
                        "<a href=\"{}\">{}</a>",
                        escape_attr(url),
                        inline(label)
                    ));
                    rest = &tail[mid + 3 + end..];
                    continue;
                }
            }
        }
        let first = tail.chars().next().unwrap();
        out.push_str(&escape_html(&first.to_string()));
        rest = &tail[first.len_utf8()..];
    }
    out
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                slug.push(lower);
            }
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if !slug.ends_with('-') && !slug.is_empty() {
                slug.push('-');
            }
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_attr(text: &str) -> String {
    escape_html(text).replace('"', "&quot;")
}
