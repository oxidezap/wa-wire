//! What an adapter says about itself, and whether it keeps its word.
//!
//! Declaring capabilities is only half of it. A declaration nobody checks
//! drifts from the code the first time an engine changes underneath. So the
//! same value that carries the claims also checks stanzas against them, which
//! turns "this adapter is zero-copy" from a comment into a test.

use core::fmt;

use wa_wire_contract::{Capability, CapabilitySet, ContractVersion, Provenance, UnmetCapabilities};

use crate::stanza::RawStanza;

/// An adapter's identity, capabilities and spec provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterInfo<'a> {
    /// Stable identifier, e.g. `whatsapp-rust`.
    pub id: &'a str,
    /// The adapter's own version.
    pub version: &'a str,
    /// Which engine version it was built against.
    pub engine_version: &'a str,
    /// The contract version it speaks.
    pub contract_version: ContractVersion,
    /// What it can do.
    pub capabilities: CapabilitySet,
    /// Which `whatspec` build its L1 derivation came from, when it has one.
    ///
    /// An adapter that only emits L0 has no derivation, so it has nothing to
    /// report — which is not the same as failing to record it.
    pub provenance: Option<Provenance<'a>>,
}

impl<'a> AdapterInfo<'a> {
    /// Declare an adapter at the current contract version.
    #[must_use]
    pub const fn new(
        id: &'a str,
        version: &'a str,
        engine_version: &'a str,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            id,
            version,
            engine_version,
            contract_version: ContractVersion::CURRENT,
            capabilities,
            provenance: None,
        }
    }

    /// Record which `whatspec` build this adapter derives L1 from.
    #[must_use]
    pub const fn with_provenance(mut self, provenance: Provenance<'a>) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Whether `capability` is declared.
    #[must_use]
    pub const fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check this adapter against what a consumer needs, before anything runs.
    ///
    /// The gate the whole capability matrix exists for. Without it a consumer
    /// discovers that its engine never emits plaintext, or re-encodes the
    /// frames it was going to replay, as *missing traffic* — at which point the
    /// evidence of what went wrong is the thing that is absent. Naming the
    /// requirement up front turns that into a refusal to start.
    ///
    /// ```
    /// use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet};
    ///
    /// # fn example(adapter: AdapterInfo<'_>) -> Result<(), Box<dyn core::error::Error>> {
    /// // "I replay traffic, so I need the original bytes and the plaintexts."
    /// adapter.require(
    ///     CapabilitySet::NONE
    ///         .with(Capability::ZeroCopyFrame)
    ///         .with(Capability::L0Plaintext),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// [`UnmetCapabilities`] naming every requirement this adapter lacks — all
    /// of them, so a caller fixes its setup once rather than one round trip per
    /// missing capability.
    pub fn require(&self, needed: CapabilitySet) -> Result<(), UnmetCapabilities> {
        self.capabilities.check_supports(needed)
    }

    /// Check a stanza against what this adapter claims.
    ///
    /// Called from an adapter's own tests, and from the conformance runner, so
    /// a capability that stops being true fails loudly instead of quietly.
    pub fn verify(&self, stanza: &RawStanza<'_>) -> Result<(), Violation> {
        if self.has(Capability::ZeroCopyFrame) && !stanza.is_verbatim() {
            return Err(Violation::ReEncodedDespiteZeroCopy);
        }
        if !self.has(Capability::L0Plaintext) && !stanza.plaintexts.is_empty() {
            return Err(Violation::PlaintextsWithoutCapability {
                count: stanza.plaintexts.len(),
            });
        }
        // `L0OutboundObserved`, not `L0Outbound`. This check has always been
        // documented as "does not claim to observe the outbound path" and has
        // always tested the capability for *sending* — which every adapter with
        // a `Sender` has, and which says nothing about whether the engine
        // reports what left. An envelope travelling outbound is an observation.
        if !self.has(Capability::L0OutboundObserved)
            && matches!(stanza.direction, wa_wire_contract::Direction::Outbound)
        {
            return Err(Violation::OutboundWithoutCapability);
        }
        // A cause is a claim about a failure, and only an adapter that reports
        // failures can make one. Without the capability an entry says
        // `Unobserved` — no payload arrived, cause unknown — which is what
        // every adapter has said so far.
        if !self.has(Capability::L0PlaintextCause)
            && stanza
                .plaintexts
                .iter()
                .any(|entry| entry.status.claims_a_cause())
        {
            return Err(Violation::CauseWithoutCapability);
        }
        Ok(())
    }
}

