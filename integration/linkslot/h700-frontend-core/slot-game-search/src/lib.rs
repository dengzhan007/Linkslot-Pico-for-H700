#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// Supplies the pinyin table without coupling the search index to slot-store
/// or any filesystem/desktop implementation.
pub trait PinyinProvider {
    fn first_reading(&self, ch: char) -> Option<String>;
    fn readings(&self, ch: char) -> Vec<String>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidatePage {
    pub items: Vec<String>,
    pub page: usize,
    pub pages: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit<P> {
    pub shelf: usize,
    pub index: usize,
    pub platform: P,
    pub label: String,
}

#[derive(Clone, Debug)]
struct Term {
    text: String,
    pinyin: String,
    initials: String,
    kind: u8,
    count: u32,
}

#[derive(Clone, Debug)]
struct Char {
    text: String,
    readings: Vec<String>,
    count: u32,
}

#[derive(Clone, Debug)]
pub struct SearchEntry<'a, P> {
    pub shelf: usize,
    pub index: usize,
    pub platform: P,
    pub stem: &'a str,
    pub title: &'a str,
}

#[derive(Clone, Debug)]
pub struct GameSearchIndex<P> {
    terms: Vec<Term>,
    chars: Vec<Char>,
    carts: Vec<(usize, usize, P, String, String)>,
}

impl<P> Default for GameSearchIndex<P> {
    fn default() -> Self {
        Self {
            terms: Vec::new(),
            chars: Vec::new(),
            carts: Vec::new(),
        }
    }
}

pub const CANDIDATES_PER_PAGE: usize = 8;

impl<P: Copy> GameSearchIndex<P> {
    pub fn build<'a, I, S>(entries: I, pinyin: &S) -> Self
    where
        I: IntoIterator<Item = SearchEntry<'a, P>>,
        S: PinyinProvider,
    {
        let mut index = Self::default();
        let mut seen_terms: Vec<(String, usize)> = Vec::new();
        let mut seen_chars: Vec<(String, usize)> = Vec::new();
        for entry in entries {
            index.carts.push((
                entry.shelf,
                entry.index,
                entry.platform,
                entry.stem.to_string(),
                entry.title.to_string(),
            ));
            let mut labels = alloc::vec![entry.stem];
            if !entry.title.is_empty() && entry.title != entry.stem {
                labels.push(entry.title);
            }
            for label in labels {
                index.add_term(&mut seen_terms, label, 0, pinyin);
                for word in split_words(label) {
                    index.add_term(&mut seen_terms, word, 1, pinyin);
                }
                for ch in label.chars() {
                    index.add_char(&mut seen_chars, ch, pinyin);
                }
            }
        }
        index
    }

    fn add_term<S: PinyinProvider>(
        &mut self,
        seen: &mut Vec<(String, usize)>,
        text: &str,
        kind: u8,
        pinyin: &S,
    ) {
        let Some((full, initials)) = pinyin_pair(text, pinyin) else {
            return;
        };
        if let Some((_, at)) = seen.iter().find(|(known, _)| known == text) {
            self.terms[*at].count += 1;
            self.terms[*at].kind = self.terms[*at].kind.min(kind);
            return;
        }
        seen.push((text.to_string(), self.terms.len()));
        self.terms.push(Term {
            text: text.to_string(),
            pinyin: full,
            initials,
            kind,
            count: 1,
        });
    }

    fn add_char<S: PinyinProvider>(
        &mut self,
        seen: &mut Vec<(String, usize)>,
        ch: char,
        pinyin: &S,
    ) {
        if !is_cjk(ch) {
            return;
        }
        let readings = pinyin.readings(ch);
        if readings.is_empty() {
            return;
        }
        let text = ch.to_string();
        if let Some((_, at)) = seen.iter().find(|(known, _)| known == &text) {
            self.chars[*at].count += 1;
            return;
        }
        seen.push((text.clone(), self.chars.len()));
        self.chars.push(Char {
            text,
            readings,
            count: 1,
        });
    }

    pub fn candidates(&self, composition: &str, page: usize, per_page: usize) -> CandidatePage {
        let composition = normalise_composition(composition);
        if composition.is_empty() {
            return CandidatePage {
                items: Vec::new(),
                page: 0,
                pages: 1,
            };
        }
        let mut matches: Vec<(u8, u32, usize, String)> = Vec::new();
        for term in &self.terms {
            if term.pinyin.starts_with(&composition) || term.initials.starts_with(&composition) {
                matches.push((term.kind, term.count, term.text.len(), term.text.clone()));
            }
        }
        for ch in &self.chars {
            if ch.readings.iter().any(|r| r.starts_with(&composition)) {
                matches.push((2, ch.count, ch.text.len(), ch.text.clone()));
            }
        }
        matches.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.3.cmp(&b.3))
        });
        let mut items: Vec<String> = Vec::new();
        for (_, _, _, text) in matches {
            if !items.iter().any(|known| known == &text) {
                items.push(text);
            }
        }
        let per_page = per_page.max(1);
        let pages = items.len().div_ceil(per_page).max(1);
        let page = page.min(pages - 1);
        let start = page * per_page;
        let end = (start + per_page).min(items.len());
        CandidatePage {
            items: items[start..end].to_vec(),
            page,
            pages,
        }
    }

    pub fn hits(&self, query: &str) -> Vec<SearchHit<P>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        let mut seen = Vec::new();
        for (shelf, index, platform, label, title) in &self.carts {
            let lower = label.to_lowercase();
            let title = title.to_lowercase();
            if (lower.contains(&query) || title.contains(&query))
                && !seen.contains(&(shelf, index))
            {
                seen.push((shelf, index));
                hits.push(SearchHit {
                    shelf: *shelf,
                    index: *index,
                    platform: *platform,
                    label: label.clone(),
                });
            }
        }
        hits
    }
}

