//! What the fence refuses, and what a detach cannot reach.

use super::*;

fn token(value: u64) -> FencingToken {
    FencingToken::new(value)
}

/// The scenario R1 exists for: two hosts each believing they own one session.
///
/// A garbage collection pause or a partition leaves the first host alive and
/// convinced. The second takes over. When the first comes back, every write it
/// makes advances a ratchet the second is also advancing, and the peer ends with
/// a `bad MAC` that no retry clears.
#[test]
fn a_host_that_comes_back_from_a_pause_is_refused() {
    let mut fence = Fence::new();

    // The first host takes the session and does some work.
    assert!(fence.admit(token(1)).is_ok());
    assert!(fence.admit(token(1)).is_ok(), "the same owner may continue");

    // It stalls. The lock service hands the session to a second host.
    assert!(fence.admit(token(2)).is_ok());

    // The first wakes up still holding its token and tries to write.
    let refused = fence.admit(token(1)).expect_err("the session moved on");
    assert_eq!(refused.presented, token(1));
    assert_eq!(refused.current, token(2));

    // And the second is unaffected by the attempt.
    assert!(fence.admit(token(2)).is_ok());
    assert_eq!(fence.current(), Some(token(2)));
}

#[test]
fn a_refusal_does_not_move_the_fence() {
    // A stale token that advanced the fence would be worse than one that did
    // nothing: the owner that legitimately holds the session would then be
    // fenced out by its own predecessor.
    let mut fence = Fence::resumed_at(token(5));

    assert!(fence.admit(token(3)).is_err());
    assert_eq!(fence.current(), Some(token(5)));
    assert!(fence.admit(token(5)).is_ok(), "the owner still owns it");
}

#[test]
fn a_restarted_host_comes_back_knowing_what_it_had_admitted() {
    // Restoring from nothing would admit an owner the session had already
    // fenced out, which is the failure this whole mechanism exists to prevent.
    let mut restored = Fence::resumed_at(token(9));

    assert!(restored.admit(token(8)).is_err());
    assert!(restored.admit(token(9)).is_ok());
    assert!(restored.admit(token(10)).is_ok());
}

#[test]
fn a_fresh_fence_admits_whatever_comes_first() {
    let mut fence = Fence::new();

    assert_eq!(fence.current(), None);
    assert!(fence.would_admit(token(0)));
    assert!(fence.would_admit(token(u64::MAX)));
    assert!(fence.admit(token(42)).is_ok());
    assert_eq!(fence.current(), Some(token(42)));
}

#[test]
fn asking_does_not_take() {
    let mut fence = Fence::new();
    fence.admit(token(4)).expect("admitted");

    assert!(!fence.would_admit(token(3)));
    assert!(fence.would_admit(token(4)));
    assert!(fence.would_admit(token(5)));
    assert_eq!(
        fence.current(),
        Some(token(4)),
        "asking about a newer token did not hand it the session"
    );
}

#[test]
fn tokens_stop_rather_than_wrap_at_the_end() {
    // A wrapped token compares below every token already in the field, which is
    // exactly the state fencing exists to make impossible — and it would look
    // like it worked.
    assert_eq!(token(7).next(), Some(token(8)));
    assert_eq!(token(u64::MAX).next(), None);
}

#[test]
fn a_token_survives_being_persisted_as_a_number() {
    let issued = token(1_234_567);
    let persisted = issued.get();

    assert_eq!(FencingToken::new(persisted), issued);
}

#[cfg(feature = "alloc")]
#[test]
fn a_refusal_says_who_owns_the_session() {
    // "Failed" is not actionable; "8 owns it and you presented 3" is — the host
    // has to be able to report that it lost the session rather than that
    // something went wrong, because the two call for opposite responses.
    let mut fence = Fence::resumed_at(token(8));
    let refused = fence.admit(token(3)).expect_err("stale");

    let rendered = alloc::format!("{refused}");
    assert!(rendered.contains("fence:3"), "{rendered}");
    assert!(rendered.contains("fence:8"), "{rendered}");
    assert!(core::error::Error::source(&refused).is_none());
}

// --- R4: detach is not logout ------------------------------------------------

#[cfg(feature = "alloc")]
mod releasing {
    use super::*;

    use alloc::boxed::Box;
    use core::future::Future;
    use core::sync::atomic::{AtomicBool, Ordering};

