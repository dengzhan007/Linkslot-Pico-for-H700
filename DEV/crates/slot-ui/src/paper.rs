//! The document layer: one cream paper stock shared by the sync bench,
//! transfer docket and other printed surfaces.  The red stamp is deliberately
//! separate from the brass UI accent.

pub const PAPER: [u8; 4] = [0xe8, 0xe2, 0xcd, 0xff];
pub const PAPER_BORDER: [u8; 4] = [0xc9, 0xc3, 0xb4, 0xff];
pub const PAPER_EDGE: [u8; 4] = [0xb5, 0xae, 0x9c, 0xff];
pub const PAPER_INK: [u8; 3] = [0x2e, 0x29, 0x22];
pub const PAPER_DIM: [u8; 3] = [0x62, 0x5c, 0x4f];
pub const PAPER_FAINT: [u8; 3] = [0x66, 0x5f, 0x51];
pub const STAMP: [u8; 3] = [0xa8, 0x34, 0x2f];