impl fmt::Display for AdapterInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} (engine {}, contract {}) [{}]",
            self.id, self.version, self.engine_version, self.contract_version, self.capabilities
        )
    }
}

/// A stanza contradicted the adapter's declared capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Violation {
    /// The adapter claims zero-copy frames but delivered a re-encoded one.
    ReEncodedDespiteZeroCopy,
    /// Plaintexts arrived from an adapter that does not claim to produce them.
    PlaintextsWithoutCapability {
        /// How many arrived.
        count: usize,
    },
    /// A plaintext named a cause from an adapter that does not claim to know
    /// one.
    ///
    /// `DecryptFailed` and `Unsupported` say why a payload is missing.
    /// `Unobserved` says only that it is. An adapter reporting the first two
    /// without declaring it can tell them apart is guessing, and a gate reading
    /// the recording cannot know that.
    CauseWithoutCapability,
    /// An outbound stanza arrived from an adapter that does not claim to
    /// observe the outbound path.
    OutboundWithoutCapability,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReEncodedDespiteZeroCopy => {
                f.write_str("adapter declares l0.zero-copy-frame but delivered a re-encoded frame")
            }
            Self::PlaintextsWithoutCapability { count } => write!(
                f,
                "adapter delivered {count} plaintext(s) without declaring l0.plaintext"
            ),
            Self::OutboundWithoutCapability => f.write_str(
                "adapter delivered an outbound stanza without declaring l0.outbound.observed",
            ),
            Self::CauseWithoutCapability => f.write_str(
                "adapter named a cause for a missing plaintext without declaring \
                 l0.plaintext.cause",
            ),
        }
    }
}

impl core::error::Error for Violation {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    use crate::path::NodePathBuf;
    use crate::stanza::Plaintext;

    fn caps(list: &[Capability]) -> CapabilitySet {
        CapabilitySet::from_iter_of(list.iter().copied())
    }

