//! Releasing a session so another engine can take it, and not two at once.
//!
//! Two requirements meet here, and both are about damage that cannot be undone.
//!
//! **R1.** A per-session lock is not enough across hosts. After a garbage
//! collection pause or a network partition, two hosts can each believe they own
//! one session, and the Signal ratchet does not survive two writers — every
//! message advances it, so a second writer leaves the peer with a `bad MAC` that
//! no retry clears. The discipline for that is a monotonic fencing token, and it
//! is not retrofittable: a system that starts without one has no moment at which
//! it can begin refusing the older owner. [`Fence`] is that gate.
//!
//! **R4.** `detach` and `logout` end a session in ways that look alike from one
//! line away and differ completely in consequence. A detach hands the session
//! on; a logout unpairs the customer's device, and nothing brings that back
//! without them scanning a code again. Getting that wrong once is worse than
//! every duplicate message this project has spent effort on.
//!
//! # Why a trait rather than an enum
//!
//! `enum End { Detach, Logout }` is the shape a bug flips. One wrong variant and
//! it still compiles. So the two are not two values of one type here: [`Detach`]
//! offers detaching **and nothing else**, and a host driving a handoff holds one
//! of those. It cannot log out because there is no method to call — not because
//! it was careful.
//!
//! Logging out stays where each engine already put it, reached deliberately by a
//! caller holding that engine's own type.
//!
//! # How the two meet
//!
//! The fence is the host's bookkeeping and the detach is the engine's act, and
//! they are separate on purpose: an engine has no way to know what else in the
//! fleet believes it owns this session. A handoff runs them in order — the
//! outgoing host presents its token, and only then asks its engine to let go;
//! the incoming host is issued a higher one, which is what makes the first host
//! discover it lost the session rather than write over its successor.
//!
//! [`Capability::Detach`](wa_wire_contract::Capability::Detach) is how an
//! adapter says its engine can do the second half at all.

use core::fmt;

/// Which owner an operation is speaking for.
///
/// Monotonic and persisted: the number has to outlive the host that issued it,
/// because the whole point is telling a host that came back from a pause that
/// the session moved on without it. Persist it as the `u64` it is.
///
/// Where it comes from is not this crate's business. A lock service, a database
/// sequence and a single-host counter all work, as long as the value only ever
/// increases over the session's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FencingToken(u64);

impl FencingToken {
    /// The token an owner was issued.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The number to persist.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next token after this one, when a single host is issuing them.
    ///
    /// Returns `None` at the maximum rather than wrapping. A wrapped token
    /// compares below every token already in the field, which is precisely the
    /// state fencing exists to make impossible — and a silent wrap is worse than
    /// a stop, because it looks like it worked.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl fmt::Display for FencingToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "fence:{}", self.0)
    }
}

/// An operation refused because a newer owner has taken the session.
///
/// Names both tokens, so a host can report that it *lost* the session rather
/// than only that something failed — the two call for opposite responses, and a
/// host that cannot tell them apart will retry its way into the double-write
/// this exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fenced {
    /// What the caller presented.
    pub presented: FencingToken,
    /// The newest this fence has admitted.
    pub current: FencingToken,
}

impl fmt::Display for Fenced {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "fenced out: presented {}, but {} owns the session",
            self.presented, self.current
        )
    }
}

impl core::error::Error for Fenced {}

/// The highest token a session has admitted, and the gate that keeps it.
///
/// One of these guards one session. Every operation that writes session state
/// goes through [`admit`](Self::admit) first; one carrying an older token is
/// refused, which is what makes a host returning from a pause discover it lost
/// ownership instead of writing over its successor.
///
/// Equal tokens are admitted: that is the same owner speaking again, and
/// refusing it would make a fence a single-use lock.
///
/// ```
/// use wa_wire_adapter::handoff::{Fence, FencingToken};
///
/// let mut fence = Fence::new();
/// let first = FencingToken::new(7);
/// let second = FencingToken::new(8);
///
/// assert!(fence.admit(first).is_ok());
/// assert!(fence.admit(second).is_ok());
/// // The first owner comes back from a pause and finds the session moved on.
/// assert!(fence.admit(first).is_err());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fence {
    current: Option<FencingToken>,
}

impl Fence {
    /// A fence that has admitted nothing, so any token may take the session.
    #[must_use]
    pub const fn new() -> Self {
        Self { current: None }
    }

    /// A fence restored from what was persisted.
    ///
    /// A host that restarts has to come back knowing the highest token the
    /// session reached, or it would admit an owner it had already fenced out.
    #[must_use]
    pub const fn resumed_at(token: FencingToken) -> Self {
        Self {
            current: Some(token),
        }
    }

    /// The newest owner this has admitted, if any.
    #[must_use]
    pub const fn current(&self) -> Option<FencingToken> {
        self.current
    }

