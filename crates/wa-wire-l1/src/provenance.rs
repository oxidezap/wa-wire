//! Which whatspec build a derivation came from.
//!
//! Not bookkeeping. When two engines disagree on L1 output, the first question
//! is whether they derived from the same spec — and without this that question
//! has no answer, so every conformance failure is ambiguous.

use core::fmt;

/// Identifies the whatspec build an L1 derivation was generated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Provenance<'a> {
    /// The WhatsApp Web version the spec was extracted from.
    pub whatsapp_version: &'a str,
    /// The IR schema version.
    pub schema_version: &'a str,
    /// The whatspec generator that produced it.
    pub generator_version: &'a str,
    /// SHA-256 of the `incoming` domain this build consumed.
    pub incoming_digest: &'a str,
    /// SHA-256 of the `WAProto.proto` the payload derivation consumed.
    ///
    /// A second digest because the two halves of L1 come from two domains and
    /// can move apart: WhatsApp can renumber a protobuf field without touching
    /// how a stanza parses, and a recording has to say which build of each it
    /// derived from.
    pub proto_digest: &'a str,
}

impl Provenance<'_> {
    /// Whether two builds derived from the same spec.
    ///
    /// The digest alone decides: it covers the whole domain, so two builds
    /// agreeing on it agree regardless of how their version strings read.
    #[must_use]
    pub fn matches(&self, other: &Provenance<'_>) -> bool {
        self.incoming_digest == other.incoming_digest && self.proto_digest == other.proto_digest
    }

    /// Whether every field was recorded.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.whatsapp_version.is_empty()
            && !self.schema_version.is_empty()
            && !self.generator_version.is_empty()
            && !self.incoming_digest.is_empty()
            && !self.proto_digest.is_empty()
    }

    /// Convert to the contract's provenance, which carries the three fields
    /// negotiation compares.
    #[must_use]
    pub const fn as_contract(&self) -> (&str, &str, &str) {
        (
            self.whatsapp_version,
            self.incoming_digest,
            self.generator_version,
        )
    }
}

impl fmt::Display for Provenance<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "whatsapp {} / schema {} / generator {} / incoming {}",
            self.whatsapp_version,
            self.schema_version,
            self.generator_version,
            self.incoming_digest
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    const A: Provenance<'static> = Provenance {
        whatsapp_version: "2.3000.1",
        schema_version: "2.0.0",
        generator_version: "0.1.0",
        incoming_digest: "sha256:aaaa",
        proto_digest: "sha256:aaaa",
    };

    #[test]
    fn the_digest_alone_decides_a_match() {
        assert!(A.matches(&A));

        let same_spec = Provenance {
            whatsapp_version: "2.3000.1-beta",
            generator_version: "0.9.0",
            ..A
        };
        assert!(A.matches(&same_spec), "version strings do not decide");

        let other = Provenance {
            incoming_digest: "sha256:bbbb",
            proto_digest: "sha256:bbbb",
            ..A
        };
        assert!(!A.matches(&other));
    }

    #[test]
    fn completeness_requires_every_field() {
        assert!(A.is_complete());
        for blanked in [
            Provenance {
                whatsapp_version: "",
                ..A
            },
            Provenance {
                schema_version: "",
                ..A
            },
            Provenance {
                generator_version: "",
                ..A
            },
            Provenance {
                incoming_digest: "",
                ..A
            },
            Provenance {
                proto_digest: "",
                ..A
            },
        ] {
            assert!(!blanked.is_complete());
        }
    }

    #[test]
    fn the_two_domains_are_compared_independently() {
        // WhatsApp can renumber a protobuf field without touching how a stanza
        // parses, so two builds agreeing on one domain and not the other are
        // not the same spec.
        assert!(!A.matches(&Provenance {
            proto_digest: "sha256:different",
            ..A
        }));
        assert!(!A.matches(&Provenance {
            incoming_digest: "sha256:different",
            ..A
        }));
        assert!(A.matches(&A));
    }

    #[test]
    fn the_contract_triple_is_the_digest_not_the_schema_version() {
        // Negotiation compares the hash that pins the domain, so that is what
        // travels as the manifest hash.
        assert_eq!(A.as_contract(), ("2.3000.1", "sha256:aaaa", "0.1.0"));
    }

    #[test]
    fn display_names_every_field() {
        let text = A.to_string();
        for fragment in ["2.3000.1", "2.0.0", "0.1.0", "sha256:aaaa"] {
            assert!(text.contains(fragment), "{text:?} lacks {fragment}");
        }
    }

    #[test]
    fn provenance_is_comparable() {
        assert_eq!(A, A);
        assert_ne!(
            A,
            Provenance {
                incoming_digest: "x",
                proto_digest: "x",
                ..A
            }
        );
        assert!(!alloc::format!("{A:?}").is_empty());
    }
}
