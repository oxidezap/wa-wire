//! What an adapter can and cannot do, declared rather than discovered.
//!
//! No engine implements every capability, and the differences are real:
//! `whatsapp-rust` covers the auth phase but has no takeover; `zapo` has
//! takeover but skips `success`/`failure`; `Baileys` sees every frame but has
//! neither a plugin host nor takeover. Rather than paper over that, an adapter
//! states its set and a consumer's unmet requirement fails at setup — never
//! silently at runtime.

use core::fmt;

/// One thing an adapter may or may not be able to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Capability {
    /// Observes every inbound stanza, without exception.
    L0InboundTap,
    /// The inbound tap also covers the authentication and stream-control
    /// phase, not just post-login traffic.
    L0InboundAuthPhase,
    /// Sends a raw stanza.
    L0Outbound,
    /// Reports each stanza the engine *sent*, as it went to the wire.
    ///
    /// Distinct from [`L0Outbound`], which is the ability to send. Sending is
    /// what an adapter does; knowing what left is what a recording needs, and
    /// an engine can perfectly well offer one and not the other — every engine
    /// did, until one of them added an outbound observation point.
    ///
    /// Without it a recording holds the inbound half of a session and nothing
    /// the client replied, so a gate can say the candidate *read* the same
    /// traffic and not that it *answered* the same way.
    ///
    /// [`L0Outbound`]: Self::L0Outbound
    L0OutboundObserved,
    /// Raw request/response against a stanza, correlated by the engine.
    L0Request,
    /// Says *why* an `<enc>` produced no plaintext, not merely that none
    /// arrived.
    ///
    /// [`PlaintextStatus`] has carried `DecryptFailed` and `Unsupported` since
    /// the format was written, and no adapter has ever emitted either: all
    /// four watch payloads appear and are never told why one did not, so they
    /// report [`Unobserved`] and no cause.
    ///
    /// The distinction is what a gate needs. Under `Unobserved`, a candidate
    /// build whose messages stopped decrypting looks exactly like one whose
    /// adapter stopped observing — the failure and the blind spot are the same
    /// absence. Every engine knows the difference internally and reports it
    /// somewhere else, on an event no adapter consumes.
    ///
    /// Named before publication rather than after, because the format
    /// anticipated it and only the vocabulary did not.
    ///
    /// [`PlaintextStatus`]: crate::PlaintextStatus
    /// [`Unobserved`]: crate::PlaintextStatus::Unobserved
    L0PlaintextCause,
    /// Emits the payloads it decrypted alongside the frame, so a consumer gets
    /// L0-plain rather than only L0-wire.
    ///
    /// Separate from the inbound tap because the two live at different points
    /// in an engine: the frame is available the moment a stanza is decoded,
    /// while a plaintext only exists after Signal has run. An engine can
    /// perfectly well offer one and not the other.
    L0Plaintext,
    /// Suppresses the engine's own dispatch, leaving it as transport and acks.
    /// Never suppresses decryption — L0-plain depends on it.
    Takeover,
    /// Supplies the engine's original frame bytes, so nothing is re-encoded.
    ZeroCopyFrame,
    /// Reports when incoming handlers have drained, which a clean detach
    /// requires.
    DrainHook,
}

impl Capability {
    /// Every capability this contract version defines.
    pub const ALL: [Self; 10] = [
        Self::L0InboundTap,
        Self::L0InboundAuthPhase,
        Self::L0Outbound,
        Self::L0OutboundObserved,
        Self::L0Request,
        Self::L0Plaintext,
        Self::L0PlaintextCause,
        Self::Takeover,
        Self::ZeroCopyFrame,
        Self::DrainHook,
    ];

    /// The stable identifier used in manifests and diagnostics.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::L0InboundTap => "l0.inbound.tap",
            Self::L0InboundAuthPhase => "l0.inbound.auth-phase",
            Self::L0Outbound => "l0.outbound",
            Self::L0OutboundObserved => "l0.outbound.observed",
            Self::L0Request => "l0.request",
            Self::L0Plaintext => "l0.plaintext",
            Self::L0PlaintextCause => "l0.plaintext.cause",
            Self::Takeover => "l0.takeover",
            Self::ZeroCopyFrame => "l0.zero-copy-frame",
            Self::DrainHook => "lifecycle.drain-hook",
        }
    }

    /// Resolve an identifier back to a capability.
    #[must_use]
    pub fn from_identifier(identifier: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|capability| capability.identifier() == identifier)
    }

    const fn bit(self) -> u16 {
        match self {
            Self::L0InboundTap => 0,
            Self::L0InboundAuthPhase => 1,
            Self::L0Outbound => 2,
            Self::L0Request => 3,
            Self::L0Plaintext => 4,
            Self::Takeover => 5,
            Self::ZeroCopyFrame => 6,
            Self::DrainHook => 7,
            Self::L0OutboundObserved => 8,
            Self::L0PlaintextCause => 9,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.identifier())
    }
}

