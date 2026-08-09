//! Every shape the derivation models must appear in the corpus.
//!
//! The four-engine agreement is only as wide as the traffic it replays. It ran
//! green for a long time over a corpus that reached five of the sixteen shapes
//! `wa-wire-l1` models: the other eleven were derived by unit tests, which are
//! written against one implementation and cannot disagree with anything.
//!
//! So this counts. A shape with no corpus stanza is a shape four engines have
//! never been asked to agree about, and a spec refresh that adds one arrives
//! here as a failure naming it rather than as a number quietly staying put.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use wa_wire_codec::Parser;
use wa_wire_l1::{Event, SHAPE_NAMES, derive};

/// Shapes no stanza can reach, and what claims them first.
///
/// Not a corpus gap and not a defect to fix by reordering. `derive` tries a
/// tag's shapes most-specific first (D-041), and a shape whose demands are a
/// superset of an earlier one's can never win: every stanza it would accept,
/// the earlier one already accepted. The real client tells these apart by
/// which request the response answers, which a pure function of one stanza
/// cannot see (D-010).
///
/// Listed rather than computed. A subset test over required fields would also
/// flag `CallReceiptParser`, which shares its required pair with
/// `IncomingMsgReceiptParser` and is reachable anyway: a `type` outside the
/// message-receipt enum makes the earlier shape reject. Reachability turns on
/// what a field *accepts*, not only on whether it is demanded.
const UNREACHABLE_BY_DISPATCH: [(&str, &str); 1] = [(
    "ParseNewsletterResponseSuccess",
    "both alternatives of its required union demand `t`, and `SendMsgAckSyncParser` \
     demands `t` and nothing else, so it is tried first and always matches",
)];

fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the corpus directory reads")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("a corpus file reads"))
        })
        .collect()
}

/// The shape name `derive` produced, or `None` where it derived nothing.
fn shape_of(event: &Event<'_>) -> String {
    // The variant name, which is the shape's name. Debug on the event prints
    // `Variant(..)`, so the name is everything before the parenthesis.
    let rendered = format!("{event:?}");
    rendered
        .split(['(', ' '])
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn the_corpus_reaches_every_shape_the_derivation_models() {
    let parser = Parser::new(wa_wire_codec::tokens::TABLE);
    let mut reached: BTreeSet<String> = BTreeSet::new();

    for (name, frame) in corpus() {
        let node = parser
            .parse(&frame)
            .unwrap_or_else(|error| panic!("{name}: the corpus must parse: {error:?}"));
        if let Ok(event) = derive(&node) {
            reached.insert(shape_of(&event));
        }
    }

    let missing: Vec<&str> = SHAPE_NAMES
        .iter()
        .copied()
        .filter(|shape| !reached.contains(*shape))
        .collect();
    let declared: Vec<&str> = UNREACHABLE_BY_DISPATCH
        .iter()
        .map(|(shape, _)| *shape)
        .collect();

    // Equal, not a subset either way. A shape that became reachable should stop
    // being excused, and one that stopped being reachable should be understood
    // before it is added to the list.
    assert_eq!(
        missing, declared,
        "the shapes no corpus stanza reaches are not the ones declared \
         unreachable. Add a stanza to `emit-corpus`, or work out what now claims \
         the shape first and say so in UNREACHABLE_BY_DISPATCH."
    );
}

/// The names this reads are the ones `derive` produces, not a parallel list.
///
/// `SHAPE_NAMES` comes from the generator, so it cannot drift from the shapes.
/// What could drift is *this file's* way of reading a shape's name back off an
/// event, which is `Debug` output. If that ever stopped matching, the coverage
/// test above would report every shape missing — or worse, none.
#[test]
fn a_shape_name_read_off_an_event_is_one_the_generator_declares() {
    let parser = Parser::new(wa_wire_codec::tokens::TABLE);
    let known: BTreeSet<&str> = SHAPE_NAMES.iter().copied().collect();

    for (name, frame) in corpus() {
        let node = parser.parse(&frame).expect("the corpus parses");
        let Ok(event) = derive(&node) else { continue };
        let shape = shape_of(&event);
        assert!(
            known.contains(shape.as_str()),
            "{name} derived {shape}, which the generator does not declare"
        );
    }
}

/// A stanza no shape claims is worth keeping, and worth being deliberate about.
#[test]
fn the_unmodelled_stanza_is_still_unmodelled() {
    let parser = Parser::new(wa_wire_codec::tokens::TABLE);
    let unmodelled: Vec<String> = corpus()
        .into_iter()
        .filter(|(_, frame)| {
            parser
                .parse(frame)
                .ok()
                .is_some_and(|node| derive(&node).is_err())
        })
        .map(|(name, _)| name)
        .collect();

    // One, on purpose: a tag the derivation does not model, kept so the four
    // engines are compared on a stanza that crosses as bytes and nothing more.
    assert_eq!(unmodelled.len(), 1, "{unmodelled:?}");
}