    /// Let this owner act, or refuse it because a newer one exists.
    ///
    /// # Errors
    ///
    /// [`Fenced`] when the token is older than the newest admitted. A refusal
    /// leaves the fence where it was — a stale token that advanced it would
    /// fence out the owner that legitimately holds the session.
    pub fn admit(&mut self, token: FencingToken) -> Result<(), Fenced> {
        match self.current {
            Some(current) if token < current => Err(Fenced {
                presented: token,
                current,
            }),
            _ => {
                self.current = Some(token);
                Ok(())
            }
        }
    }

    /// Whether this token would be admitted, without admitting it.
    #[must_use]
    pub fn would_admit(&self, token: FencingToken) -> bool {
        self.current.is_none_or(|current| token >= current)
    }
}

/// Whether anything actually said the engine had gone quiet.
///
/// RFC-003's phase 2 drains in-flight sends, pending acks and active retries
/// before the session is released. An engine that reports when its incoming
/// handlers have finished can say so; three of the four cannot, and
/// [`Capability::DrainHook`] is how an adapter declares which it is.
///
/// So a barrier has two outcomes and not one. Collapsing them would let a host
/// read "I stopped waiting" as "there was nothing left" — the same mistake
/// [`Admission::Untracked`] and [`PlaintextStatus::Unobserved`] exist to refuse,
/// and a worse one here, because what is lost is an ack the server will now
/// resend to whoever holds the session next.
///
/// [`Capability::DrainHook`]: wa_wire_contract::Capability::DrainHook
/// [`Admission::Untracked`]: https://docs.rs/wa-wire-l1
/// [`PlaintextStatus::Unobserved`]: wa_wire_contract::PlaintextStatus::Unobserved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quiet {
    /// The engine reported that its handlers had drained.
    Confirmed,
    /// Nothing reported anything. The host stopped waiting, and what was in
    /// flight is unknown rather than absent.
    Unconfirmed,
}

impl Quiet {
    /// Whether the engine said so, rather than the host having given up.
    #[must_use]
    pub const fn is_confirmed(self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

impl fmt::Display for Quiet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Confirmed => "drained",
            Self::Unconfirmed => "not known to have drained",
        })
    }
}

/// The drain, waited on by the host and completed by the adapter.
///
/// Deliberately not a future. An engine reports a drain from a callback, and
/// making this async would push a runtime into a `no_std` crate that has none —
/// so it is a flag the adapter sets and the host reads, and how long to wait is
/// the host's business because the host is the one with a clock.
///
/// ```
/// use wa_wire_adapter::handoff::{Barrier, Quiet};
///
/// let barrier = Barrier::new();
/// assert_eq!(barrier.state(), Quiet::Unconfirmed);
///
/// // The adapter's drain hook fires.
/// barrier.drained();
/// assert_eq!(barrier.state(), Quiet::Confirmed);
/// ```
///
/// An adapter that does not declare
/// [`Capability::DrainHook`](wa_wire_contract::Capability::DrainHook) never
/// calls [`drained`](Self::drained), so its barrier stays `Unconfirmed` and the
/// host learns that from the type rather than from the matrix.
#[derive(Debug, Default)]
pub struct Barrier {
    drained: core::sync::atomic::AtomicBool,
}

impl Barrier {
    /// A barrier nothing has reported on yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            drained: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Report that the engine's handlers have finished.
    ///
    /// Idempotent, because a drain hook that fires twice is reporting the same
    /// fact and an adapter should not have to remember whether it already did.
    pub fn drained(&self) {
        self.drained
            .store(true, core::sync::atomic::Ordering::Release);
    }

    /// What is known right now.
    #[must_use]
    pub fn state(&self) -> Quiet {
        if self.drained.load(core::sync::atomic::Ordering::Acquire) {
            Quiet::Confirmed
        } else {
            Quiet::Unconfirmed
        }
    }
}

/// A command offered while the session was being handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offered<T> {
    /// The gate is open; send it now.
    Pass(T),
    /// Held for release when the session comes back.
    Held,
    /// The backlog is full and this was not held.
    ///
    /// Handed back rather than dropped, so the caller still has it and can
    /// decide: refuse the command, or give up on the handoff. Dropping it here
    /// would make a full backlog look like a successful hold.
    Full(T),
}

