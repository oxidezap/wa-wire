//! What the window can and cannot tell, including where it stops.

use super::*;

use alloc::format;

use crate::testing::{Fixture, FixtureBuilder, parse};

fn stanza(tag: &str, id: &str) -> FixtureBuilder {
    Fixture::node(tag).attr("id", id)
}

/// Admit one stanza and say what came back.
///
/// The fixture has to outlive the borrow, so each call builds and parses in one
/// place rather than handing a `NodeRef` around.
fn admit<const N: usize>(seen: &mut SeenStanzas<N>, builder: FixtureBuilder) -> Admission {
    let fixture = builder.build();
    seen.admit(&parse(&fixture))
}

#[test]
fn the_same_stanza_twice_is_a_duplicate_the_second_time() {
    let mut seen = SeenStanzas::<8>::new();

    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::New);
    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::Duplicate);
    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::Duplicate);
}

#[test]
fn two_stanzas_sharing_an_id_are_told_apart_by_their_tag() {
    // An ack and a receipt for one message carry that message's id. Keying on
    // the id alone would report the second as a redelivery of the first and
    // drop it.
    let mut seen = SeenStanzas::<8>::new();

    assert_eq!(admit(&mut seen, stanza("ack", "M1")), Admission::New);
    assert_eq!(admit(&mut seen, stanza("receipt", "M1")), Admission::New);
}

#[test]
fn a_stanza_with_no_id_is_untracked_rather_than_new() {
    // Nothing identifies it, so claiming it is new is a claim this cannot
    // support — and a caller counting duplicates must not read it as evidence
    // there were none.
    let mut seen = SeenStanzas::<8>::new();

    let first = admit(&mut seen, Fixture::node("ack"));
    let second = admit(&mut seen, Fixture::node("ack"));

    assert_eq!(first, Admission::Untracked);
    assert_eq!(second, Admission::Untracked);
    assert!(seen.is_empty(), "and nothing was spent remembering it");
}

#[test]
fn untracked_is_still_processed() {
    // The alternative is dropping real traffic to avoid a duplicate that was
    // never detected.
    assert!(Admission::Untracked.should_process());
    assert!(Admission::New.should_process());
    assert!(!Admission::Duplicate.should_process());
}

#[test]
fn an_id_longer_than_the_window_holds_is_untracked_rather_than_truncated() {
    // Truncating would make every id sharing a prefix collide, and a collision
    // here reports real traffic as a duplicate — the one error that loses a
    // message.
    let mut seen = SeenStanzas::<8>::new();
    let long = core::iter::repeat_n('L', MAX_ID + 1).collect::<alloc::string::String>();

    assert_eq!(admit(&mut seen, stanza("ack", &long)), Admission::Untracked);
    assert!(seen.is_empty());
}

#[test]
fn an_id_exactly_at_the_bound_still_fits() {
    let mut seen = SeenStanzas::<8>::new();
    let exact = core::iter::repeat_n('L', MAX_ID).collect::<alloc::string::String>();

    assert_eq!(admit(&mut seen, stanza("ack", &exact)), Admission::New);
    assert_eq!(
        admit(&mut seen, stanza("ack", &exact)),
        Admission::Duplicate
    );
}

#[test]
fn the_oldest_falls_out_when_the_window_is_full() {
    let mut seen = SeenStanzas::<4>::new();

    for index in 0..4 {
        assert_eq!(
            admit(&mut seen, stanza("ack", &format!("A{index}"))),
            Admission::New
        );
    }
    assert_eq!(seen.len(), 4);

    // A fifth pushes the first out.
    assert_eq!(admit(&mut seen, stanza("ack", "A4")), Admission::New);
    assert_eq!(
        admit(&mut seen, stanza("ack", "A0")),
        Admission::New,
        "evicted, so it reads as new — the trade for a bounded window"
    );
    // What is still inside is still recognised.
    assert_eq!(admit(&mut seen, stanza("ack", "A3")), Admission::Duplicate);
}

#[test]
fn a_burst_of_duplicates_does_not_empty_the_window() {
    // Re-inserting a duplicate would evict one older entry per redelivery, so a
    // run of them would push out everything the window was holding — and the
    // stanzas it was there to recognise would come back as new.
    let mut seen = SeenStanzas::<4>::new();

    for index in 0..4 {
        admit(&mut seen, stanza("ack", &format!("A{index}")));
    }
    for _ in 0..50 {
        assert_eq!(admit(&mut seen, stanza("ack", "A0")), Admission::Duplicate);
    }

    for index in 0..4 {
        assert_eq!(
            admit(&mut seen, stanza("ack", &format!("A{index}"))),
            Admission::Duplicate,
            "A{index} survived the burst"
        );
    }
}

#[test]
fn clearing_forgets_everything() {
    let mut seen = SeenStanzas::<8>::new();

    admit(&mut seen, stanza("ack", "A1"));
    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::Duplicate);

    seen.clear();
    assert!(seen.is_empty());
    assert_eq!(
        admit(&mut seen, stanza("ack", "A1")),
        Admission::New,
        "a new session's ids mean nothing to the old one's window"
    );
}

#[test]
fn a_window_of_zero_remembers_nothing_and_does_not_panic() {
    // A caller asking for no deduplication, said in the type.
    let mut seen = SeenStanzas::<0>::new();

    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::New);
    assert_eq!(admit(&mut seen, stanza("ack", "A1")), Admission::New);
    assert!(seen.is_empty());
}

#[test]
fn debug_does_not_print_the_ids() {
    // They are traffic. A log line is not where they belong.
    let mut seen = SeenStanzas::<8>::new();
    admit(&mut seen, stanza("ack", "SECRET-ID"));

    let rendered = format!("{seen:?}");
    assert!(!rendered.contains("SECRET"), "{rendered}");
    assert!(rendered.contains("window"), "{rendered}");
}

#[test]
fn a_wrapped_ring_is_searched_whole_rather_than_up_to_the_write_point() {
    // `filled` saturates at the window and the scan takes that many, so a ring
    // that has gone round must still be searched to its end. Reading only up to
    // the next write position would forget the newest half.
    let mut seen = SeenStanzas::<4>::new();

    for index in 0..7 {
        admit(&mut seen, stanza("ack", &format!("A{index}")));
    }

    // A6 A5 A4 A3 are held; A3 sits at the highest index the ring reached.
    for index in 3..7 {
        assert_eq!(
            admit(&mut seen, stanza("ack", &format!("A{index}"))),
            Admission::Duplicate,
            "A{index} is still inside a ring that wrapped"
        );
    }
}
