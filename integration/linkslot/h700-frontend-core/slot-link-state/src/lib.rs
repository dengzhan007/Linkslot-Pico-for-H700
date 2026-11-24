#![no_std]

extern crate alloc;

use alloc::vec::Vec;

const MAGIC: &[u8; 4] = b"SLK1";

pub fn is_pair(state: &[u8]) -> bool {
    state.starts_with(MAGIC)
}

fn local_slice(state: &[u8], player: u8) -> Option<&[u8]> {
    let mut rest = state.strip_prefix(MAGIC)?;
    let mut selected = None;
    for index in 0..2 {
        let len = u32::from_le_bytes(rest.get(..4)?.try_into().ok()?) as usize;
        rest = rest.get(4..)?;
        let part = rest.get(..len)?;
        rest = rest.get(len..)?;
        if index == usize::from(player) {
            selected = Some(part);
        }
    }
    if rest.is_empty() {
        selected
    } else {
        None
    }
}

/// Return one player's original GBA save state from a two-player wrapper.
///
/// The wrapper is valid only when it contains exactly two length-prefixed
/// entries and no trailing bytes.
pub fn local(state: &[u8], player: u8) -> Option<Vec<u8>> {
    Some(local_slice(state, player)?.to_vec())
}

/// Build a two-player wrapper with the same GBA state in both slots.
pub fn pair(state: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 * (4 + state.len()));
    out.extend_from_slice(MAGIC);
    for _ in 0..2 {
        out.extend_from_slice(&(state.len() as u32).to_le_bytes());
        out.extend_from_slice(state);
    }
    out
}

fn bios(state: &[u8]) -> Option<u32> {
    let gba = if is_pair(state) {
        local_slice(state, 0)?
    } else {
        state
    };
    if gba.len() < 0x400 {
        return None;
    }
    Some(u32::from_le_bytes(gba.get(4..8)?.try_into().ok()?))
}

/// States can be compared when both contain enough data to identify the BIOS.
/// Short/unknown states deliberately return true, matching Linkslot's
/// conservative compatibility policy.
pub fn same_bios(a: &[u8], b: &[u8]) -> bool {
    match (bios(a), bios(b)) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn a_pair_gives_each_player_the_state_back() {
        let paired = pair(b"gba");
        assert!(is_pair(&paired));
        assert_eq!(local(&paired, 0).as_deref(), Some(&b"gba"[..]));
        assert_eq!(local(&paired, 1).as_deref(), Some(&b"gba"[..]));
    }

    #[test]
    fn each_player_gets_their_own_gba() {
        let mut state = MAGIC.to_vec();
        for gba in [&b"zero"[..], &b"one!"[..]] {
            state.extend_from_slice(&4u32.to_le_bytes());
            state.extend_from_slice(gba);
        }
        assert_eq!(local(&state, 0).as_deref(), Some(&b"zero"[..]));
        assert_eq!(local(&state, 1).as_deref(), Some(&b"one!"[..]));
    }

    #[test]
    fn anything_but_a_whole_link_state_has_no_local_gba() {
        let whole = pair(b"gba");
        assert_eq!(local(b"a one-GBA state", 0), None);
        assert_eq!(local(&whole[..whole.len() - 1], 0), None);
        assert_eq!(local(&[whole.as_slice(), b"x"].concat(), 0), None);
        assert_eq!(local(&whole, 2), None);
        assert_eq!(local(&whole, u8::MAX), None);
    }

    fn gba(bios: u32) -> Vec<u8> {
        let mut state = vec![0u8; 0x400];
        state[4..8].copy_from_slice(&bios.to_le_bytes());
        state
    }

    #[test]
    fn states_on_the_same_bios_match_whether_paired_or_not() {
        assert!(same_bios(&pair(&gba(0xBAAE187F)), &gba(0xBAAE187F)));
        assert!(same_bios(&gba(1), &pair(&gba(1))));
    }

    #[test]
    fn states_on_different_bioses_do_not() {
        assert!(!same_bios(&pair(&gba(0xBAAE187F)), &pair(&gba(0xA6473709))));
    }

    #[test]
    fn a_state_too_short_to_be_a_gba_says_nothing() {
        assert!(same_bios(&0u64.to_le_bytes(), &1u64.to_le_bytes()));
    }

    #[test]
    fn malformed_large_lengths_are_rejected_without_overflow() {
        let malformed = [b'S', b'L', b'K', b'1', 0xff, 0xff, 0xff, 0xff];
        assert_eq!(local(&malformed, 0), None);
        assert!(same_bios(&malformed, &malformed));
    }
}
