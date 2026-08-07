//! Which `whatspec` build an L1 derivation came from.
//!
//! This is not bookkeeping. When two engines disagree on L1 output, the first
//! question is whether they were generated from the same spec — and without
//! provenance that question has no answer, which makes every conformance
//! failure ambiguous.
//!
//! Provenance is the second version axis. A WhatsApp change moves this and
//! leaves [`ContractVersion`](crate::version::ContractVersion) alone.

use core::fmt;

/// Identifies the `whatspec` manifest an L1 derivation was generated from.
///
/// Borrowed rather than owned so a generated module can hold `&'static str`
/// constants with no allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Provenance<'a> {
    /// The WhatsApp Web version the spec was extracted from.
    pub whatsapp_version: &'a str,
    /// The `whatspec` manifest hash, which pins every extracted domain.
    pub manifest_hash: &'a str,
    /// The version of the generator that produced the derivation.
    pub generator_version: &'a str,
}

impl<'a> Provenance<'a> {
    /// Record a provenance triple.
    #[must_use]
    pub const fn new(
        whatsapp_version: &'a str,
        manifest_hash: &'a str,
        generator_version: &'a str,
    ) -> Self {
        Self {
            whatsapp_version,
            manifest_hash,
            generator_version,
        }
    }

    /// Whether two builds derive from the same spec.
    ///
    /// The manifest hash alone decides: it pins every extracted domain, so two
    /// builds agreeing on it agree on the spec regardless of how their version
    /// strings were formatted.
    #[must_use]
    pub fn matches(&self, other: &Provenance<'_>) -> bool {
        self.manifest_hash == other.manifest_hash
    }

    /// Compare against a peer's provenance.
    ///
    /// A mismatch is a **warning**, not an error: adapter and host generated
    /// from different WhatsApp versions is expected mid-rollout, and detecting
    /// it is exactly what the conformance suite is for. Refusing to start would
    /// turn a routine condition into an outage.
    #[must_use]
    pub fn compare(&self, peer: &Provenance<'a>) -> ProvenanceCheck<'a> {
        if self.matches(peer) {
            ProvenanceCheck::Match
        } else {
            ProvenanceCheck::Divergent { peer: *peer }
        }
    }

    /// Whether any field is blank, which means the build did not record it.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.whatsapp_version.is_empty()
            && !self.manifest_hash.is_empty()
            && !self.generator_version.is_empty()
    }
}

impl fmt::Display for Provenance<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "whatsapp {} / manifest {} / generator {}",
            self.whatsapp_version, self.manifest_hash, self.generator_version
        )
    }
}

/// Outcome of comparing two provenances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceCheck<'a> {
    /// Both sides derive from the same spec.
    Match,
    /// The sides derive from different specs. Not fatal — surfaced so the
    /// operator and the conformance suite can see it.
    Divergent {
        /// What the peer reported.
        peer: Provenance<'a>,
    },
}

impl ProvenanceCheck<'_> {
    /// Whether both sides agree.
    #[must_use]
    pub const fn is_match(&self) -> bool {
        matches!(self, Self::Match)
    }

    /// Whether the operator should be warned.
    #[must_use]
    pub const fn is_divergent(&self) -> bool {
        matches!(self, Self::Divergent { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    const A: Provenance<'static> = Provenance::new("2.3000.1234567890", "sha256:aaaa", "0.1.0");
    const B: Provenance<'static> = Provenance::new("2.3000.9999999999", "sha256:bbbb", "0.1.0");

    #[test]
    fn fields_are_recorded_verbatim() {
        assert_eq!(A.whatsapp_version, "2.3000.1234567890");
        assert_eq!(A.manifest_hash, "sha256:aaaa");
        assert_eq!(A.generator_version, "0.1.0");
    }

    #[test]
    fn the_manifest_hash_alone_decides_a_match() {
        assert!(A.matches(&A));
        assert!(!A.matches(&B));

        // Same spec, differently formatted version strings, newer generator.
        let same_spec = Provenance::new("2.3000.1234567890-beta", "sha256:aaaa", "0.9.0");
        assert!(A.matches(&same_spec));

        // Same WhatsApp version string but a different manifest is divergent:
        // the hash is what pins the extracted domains.
        let restamped = Provenance::new("2.3000.1234567890", "sha256:cccc", "0.1.0");
        assert!(!A.matches(&restamped));
    }

    #[test]
    fn compare_reports_a_match() {
        let check = A.compare(&A);
        assert_eq!(check, ProvenanceCheck::Match);
        assert!(check.is_match());
        assert!(!check.is_divergent());
    }

    #[test]
    fn compare_reports_divergence_without_failing() {
        let check = A.compare(&B);
        assert_eq!(check, ProvenanceCheck::Divergent { peer: B });
        assert!(check.is_divergent());
        assert!(!check.is_match());
    }

    #[test]
    fn completeness_requires_every_field() {
        assert!(A.is_complete());
        assert!(!Provenance::new("", "h", "g").is_complete());
        assert!(!Provenance::new("v", "", "g").is_complete());
        assert!(!Provenance::new("v", "h", "").is_complete());
        assert!(!Provenance::new("", "", "").is_complete());
    }

    #[test]
    fn display_names_all_three_fields() {
        let text = A.to_string();
        assert!(text.contains("2.3000.1234567890"), "{text}");
        assert!(text.contains("sha256:aaaa"), "{text}");
        assert!(text.contains("0.1.0"), "{text}");
    }

    #[test]
    fn provenance_is_comparable() {
        assert_eq!(A, A);
        assert_ne!(A, B);
        assert!(!alloc::format!("{A:?}").is_empty());
    }
}
