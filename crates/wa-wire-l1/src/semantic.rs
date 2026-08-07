//! Comparing derivations by meaning.
//!
//! Two engines can encode one stanza differently and both be right — a token
//! here, the same text as bytes there. A conformance run that called that a
//! divergence would bury the real ones, so comparison goes through the value's
//! meaning rather than its bytes.

use crate::error::DeriveError;

/// Whether two lazily-derived child sequences agree, item for item.
///
/// A derivation failure is part of the comparison: two engines that both fail
/// the same way on the same child agree, and one that fails where the other
/// does not is exactly the divergence worth reporting.
pub fn iter_eq<T, A, B>(mine: A, theirs: B) -> bool
where
    T: SemanticEq,
    A: Iterator<Item = Result<T, DeriveError>>,
    B: Iterator<Item = Result<T, DeriveError>>,
{
    let mut mine = mine;
    let mut theirs = theirs;
    loop {
        match (mine.next(), theirs.next()) {
            (None, None) => return true,
            (Some(Ok(a)), Some(Ok(b))) if a.semantic_eq(&b) => {}
            (Some(Err(a)), Some(Err(b))) if a == b => {}
            _ => return false,
        }
    }
}

/// A derived value that can be compared by meaning.
pub trait SemanticEq {
    /// Whether `self` and `other` mean the same thing.
    fn semantic_eq(&self, other: &Self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Item(u8);

    impl SemanticEq for Item {
        fn semantic_eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    fn run(a: Vec<Result<Item, DeriveError>>, b: Vec<Result<Item, DeriveError>>) -> bool {
        iter_eq(a.into_iter(), b.into_iter())
    }

    #[test]
    fn equal_sequences_agree() {
        assert!(run(
            vec![Ok(Item(1)), Ok(Item(2))],
            vec![Ok(Item(1)), Ok(Item(2))]
        ));
        assert!(run(vec![], vec![]));
    }

    #[test]
    fn a_different_item_is_a_divergence() {
        assert!(!run(vec![Ok(Item(1))], vec![Ok(Item(2))]));
    }

    #[test]
    fn a_different_length_is_a_divergence() {
        assert!(!run(vec![Ok(Item(1))], vec![Ok(Item(1)), Ok(Item(2))]));
        assert!(!run(vec![Ok(Item(1)), Ok(Item(2))], vec![Ok(Item(1))]));
    }

    #[test]
    fn failing_the_same_way_is_agreement() {
        // Both engines choked on the same child for the same reason. That is
        // consistency, not a finding.
        let error = DeriveError::MissingAttr { key: "t" };
        assert!(run(vec![Err(error)], vec![Err(error)]));
    }

    #[test]
    fn failing_differently_is_a_divergence() {
        assert!(!run(
            vec![Err(DeriveError::MissingAttr { key: "t" })],
            vec![Err(DeriveError::MissingAttr { key: "id" })]
        ));
        assert!(!run(
            vec![Err(DeriveError::MissingAttr { key: "t" })],
            vec![Ok(Item(1))]
        ));
        assert!(!run(
            vec![Ok(Item(1))],
            vec![Err(DeriveError::MissingAttr { key: "t" })]
        ));
    }
}