/// A set of capabilities, packed into a bitmask.
///
/// `u16` because there are ten and there were eight. Widening it is a
/// source-level change and not a wire one: a recording declares capabilities by
/// *name* and keeps the ones it does not recognise as bytes, so a reader from
/// before the ninth still round-trips a recording that claims it. That is why
/// adding one does not bump [`ContractVersion`](crate::ContractVersion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct CapabilitySet(u16);

impl CapabilitySet {
    /// The empty set.
    pub const NONE: Self = Self(0);

    /// Build from an iterator of capabilities.
    #[must_use]
    pub fn from_iter_of<I: IntoIterator<Item = Capability>>(capabilities: I) -> Self {
        capabilities.into_iter().fold(Self::NONE, Self::with)
    }

    /// This set plus `capability`.
    #[must_use]
    pub const fn with(self, capability: Capability) -> Self {
        Self(self.0 | (1u16 << capability.bit()))
    }

    /// This set without `capability`.
    #[must_use]
    pub const fn without(self, capability: Capability) -> Self {
        Self(self.0 & !(1u16 << capability.bit()))
    }

    /// Whether `capability` is present.
    #[must_use]
    pub const fn contains(self, capability: Capability) -> bool {
        self.0 & (1u16 << capability.bit()) != 0
    }

    /// Whether every capability in `other` is present here.
    #[must_use]
    pub const fn contains_all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// How many capabilities are present.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// The capabilities in `required` that this set lacks.
    #[must_use]
    pub const fn missing_from(self, required: Self) -> Self {
        Self(required.0 & !self.0)
    }

    /// Iterate the capabilities present, in declaration order.
    pub fn iter(self) -> impl Iterator<Item = Capability> {
        Capability::ALL
            .into_iter()
            .filter(move |capability| self.contains(*capability))
    }

    /// Verify this set satisfies everything a consumer requires.
    ///
    /// This is the setup-time gate: an unmet requirement is an error here, not
    /// a surprise later.
    pub fn check_supports(self, required: Self) -> Result<(), UnmetCapabilities> {
        let missing = self.missing_from(required);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(UnmetCapabilities { missing })
        }
    }
}

impl FromIterator<Capability> for CapabilitySet {
    fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        Self::from_iter_of(iter)
    }
}

impl fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("(none)");
        }
        for (index, capability) in self.iter().enumerate() {
            if index != 0 {
                f.write_str(", ")?;
            }
            f.write_str(capability.identifier())?;
        }
        Ok(())
    }
}

/// A consumer required capabilities the adapter does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmetCapabilities {
    /// The capabilities that are required but absent.
    pub missing: CapabilitySet,
}

impl fmt::Display for UnmetCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "adapter lacks required capabilities: {}", self.missing)
    }
}