    /// Minimal executor, for the reason `send`'s tests carry their own: this
    /// crate has no runtime dependency and should not grow one for a test. The
    /// futures here never yield, so a no-op waker is all a poll loop needs.
    fn block_on<T>(future: impl Future<Output = T>) -> T {
        use core::task::{Context, Poll, Waker};

        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut future = Box::pin(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    /// An engine that can release its transport, and an account it must not
    /// unpair.
    struct Engine {
        detached: AtomicBool,
        paired: AtomicBool,
    }

    impl Engine {
        fn connected() -> Self {
            Self {
                detached: AtomicBool::new(false),
                paired: AtomicBool::new(true),
            }
        }

        fn detached(&self) -> bool {
            self.detached.load(Ordering::Relaxed)
        }

        fn paired(&self) -> bool {
            self.paired.load(Ordering::Relaxed)
        }

        /// Deliberately not part of [`Detach`]: reaching this is a choice a
        /// caller has to make by name, holding the engine's own type.
        fn logout(&self) {
            self.paired.store(false, Ordering::Relaxed);
        }
    }

    impl Detach for Engine {
        fn detach(&self) -> DetachFuture<'_> {
            Box::pin(async move {
                self.detached.store(true, Ordering::Relaxed);
                Ok(())
            })
        }
    }

    /// What a host driving a handoff is given: the ability to detach, and
    /// nothing that can unpair a device. There is no `engine.logout()` to write
    /// here — the doctest on [`Detach`] is what proves that, by failing to
    /// compile.
    fn hand_the_session_on(engine: &dyn Detach) -> Result<(), DetachFailed> {
        block_on(engine.detach())
    }

    #[test]
    fn detaching_leaves_the_pairing_alone() {
        let engine = Engine::connected();

        hand_the_session_on(&engine).expect("released");

        assert!(engine.detached(), "the session was released");
        assert!(
            engine.paired(),
            "and the device is still paired — a detach that unpaired would be a logout"
        );
    }

    #[test]
    fn logging_out_is_reachable_only_from_the_engine_itself() {
        // Less an assertion about behaviour than a record of the shape: a caller
        // that wants this must hold the concrete engine and say the word.
        let engine = Engine::connected();

        engine.logout();

        assert!(!engine.paired());
        assert!(
            !engine.detached(),
            "and logging out is not a detach either — the two are separate acts"
        );
    }

    #[test]
    fn detaching_twice_succeeds() {
        // A host that crashed mid-handoff has to be able to start over, and the
        // postcondition it needs — no socket, no reconnect — already holds.
        let engine = Engine::connected();

        assert!(hand_the_session_on(&engine).is_ok());
        assert!(hand_the_session_on(&engine).is_ok());
        assert!(engine.paired());
    }

    #[test]
    fn a_failed_detach_names_what_happened_and_keeps_the_cause() {
        #[derive(Debug)]
        struct Stuck;

        impl fmt::Display for Stuck {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("the socket never closed")
            }
        }

        impl core::error::Error for Stuck {}

        struct Refusing;

        impl Detach for Refusing {
            fn detach(&self) -> DetachFuture<'_> {
                Box::pin(async { Err(DetachFailed::new(Stuck)) })
            }
        }

        let failed = hand_the_session_on(&Refusing).expect_err("it did not let go");

        let rendered = alloc::format!("{failed}");
        assert!(rendered.contains("not released"), "{rendered}");
        assert!(rendered.contains("never closed"), "{rendered}");
        assert!(
            core::error::Error::source(&failed).is_some(),
            "the engine's own error stays reachable for a report"
        );
    }
}

// --- phases 1, 2 and 6: quiesce, barrier, resume ------------------------------

#[test]
fn a_gate_passes_until_it_is_told_not_to() {
    let mut gate: Gate<u8, 4> = Gate::new();

    assert!(!gate.is_quiesced());
    assert!(matches!(gate.offer(1), Offered::Pass(1)));
    assert_eq!(gate.backlog(), 0);

    gate.quiesce();
    assert!(gate.is_quiesced());
    assert!(matches!(gate.offer(2), Offered::Held));
    assert_eq!(gate.backlog(), 1);
}

#[test]
fn the_backlog_comes_back_in_the_order_it_went_in() {
    // Releasing out of order would reorder an application's sends — a bug it
    // cannot see and did not cause.
    let mut gate: Gate<u8, 8> = Gate::new();
    gate.quiesce();
    for command in 1..=5 {
        assert!(matches!(gate.offer(command), Offered::Held));
    }

    let mut released = alloc::vec::Vec::new();
    let count = gate.resume(|command| released.push(command));

    assert_eq!(count, 5);
    assert_eq!(released, alloc::vec![1, 2, 3, 4, 5]);
    assert_eq!(gate.backlog(), 0);
    assert!(!gate.is_quiesced());
}

