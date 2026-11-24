#[derive(Copy, Clone, Debug)]
pub struct Build {
    pub version: &'static str,
    pub hash: &'static str,
    pub dirty: bool,
    pub date: &'static str,
}

impl Build {
    pub fn current() -> Build {
        Build {
            version: env!("CARGO_PKG_VERSION"),
            hash: env!("SLOT_GIT_HASH"),
            dirty: env!("SLOT_GIT_DIRTY") == "1",
            date: env!("SLOT_BUILD_DATE"),
        }
    }

    pub fn serial(&self) -> String {
        self.hash.to_uppercase()
    }

    pub fn dirty_digit(&self) -> char {
        match self.dirty {
            true => '1',
            false => '0',
        }
    }
}

/// The build's Unix timestamp (UTC), compiled in by `build.rs`.  `0` means the
/// environment did not carry a usable clock at build time.
pub fn build_epoch() -> i64 {
    env!("SLOT_BUILD_EPOCH").parse().unwrap_or(0)
}