impl core::error::Error for UnmetCapabilities {}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    #[test]
    fn identifiers_are_unique_and_resolvable() {
        for (i, a) in Capability::ALL.iter().enumerate() {
            assert!(!a.identifier().is_empty());
            assert_eq!(Capability::from_identifier(a.identifier()), Some(*a));
            assert_eq!(a.to_string(), a.identifier());
            for b in Capability::ALL.iter().skip(i.saturating_add(1)) {
                assert_ne!(a.identifier(), b.identifier());
            }
        }
        assert_eq!(Capability::from_identifier("nope"), None);
        assert_eq!(Capability::from_identifier(""), None);
    }

    #[test]
    fn bits_are_unique_so_the_mask_is_lossless() {
        let mut seen = 0u16;
        for capability in Capability::ALL {
            let bit = 1u16 << capability.bit();
            assert_eq!(seen & bit, 0, "{capability} reuses a bit");
            seen |= bit;
        }
        assert_eq!(
            seen.count_ones(),
            u32::try_from(Capability::ALL.len()).unwrap()
        );
    }

    #[test]
    fn empty_set_contains_nothing() {
        let set = CapabilitySet::NONE;
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.iter().count(), 0);
        assert_eq!(set, CapabilitySet::default());
        assert_eq!(set.to_string(), "(none)");
        for capability in Capability::ALL {
            assert!(!set.contains(capability));
        }
    }

    #[test]
    fn adding_and_removing_is_symmetric() {
        for capability in Capability::ALL {
            let set = CapabilitySet::NONE.with(capability);
            assert!(set.contains(capability));
            assert_eq!(set.len(), 1);
            let back = set.without(capability);
            assert!(!back.contains(capability));
            assert!(back.is_empty());
            // Both operations are idempotent.
            assert_eq!(set.with(capability), set);
            assert_eq!(back.without(capability), back);
        }
    }

    #[test]
    fn a_full_set_holds_everything_in_order() {
        let full = CapabilitySet::from_iter_of(Capability::ALL);
        assert_eq!(full.len(), u32::try_from(Capability::ALL.len()).unwrap());
        assert!(!full.is_empty());
        assert_eq!(full.iter().collect::<Vec<_>>(), Capability::ALL.to_vec());
        assert!(full.contains_all(full));
        assert!(full.contains_all(CapabilitySet::NONE));
    }

    #[test]
    fn collect_matches_the_explicit_constructor() {
        let wanted = [Capability::Takeover, Capability::L0Outbound];
        let collected: CapabilitySet = wanted.into_iter().collect();
        assert_eq!(collected, CapabilitySet::from_iter_of(wanted));
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn contains_all_detects_a_partial_set() {
        let adapter =
            CapabilitySet::from_iter_of([Capability::L0InboundTap, Capability::L0Outbound]);
        let subset = CapabilitySet::NONE.with(Capability::L0Outbound);
        let superset = adapter.with(Capability::Takeover);

        assert!(adapter.contains_all(subset));
        assert!(!adapter.contains_all(superset));
        assert!(superset.contains_all(adapter));
    }

    #[test]
    fn missing_from_names_exactly_what_is_absent() {
        let adapter = CapabilitySet::NONE.with(Capability::L0InboundTap);
        let required = CapabilitySet::from_iter_of([
            Capability::L0InboundTap,
            Capability::Takeover,
            Capability::ZeroCopyFrame,
        ]);
        let missing = adapter.missing_from(required);
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(Capability::Takeover));
        assert!(missing.contains(Capability::ZeroCopyFrame));
        assert!(!missing.contains(Capability::L0InboundTap));
    }

    #[test]
    fn check_supports_gates_at_setup() {
        // Modelled on the real matrix: zapo has takeover, whatsapp-rust does not.
        let zapo = CapabilitySet::from_iter_of([
            Capability::L0InboundTap,
            Capability::L0Outbound,
            Capability::L0Request,
            Capability::Takeover,
            Capability::DrainHook,
        ]);
        let whatsapp_rust = CapabilitySet::from_iter_of([
            Capability::L0InboundTap,
            Capability::L0InboundAuthPhase,
            Capability::L0Outbound,
            Capability::L0Request,
            Capability::ZeroCopyFrame,
            Capability::DrainHook,
        ]);
        let needs_takeover = CapabilitySet::NONE.with(Capability::Takeover);

        assert_eq!(zapo.check_supports(needs_takeover), Ok(()));

        let err = whatsapp_rust
            .check_supports(needs_takeover)
            .expect_err("must not pass");
        assert_eq!(err.missing, needs_takeover);
        assert!(err.to_string().contains("l0.takeover"), "{err}");

        // The auth-phase gap runs the other way.
        let needs_auth = CapabilitySet::NONE.with(Capability::L0InboundAuthPhase);
        assert_eq!(whatsapp_rust.check_supports(needs_auth), Ok(()));
        assert!(zapo.check_supports(needs_auth).is_err());
    }

    #[test]
    fn requiring_nothing_always_passes() {
        assert_eq!(
            CapabilitySet::NONE.check_supports(CapabilitySet::NONE),
            Ok(())
        );
    }

    #[test]
    fn display_lists_capabilities() {
        let set = CapabilitySet::from_iter_of([Capability::L0InboundTap, Capability::Takeover]);
        let text = set.to_string();
        assert!(text.contains("l0.inbound.tap"), "{text}");
        assert!(text.contains("l0.takeover"), "{text}");
        assert!(text.contains(", "), "{text}");
    }

    #[test]
    fn unmet_is_a_std_error() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&UnmetCapabilities {
            missing: CapabilitySet::NONE.with(Capability::Takeover),
        });
    }

    #[test]
    fn capabilities_order_and_compare() {
        assert!(Capability::L0InboundTap < Capability::Takeover);
        assert_eq!(Capability::Takeover, Capability::Takeover);
    }
}