/// Phases 1 and 6: stop accepting commands, then let them go.
///
/// A handoff is stop-the-world because one device gets one connection, so
/// something has to hold what the application asks for in between. That
/// something is here rather than in an adapter: the commands are the host's,
/// no engine knows they exist, and the window is the host's to choose.
///
/// Bounded, for the same reason [`SeenStanzas`] is: this crate allocates
/// nowhere, and an unbounded backlog behind a handoff that has stalled is a
/// leak that presents as a hang. A full gate says so and hands the command
/// back.
///
/// Order is preserved. Releasing commands in a different order than they were
/// offered would reorder an application's sends, which is a bug it cannot see
/// and cannot have caused.
///
/// ```
/// use wa_wire_adapter::handoff::{Gate, Offered};
///
/// let mut gate: Gate<&str, 4> = Gate::new();
/// assert!(matches!(gate.offer("before"), Offered::Pass("before")));
///
/// gate.quiesce();
/// assert!(matches!(gate.offer("during"), Offered::Held));
///
/// let mut released = Vec::new();
/// gate.resume(|command| released.push(command));
/// assert_eq!(released, ["during"]);
/// assert!(matches!(gate.offer("after"), Offered::Pass("after")));
/// ```
///
/// [`SeenStanzas`]: https://docs.rs/wa-wire-l1
pub struct Gate<T, const N: usize = 32> {
    slots: [Option<T>; N],
    head: usize,
    len: usize,
    quiesced: bool,
}

impl<T, const N: usize> Gate<T, N> {
    /// An open gate with an empty backlog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| None),
            head: 0,
            len: 0,
            quiesced: false,
        }
    }

    /// Stop accepting commands. Idempotent.
    ///
    /// Phase 1. Everything offered from here is held until
    /// [`resume`](Self::resume), which is what makes the phases after it
    /// meaningful: draining what is in flight is pointless if the application
    /// is still adding to it.
    pub const fn quiesce(&mut self) {
        self.quiesced = true;
    }

    /// Whether commands are being held.
    #[must_use]
    pub const fn is_quiesced(&self) -> bool {
        self.quiesced
    }

    /// How many commands are waiting.
    #[must_use]
    pub const fn backlog(&self) -> usize {
        self.len
    }

    /// Whether the backlog has no room left.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Offer a command: send it now, hold it, or hand it back.
    pub fn offer(&mut self, command: T) -> Offered<T> {
        if !self.quiesced {
            return Offered::Pass(command);
        }
        if self.len >= N {
            // Also the zero-capacity case, which is a caller saying it wants no
            // backlog rather than an error — every command comes straight back.
            return Offered::Full(command);
        }
        // Wrapped by comparison rather than by `%`, the way `SeenStanzas` does:
        // a modulo is a second place the empty-capacity case has to be right in.
        let advanced = self.head.saturating_add(self.len);
        let at = if advanced >= N {
            advanced.saturating_sub(N)
        } else {
            advanced
        };
        let Some(slot) = self.slots.get_mut(at) else {
            return Offered::Full(command);
        };
        *slot = Some(command);
        self.len = self.len.saturating_add(1);
        Offered::Held
    }

    /// The slot after `at`, wrapped.
    const fn after(at: usize) -> usize {
        let advanced = at.saturating_add(1);
        if advanced >= N { 0 } else { advanced }
    }

    /// Phase 6: open the gate and release the backlog, oldest first.
    ///
    /// The gate opens *before* the backlog drains, so a command the release
    /// itself produces is passed rather than appended to a queue being emptied.
    /// Returns how many were released.
    pub fn resume(&mut self, mut release: impl FnMut(T)) -> usize {
        self.quiesced = false;
        let released = self.len;
        for _ in 0..released {
            if let Some(command) = self.slots.get_mut(self.head).and_then(Option::take) {
                release(command);
            }
            self.head = Self::after(self.head);
        }
        self.len = 0;
        released
    }

    /// Drop the backlog without releasing it, and open the gate.
    ///
    /// For a handoff that failed and is being abandoned: the commands were
    /// never sent, and the caller has already been told so by whatever refused
    /// them. Returns how many were discarded, because a host that abandons a
    /// handoff should be able to report what it cost.
    pub fn abandon(&mut self) -> usize {
        self.quiesced = false;
        let discarded = self.len;
        for _ in 0..discarded {
            if let Some(slot) = self.slots.get_mut(self.head) {
                *slot = None;
            }
            self.head = Self::after(self.head);
        }
        self.len = 0;
        discarded
    }
}

impl<T, const N: usize> Default for Gate<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Counts and state, never the commands.
///
/// A command is the application's, and can carry a message body. The same
/// reasoning keeps ids out of `SeenStanzas`' `Debug`.
#[expect(
    clippy::missing_fields_in_debug,
    reason = "a command can carry a message body and does not belong in a log line"
)]
impl<T, const N: usize> fmt::Debug for Gate<T, N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Gate")
            .field("quiesced", &self.quiesced)
            .field("backlog", &self.len)
            .field("capacity", &N)
            .finish()
    }
}

#[cfg(feature = "alloc")]
pub use self::releasing::{Detach, DetachFailed, DetachFuture};

#[cfg(feature = "alloc")]
mod releasing {
    use alloc::boxed::Box;
    use core::fmt;
    use core::future::Future;
    use core::pin::Pin;

