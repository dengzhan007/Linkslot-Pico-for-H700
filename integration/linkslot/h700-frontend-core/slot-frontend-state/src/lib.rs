#![no_std]

/// Tracks which language-specific UI resource generation is baked and which
/// generation is currently being built.
///
/// The type is generic over the language identifier so that this state machine
/// has no dependency on the host translation loader or its std/serde backend.
/// The caller owns all workers, rendering resources and result delivery.
pub struct FacesLang<L> {
    baked: Option<L>,
    building: Option<(L, u64)>,
    generation: u64,
}

impl<L> Default for FacesLang<L> {
    fn default() -> Self {
        Self {
            baked: None,
            building: None,
            generation: 0,
        }
    }
}

impl<L: Copy + PartialEq> FacesLang<L> {
    /// The language identifier of the resources currently visible.
    pub fn baked(&self) -> Option<L> {
        self.baked
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn is_building(&self) -> bool {
        self.building.is_some()
    }

    /// Request a build if no build is in flight and the current result differs.
    ///
    /// While a build is pending, repeated language changes are coalesced. When
    /// the worker completes, the caller asks again using the latest language.
    pub fn want(&mut self, lang: L) -> Option<u64> {
        if self.building.is_some() || self.baked == Some(lang) {
            return None;
        }
        Some(self.begin(lang))
    }

    /// Record a build that is about to start, unconditionally.
    pub fn begin(&mut self, lang: L) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.building = Some((lang, self.generation));
        self.generation
    }

    /// Clear an in-flight build after its worker exits without returning.
    pub fn abandon(&mut self) {
        self.building = None;
    }

    /// Mark a worker result complete. Stale results never replace current state.
    pub fn finish(&mut self, generation: u64) -> bool {
        let Some((lang, wanted)) = self.building else {
            return false;
        };
        if wanted != generation {
            return false;
        }
        self.building = None;
        self.baked = Some(lang);
        true
    }

    /// Record a synchronous build completed by the caller.
    pub fn baked_now(&mut self, lang: L) {
        self.building = None;
        self.baked = Some(lang);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Language {
        ZhCn,
        EnUs,
    }

    #[test]
    fn the_first_build_is_requested_and_lands() {
        let mut state = FacesLang::default();
        assert_eq!(state.baked(), None);
        let generation = state.want(Language::EnUs).expect("the first build");
        assert!(state.is_building());
        assert!(state.finish(generation));
        assert_eq!(state.baked(), Some(Language::EnUs));
        assert!(!state.is_building());
    }

    #[test]
    fn a_matching_language_never_rebuilds() {
        let mut state = FacesLang::default();
        state.baked_now(Language::ZhCn);
        assert_eq!(state.want(Language::ZhCn), None);
        assert_eq!(state.generation(), 0);
    }

    #[test]
    fn rapid_language_changes_coalesce_into_one_follow_up_build() {
        let mut state = FacesLang::default();
        let first = state.want(Language::EnUs).expect("the first build");
        for _ in 0..20 {
            assert_eq!(state.want(Language::ZhCn), None);
            assert_eq!(state.want(Language::EnUs), None);
        }
        assert!(state.finish(first));
        assert_eq!(state.baked(), Some(Language::EnUs));
        let second = state.want(Language::ZhCn).expect("the late build");
        assert_ne!(second, first);
        assert!(state.finish(second));
        assert_eq!(state.baked(), Some(Language::ZhCn));
        assert_eq!(state.generation(), 2);
    }

    #[test]
    fn an_old_generation_cannot_finish_a_newer_build() {
        let mut state = FacesLang::default();
        let first = state.begin(Language::EnUs);
        let second = state.begin(Language::ZhCn);
        assert!(!state.finish(first));
        assert!(state.is_building());
        assert_eq!(state.baked(), None);
        assert!(state.finish(second));
        assert_eq!(state.baked(), Some(Language::ZhCn));
    }

    #[test]
    fn abandoning_a_dead_worker_allows_a_retry() {
        let mut state = FacesLang::default();
        let _ = state.want(Language::EnUs).expect("the first build");
        state.abandon();
        assert!(!state.is_building());
        assert!(state.want(Language::EnUs).is_some());
    }

    #[test]
    fn synchronous_build_replaces_any_pending_request() {
        let mut state = FacesLang::default();
        let _ = state.begin(Language::EnUs);
        state.baked_now(Language::ZhCn);
        assert!(!state.is_building());
        assert_eq!(state.baked(), Some(Language::ZhCn));
    }
}