fn pinyin_pair<S: PinyinProvider>(text: &str, pinyin: &S) -> Option<(String, String)> {
    let mut full = String::new();
    let mut initials = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            let lower = ch.to_ascii_lowercase();
            full.push(lower);
            initials.push(lower);
        } else if ch.is_ascii_whitespace() || ch.is_ascii_punctuation() || ch == '　' {
            continue;
        } else if is_cjk(ch) {
            let reading = pinyin.first_reading(ch)?;
            full.push_str(&reading);
            initials.push(reading.chars().next()?);
        } else {
            continue;
        }
    }
    if full.is_empty() {
        None
    } else {
        Some((full, initials))
    }
}

fn split_words(label: &str) -> Vec<&str> {
    label
        .split(|c: char| c.is_whitespace() || is_punct(c))
        .filter(|word| {
            let mut chars = word.chars();
            chars.next().is_some() && chars.next().is_some()
        })
        .collect()
}

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(
            c,
            '、' | '。' | '，' | '：' | '；' | '！' | '？' | '（' | '）' | '【' | '】' | '·'
        )
}

fn is_cjk(ch: char) -> bool {
    (0x3400..0xA000).contains(&(ch as u32))
}

fn normalise_composition(text: &str) -> String {
    text.chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphabetic() {
                Some(ch.to_ascii_lowercase())
            } else if ch == 'ü' || ch == 'v' {
                Some('v')
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    struct TestPinyin;

    impl PinyinProvider for TestPinyin {
        fn first_reading(&self, ch: char) -> Option<String> {
            Some(match ch {
                '宝' => "bao",
                '可' => "ke",
                '梦' => "meng",
                '三' => "san",
                '国' => "guo",
                '演' => "yan",
                '义' => "yi",
                _ => return None,
            }.to_string())
        }

        fn readings(&self, ch: char) -> Vec<String> {
            self.first_reading(ch).into_iter().collect()
        }
    }

    #[test]
    fn full_pinyin_and_initials_find_chinese_titles() {
        let entries = [
            SearchEntry { shelf: 0, index: 0, platform: 0u8, stem: "宝可梦 红宝石", title: "POKEMON RUBY" },
            SearchEntry { shelf: 1, index: 1, platform: 1u8, stem: "三国演义", title: "" },
        ];
        let index = GameSearchIndex::build(entries, &TestPinyin);
        let full = index.candidates("baokemeng", 0, 8);
        assert!(full.items.iter().any(|item| item.contains("宝可梦")));
        let initials = index.candidates("bkm", 0, 8);
        assert!(initials.items.iter().any(|item| item.contains("宝可梦")));
        let sgyy = index.candidates("sgyy", 0, 8);
        assert!(sgyy.items.iter().any(|item| item.contains("三国演义")));
    }

    #[test]
    fn hits_preserve_platform_and_shelf_indices() {
        let entries = [
            SearchEntry { shelf: 2, index: 4, platform: 7u8, stem: "宝可梦 红宝石", title: "POKEMON RUBY" },
            SearchEntry { shelf: 3, index: 1, platform: 2u8, stem: "Metroid Fusion", title: "" },
        ];
        let index = GameSearchIndex::build(entries, &TestPinyin);
        let chinese = index.hits("宝可梦");
        assert_eq!(chinese.len(), 1);
        assert_eq!(chinese[0].platform, 7);
        assert_eq!(chinese[0].shelf, 2);
        assert_eq!(chinese[0].index, 4);
        let english = index.hits("metroid");
        assert_eq!(english.len(), 1);
    }

    #[test]
    fn paging_and_empty_compositions_are_bounded() {
        let entries = [
            SearchEntry { shelf: 0, index: 0, platform: 0u8, stem: "宝可梦", title: "" },
            SearchEntry { shelf: 0, index: 1, platform: 0u8, stem: "宝可梦 红宝石", title: "" },
            SearchEntry { shelf: 0, index: 2, platform: 0u8, stem: "宝可梦 绿宝石", title: "" },
        ];
        let index = GameSearchIndex::build(entries, &TestPinyin);
        let first = index.candidates("bao", 0, 2);
        let second = index.candidates("bao", 1, 2);
        assert_eq!(first.items.len(), 2);
        assert!(!second.items.is_empty());
        assert_eq!(index.candidates("", 9, 8).page, 0);
    }

    #[test]
    fn no_pinyin_dictionary_means_no_ime_candidates() {
        struct Empty;
        impl PinyinProvider for Empty {
            fn first_reading(&self, _: char) -> Option<String> { None }
            fn readings(&self, _: char) -> Vec<String> { Vec::new() }
        }
        let entries = [SearchEntry { shelf: 0, index: 0, platform: 0u8, stem: "中文游戏", title: "" }];
        let index = GameSearchIndex::build(entries, &Empty);
        assert!(index.candidates("zhong", 0, 8).items.is_empty());
    }
}