    /// A future an adapter's detach returns.
    ///
    /// Boxed for the same reason [`SendFuture`] is: a host driving a handoff
    /// holds "some engine", and the whole point is not knowing which.
    ///
    /// [`SendFuture`]: crate::send::SendFuture
    pub type DetachFuture<'a> = Pin<Box<dyn Future<Output = Result<(), DetachFailed>> + Send + 'a>>;

    /// The session was not released.
    ///
    /// One failure rather than a taxonomy, because a host has one correct
    /// response to every version of it: do not attach elsewhere. The old
    /// connection may still be live, and a handoff that continues past this is
    /// the two-writer case with extra steps.
    ///
    /// Carries the engine's own error so a report can name what happened, while
    /// keeping the type opaque so a host cannot come to depend on which engine
    /// produced it.
    #[derive(Debug)]
    pub struct DetachFailed(Box<dyn core::error::Error + Send + Sync>);

    impl DetachFailed {
        /// Wrap what the engine reported.
        #[must_use]
        pub fn new<E: core::error::Error + Send + Sync + 'static>(error: E) -> Self {
            Self(Box::new(error))
        }
    }

    impl fmt::Display for DetachFailed {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "the session was not released: {}", self.0)
        }
    }

    impl core::error::Error for DetachFailed {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            Some(self.0.as_ref())
        }
    }

    /// Releasing a session without ending the account's pairing.
    ///
    /// The only way this crate offers to give a session up, and it offers no way
    /// to log out — a host driving a handoff holds one of these and therefore
    /// cannot unpair a device by reaching for the neighbouring method. That is
    /// R4: the distinction is in what exists, not in what a caller remembers.
    ///
    /// # What an implementation promises
    ///
    /// - The connection ends and no other device is told anything. The account
    ///   stays registered and the pairing survives.
    /// - Once the future resolves, the engine's connection is gone and it will
    ///   not open another of its own accord, so a second engine can take the
    ///   session without the server killing one of them — one device, one
    ///   connection.
    /// - Whatever the engine would carry across a network drop it carries across
    ///   this, and nothing more. A session resumed elsewhere resyncs like any
    ///   reconnect; that loss is
    ///   [declared](wa_wire_contract::Capability::Detach), not avoided.
    ///
    /// "Will not open another" is the load-bearing half, and it is stated that
    /// way rather than as "holds no socket" because an engine can have a
    /// connection attempt in flight that this call cannot reach into. What every
    /// implementation must guarantee is that no such attempt becomes a live
    /// session; an engine that can only ask nicely does not have this
    /// capability.
    ///
    /// An engine with no way to release its transport does not implement this,
    /// and says so by not declaring
    /// [`Capability::Detach`](wa_wire_contract::Capability::Detach) — rather
    /// than by detaching badly.
    ///
    /// # The distinction, as a check
    ///
    /// A host driving a handoff holds `&dyn Detach`, and unpairing the device is
    /// not something it can reach. This is the same program twice, and only the
    /// last line differs — the first compiles:
    ///
    /// ```
    /// # use wa_wire_adapter::handoff::{Detach, DetachFuture};
    /// # struct Engine;
    /// # impl Engine { fn logout(&self) {} }
    /// # impl Detach for Engine {
    /// #     fn detach(&self) -> DetachFuture<'_> { Box::pin(async { Ok(()) }) }
    /// # }
    /// fn hand_the_session_on(engine: &dyn Detach) {
    ///     let _releasing = engine.detach();
    /// }
    /// ```
    ///
    /// and the second does not, because there is no such method to call:
    ///
    /// ```compile_fail
    /// # use wa_wire_adapter::handoff::{Detach, DetachFuture};
    /// # struct Engine;
    /// # impl Engine { fn logout(&self) {} }
    /// # impl Detach for Engine {
    /// #     fn detach(&self) -> DetachFuture<'_> { Box::pin(async { Ok(()) }) }
    /// # }
    /// fn hand_the_session_on(engine: &dyn Detach) {
    ///     engine.logout();
    /// }
    /// ```
    ///
    /// The pair is deliberate: a `compile_fail` example alone would keep passing
    /// if it started failing for some unrelated reason, and then it would be
    /// proving nothing.
    pub trait Detach: Send + Sync {
        /// Release the session, leaving the pairing intact.
        ///
        /// Idempotent: detaching an already-detached session succeeds, because
        /// the postcondition a host needs — no socket, no reconnect — already
        /// holds, and a host that crashed mid-handoff has to be able to start
        /// over.
        ///
        /// # Errors
        ///
        /// [`DetachFailed`] when the engine could not let go. The session stays
        /// where it was: a host must not read a failure as permission to attach
        /// elsewhere, because the old connection may still be live.
        fn detach(&self) -> DetachFuture<'_>;
    }
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;