    fn info(list: &[Capability]) -> AdapterInfo<'static> {
        AdapterInfo::new("test-engine", "0.1.0", "9.9.9", caps(list))
    }

    #[test]
    fn an_adapter_reports_what_it_was_declared_with() {
        let adapter = info(&[Capability::L0InboundTap, Capability::ZeroCopyFrame]);
        assert_eq!(adapter.id, "test-engine");
        assert_eq!(adapter.version, "0.1.0");
        assert_eq!(adapter.engine_version, "9.9.9");
        assert_eq!(adapter.contract_version, ContractVersion::CURRENT);
        assert!(adapter.has(Capability::L0InboundTap));
        assert!(adapter.has(Capability::ZeroCopyFrame));
        assert!(!adapter.has(Capability::Takeover));
        assert_eq!(adapter.provenance, None);
    }

    #[test]
    fn provenance_is_absent_until_recorded() {
        let derivation = Provenance::new("2.3000.1", "sha256:abc", "0.1.0");
        let adapter = info(&[Capability::L0InboundTap]).with_provenance(derivation);
        assert_eq!(adapter.provenance, Some(derivation));
    }

    #[test]
    fn a_zero_copy_claim_is_checked_against_every_stanza() {
        let adapter = info(&[Capability::L0InboundTap, Capability::ZeroCopyFrame]);
        assert_eq!(adapter.verify(&RawStanza::inbound(b"f")), Ok(()));
        assert_eq!(
            adapter.verify(&RawStanza::inbound(b"f").re_encoded()),
            Err(Violation::ReEncodedDespiteZeroCopy)
        );
    }

    #[test]
    fn an_adapter_without_the_zero_copy_claim_may_re_encode() {
        let adapter = info(&[Capability::L0InboundTap]);
        assert_eq!(
            adapter.verify(&RawStanza::inbound(b"f").re_encoded()),
            Ok(())
        );
        assert_eq!(adapter.verify(&RawStanza::inbound(b"f")), Ok(()));
    }

    #[test]
    fn plaintexts_require_the_capability_that_promises_them() {
        let mut path = NodePathBuf::new();
        path.push(0).unwrap();
        let plaintexts = [Plaintext::ok(path.as_path(), b"body")];
        let stanza = RawStanza::inbound(b"f").with_plaintexts(&plaintexts);

        let silent = info(&[Capability::L0InboundTap]);
        assert_eq!(
            silent.verify(&stanza),
            Err(Violation::PlaintextsWithoutCapability { count: 1 })
        );
        assert_eq!(silent.verify(&RawStanza::inbound(b"f")), Ok(()));

        let full = info(&[Capability::L0InboundTap, Capability::L0Plaintext]);
        assert_eq!(full.verify(&stanza), Ok(()));
    }

    #[test]
    fn outbound_stanzas_require_the_capability_that_observes_them() {
        let inbound_only = info(&[Capability::L0InboundTap]);
        assert_eq!(
            inbound_only.verify(&RawStanza::outbound(b"f")),
            Err(Violation::OutboundWithoutCapability)
        );
        assert_eq!(inbound_only.verify(&RawStanza::inbound(b"f")), Ok(()));

        let both = info(&[Capability::L0InboundTap, Capability::L0OutboundObserved]);
        assert_eq!(both.verify(&RawStanza::outbound(b"f")), Ok(()));
    }

    /// Being able to send is not being able to see what was sent.
    ///
    /// Every adapter with a `Sender` declares `l0.outbound`, and until one
    /// engine added an outbound observation point none of them could report a
    /// single stanza that left. An adapter that could send and emitted an
    /// outbound envelope would have been inventing it.
    #[test]
    fn being_able_to_send_does_not_admit_an_outbound_envelope() {
        let sender = info(&[Capability::L0InboundTap, Capability::L0Outbound]);
        assert_eq!(
            sender.verify(&RawStanza::outbound(b"f")),
            Err(Violation::OutboundWithoutCapability)
        );
    }

    #[test]
    fn an_adapter_declaring_nothing_still_accepts_a_bare_inbound_stanza() {
        let bare = info(&[]);
        assert_eq!(bare.verify(&RawStanza::inbound(b"f")), Ok(()));
    }

    #[test]
    fn display_names_the_adapter_and_its_capabilities() {
        let adapter = info(&[Capability::L0InboundTap, Capability::ZeroCopyFrame]);
        let text = adapter.to_string();
        assert!(text.contains("test-engine"), "{text}");
        assert!(text.contains("0.1.0"), "{text}");
        assert!(text.contains("9.9.9"), "{text}");
        assert!(text.contains("v1"), "{text}");
        assert!(text.contains("l0.inbound.tap"), "{text}");
        assert!(text.contains("l0.zero-copy-frame"), "{text}");
    }

    #[test]
    fn violations_render_and_are_std_errors() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        for violation in [
            Violation::ReEncodedDespiteZeroCopy,
            Violation::PlaintextsWithoutCapability { count: 3 },
            Violation::OutboundWithoutCapability,
        ] {
            assert!(!violation.to_string().is_empty());
            assert_error(&violation);
        }
        assert!(
            Violation::PlaintextsWithoutCapability { count: 3 }
                .to_string()
                .contains('3')
        );
    }

    #[test]
    fn adapter_info_is_comparable() {
        assert_eq!(info(&[]), info(&[]));
        assert_ne!(info(&[]), info(&[Capability::Takeover]));
        assert!(!alloc::format!("{:?}", info(&[])).is_empty());
    }

    /// A cause is a claim, and only an adapter that can tell causes apart may
    /// make one.
    ///
    /// `Unobserved` says a payload did not arrive. `DecryptFailed` says Signal
    /// refused it. Under the first, a build whose messages stopped decrypting
    /// looks exactly like one whose adapter stopped observing — the failure and
    /// the blind spot are the same absence, and a gate cannot tell them apart.
    ///
    /// Every adapter reports `Unobserved` today, and the format has carried the
    /// other two since it was written. Naming the capability is what lets one
    /// of them start saying more without a reader having to guess whether the
    /// silence was ignorance.
    #[test]
    fn a_cause_needs_the_capability_that_claims_one() {
        let path = NodePathBuf::new();
        let watching = info(&[Capability::L0InboundTap, Capability::L0Plaintext]);

        // What every adapter says today, and always may.
        let unobserved = [Plaintext::unobserved(path.as_path())];
        assert_eq!(
            watching.verify(&RawStanza::inbound(b"f").with_plaintexts(&unobserved)),
            Ok(())
        );

        // A cause, from an adapter that never claimed to know one.
        let blamed = [Plaintext::failed(path.as_path())];
        assert_eq!(
            watching.verify(&RawStanza::inbound(b"f").with_plaintexts(&blamed)),
            Err(Violation::CauseWithoutCapability)
        );

        // And with the claim declared, it stands.
        let knowing = info(&[
            Capability::L0InboundTap,
            Capability::L0Plaintext,
            Capability::L0PlaintextCause,
        ]);
        assert_eq!(
            knowing.verify(&RawStanza::inbound(b"f").with_plaintexts(&blamed)),
            Ok(())
        );
    }
}