#[test]
fn a_full_gate_hands_the_command_back_rather_than_dropping_it() {
    // Dropping here would make a full backlog look like a successful hold, and
    // the application would never learn its command went nowhere.
    let mut gate: Gate<u8, 2> = Gate::new();
    gate.quiesce();

    assert!(matches!(gate.offer(1), Offered::Held));
    assert!(matches!(gate.offer(2), Offered::Held));
    assert!(gate.is_full());

    match gate.offer(3) {
        Offered::Full(command) => assert_eq!(command, 3, "the caller still has it"),
        other => panic!("expected the command back, got {other:?}"),
    }
    assert_eq!(gate.backlog(), 2, "and nothing already held was displaced");
}

#[test]
fn the_gate_opens_before_the_backlog_drains() {
    // A command produced by releasing another must be sent, not appended to a
    // queue that is being emptied — otherwise it waits for a resume that has
    // already happened.
    let mut gate: Gate<u8, 4> = Gate::new();
    gate.quiesce();
    gate.offer(1);

    let mut sent = alloc::vec::Vec::new();
    gate.resume(|command| {
        sent.push(command);
    });
    assert!(matches!(gate.offer(9), Offered::Pass(9)));
    assert_eq!(sent, alloc::vec![1]);
}

#[test]
fn the_slots_are_reused_around_the_ring() {
    let mut gate: Gate<u8, 3> = Gate::new();
    for round in 0..4u8 {
        gate.quiesce();
        assert!(matches!(gate.offer(round), Offered::Held));
        assert!(matches!(gate.offer(round + 100), Offered::Held));

        let mut released = alloc::vec::Vec::new();
        gate.resume(|command| released.push(command));
        assert_eq!(released, alloc::vec![round, round + 100], "round {round}");
    }
}

#[test]
fn abandoning_discards_the_backlog_and_says_how_much() {
    // A handoff that failed is being given up on: the commands were never sent
    // and whoever refused them has already said so.
    let mut gate: Gate<u8, 4> = Gate::new();
    gate.quiesce();
    gate.offer(1);
    gate.offer(2);

    assert_eq!(gate.abandon(), 2);
    assert_eq!(gate.backlog(), 0);
    assert!(!gate.is_quiesced());
    assert!(matches!(gate.offer(3), Offered::Pass(3)));
}

#[test]
fn a_gate_reports_its_state_without_reporting_its_commands() {
    // A command can carry a message body. Ids stay out of `SeenStanzas`' Debug
    // for the same reason.
    let mut gate: Gate<&str, 4> = Gate::new();
    gate.quiesce();
    gate.offer("hello, this is the body");

    let rendered = alloc::format!("{gate:?}");
    assert!(rendered.contains("backlog"), "{rendered}");
    assert!(rendered.contains("capacity"), "{rendered}");
    assert!(
        !rendered.contains("hello"),
        "the command leaked: {rendered}"
    );
}

#[test]
fn a_barrier_says_nothing_until_something_reports() {
    // The whole point of the second answer: an adapter with no drain hook never
    // calls `drained`, so a host reads "not known" instead of "there was
    // nothing left".
    let barrier = Barrier::new();

    assert_eq!(barrier.state(), Quiet::Unconfirmed);
    assert!(!barrier.state().is_confirmed());

    barrier.drained();
    assert_eq!(barrier.state(), Quiet::Confirmed);
    assert!(barrier.state().is_confirmed());
}

#[test]
fn reporting_a_drain_twice_reports_the_same_fact() {
    let barrier = Barrier::default();
    barrier.drained();
    barrier.drained();
    assert_eq!(barrier.state(), Quiet::Confirmed);
}

#[cfg(feature = "alloc")]
#[test]
fn an_unconfirmed_barrier_says_so_in_words() {
    // This lands in a report a person reads, and "drained" against "not known
    // to have drained" is the difference between two handoffs that look alike.
    assert_eq!(alloc::format!("{}", Quiet::Confirmed), "drained");
    assert_eq!(
        alloc::format!("{}", Quiet::Unconfirmed),
        "not known to have drained"
    );
}
