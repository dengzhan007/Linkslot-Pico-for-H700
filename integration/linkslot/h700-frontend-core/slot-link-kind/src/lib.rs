#![no_std]

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Cable,
    Wireless,
}

impl LinkKind {
    pub fn other(self) -> LinkKind {
        match self {
            LinkKind::Cable => LinkKind::Wireless,
            LinkKind::Wireless => LinkKind::Cable,
        }
    }
}

const WIRELESS: [&str; 43] = [
    "B2WE", "B3AE", "B4UE", "B4UP", "B85A", "B85P", "BDGE", "BDGP", "BG3E", "BKRJ", "BMGD", "BMGE",
    "BMGF", "BMGI", "BMGJ", "BMGP", "BMGS", "BMGU", "BPED", "BPEE", "BPEF", "BPEI", "BPEJ", "BPES",
    "BPGD", "BPGE", "BPGF", "BPGI", "BPGJ", "BPGS", "BPRD", "BPRE", "BPRF", "BPRI", "BPRJ", "BPRS",
    "BR5E", "BR6E", "BRBE", "BRKE", "BTME", "BTMJ", "BTMP",
];

pub fn link_kind(code: &str, title: &str, clean: bool) -> LinkKind {
    if pokemon(code, title) {
        let retail = clean
            && WIRELESS.contains(&code)
            && ["POKEMON FIRE", "POKEMON LEAF", "POKEMON EMER"].contains(&title);
        return if retail {
            LinkKind::Wireless
        } else {
            LinkKind::Cable
        };
    }
    if WIRELESS.contains(&code) {
        LinkKind::Wireless
    } else {
        LinkKind::Cable
    }
}

pub fn serial_option(chosen: LinkKind, auto: LinkKind, code: &str, title: &str) -> &'static str {
    if chosen == auto {
        return "auto";
    }
    match chosen {
        LinkKind::Wireless => "rfu",
        LinkKind::Cable if pokemon(code, title) => "mul_poke",
        LinkKind::Cable if code.starts_with("AWR") => "mul_aw1",
        LinkKind::Cable if code.starts_with("AW2") => "mul_aw2",
        LinkKind::Cable => "auto",
    }
}

pub fn link_carried(code: &str, title: &str) -> bool {
    WIRELESS.contains(&code)
        || pokemon(code, title)
        || code.starts_with("AWR")
        || code.starts_with("AW2")
}

fn pokemon(code: &str, title: &str) -> bool {
    title.starts_with("POKEMON")
        || ["AXV", "AXP", "BPE", "BPR", "BPG"]
            .iter()
            .any(|p| code.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_kind_toggle_is_bijective() {
        assert_eq!(LinkKind::Cable.other(), LinkKind::Wireless);
        assert_eq!(LinkKind::Wireless.other(), LinkKind::Cable);
        assert_eq!(LinkKind::Cable.other().other(), LinkKind::Cable);
    }

    #[test]
    fn verified_retail_pokemon_uses_wireless_only_for_clean_roms() {
        assert_eq!(link_kind("BPEE", "POKEMON FIRE", true), LinkKind::Wireless);
        assert_eq!(link_kind("BPRE", "POKEMON LEAF", true), LinkKind::Wireless);
        assert_eq!(link_kind("BPEE", "POKEMON FIRE", false), LinkKind::Cable);
        assert_eq!(link_kind("BPRE", "POKEMON SAPP", true), LinkKind::Cable);
    }

    #[test]
    fn wireless_table_entries_are_wireless_for_non_pokemon_titles() {
        assert_eq!(link_kind("B2WE", "OTHER GAME", false), LinkKind::Wireless);
        assert_eq!(link_kind("BTMP", "OTHER GAME", true), LinkKind::Wireless);
    }

    #[test]
    fn pokemon_hacks_and_rom_prefixes_remain_cable() {
        assert_eq!(link_kind("BPEE", "POKEMON FIRE HACK", true), LinkKind::Cable);
        assert_eq!(link_kind("AXVE", "SOME OTHER TITLE", true), LinkKind::Cable);
        assert_eq!(link_kind("BPRX", "SOME OTHER TITLE", true), LinkKind::Cable);
    }

    #[test]
    fn unknown_games_default_to_cable() {
        assert_eq!(link_kind("ABCD", "OTHER GAME", true), LinkKind::Cable);
    }

    #[test]
    fn serial_protocol_selection_matches_core_expectations() {
        assert_eq!(serial_option(LinkKind::Wireless, LinkKind::Cable, "B2WE", "OTHER"), "rfu");
        assert_eq!(serial_option(LinkKind::Cable, LinkKind::Wireless, "BPEE", "POKEMON FIRE"), "mul_poke");
        assert_eq!(serial_option(LinkKind::Cable, LinkKind::Wireless, "AWRX", "OTHER"), "mul_aw1");
        assert_eq!(serial_option(LinkKind::Cable, LinkKind::Wireless, "AW2X", "OTHER"), "mul_aw2");
        assert_eq!(serial_option(LinkKind::Cable, LinkKind::Cable, "ABCD", "OTHER"), "auto");
    }

    #[test]
    fn automatic_selection_is_not_overridden() {
        assert_eq!(serial_option(LinkKind::Wireless, LinkKind::Wireless, "B2WE", "OTHER"), "auto");
        assert_eq!(serial_option(LinkKind::Cable, LinkKind::Cable, "ABCD", "OTHER"), "auto");
    }

    #[test]
    fn link_capability_includes_wireless_pokemon_and_aw_titles() {
        assert!(link_carried("B2WE", "OTHER"));
        assert!(link_carried("AXVE", "SOME TITLE"));
        assert!(link_carried("BPRX", "SOME TITLE"));
        assert!(link_carried("AWRX", "OTHER"));
        assert!(link_carried("AW2X", "OTHER"));
        assert!(!link_carried("ABCD", "OTHER"));
    }
}
