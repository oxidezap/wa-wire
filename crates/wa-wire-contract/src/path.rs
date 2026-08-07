//! Addressing a node inside a frame.
//!
//! A path is the list of child indices from the root node. Correlating a
//! plaintext to its `<enc>` by path rather than by ordinal survives an engine
//! reordering or filtering children, and rather than by buffer offset because
//! the Go and TypeScript decoders do not expose offsets — and adapters are
//! meant to stay dumb.

use core::fmt;

/// A borrowed node path: little-endian `u16` components over the envelope's
/// own buffer.
///
/// Holding the raw bytes rather than a `&[u16]` avoids both an alignment
/// requirement the envelope cannot guarantee and a copy the boundary does not
/// need.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodePath<'a> {
    bytes: &'a [u8],
}

impl<'a> NodePath<'a> {
    /// Wrap the little-endian component bytes of a path.
    ///
    /// `bytes.len()` must be even; the decoder only ever produces such slices.
    /// An odd length truncates the trailing half-component rather than
    /// panicking.
    #[must_use]
    pub const fn from_le_bytes(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// The root node itself.
    #[must_use]
    pub const fn root() -> Self {
        Self { bytes: &[] }
    }

    /// Number of components.
    #[must_use]
    pub const fn len(self) -> usize {
        self.bytes.len() / 2
    }

    /// Whether this path addresses the root node.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The component at `index`, if present.
    #[must_use]
    pub fn get(self, index: usize) -> Option<u16> {
        let start = index.checked_mul(2)?;
        let end = start.checked_add(2)?;
        let pair = self.bytes.get(start..end)?;
        // `get` guaranteed exactly two bytes.
        let (lo, hi) = (*pair.first()?, *pair.get(1)?);
        Some(u16::from_le_bytes([lo, hi]))
    }

    /// Iterate the components from the root downwards.
    ///
    /// `as_chunks` yields fixed-size pairs, so decoding needs no fallible step
    /// and a trailing odd byte is simply not a component.
    pub fn iter(self) -> impl Iterator<Item = u16> + 'a {
        let (pairs, _odd_tail) = self.bytes.as_chunks::<2>();
        pairs.iter().copied().map(u16::from_le_bytes)
    }

    /// The underlying little-endian bytes.
    #[must_use]
    pub const fn as_le_bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Whether `self` is `other` or a node beneath it.
    #[must_use]
    pub fn starts_with(self, other: NodePath<'_>) -> bool {
        let prefix = other.len();
        if prefix > self.len() {
            return false;
        }
        self.iter()
            .zip(other.iter())
            .take(prefix)
            .all(|(a, b)| a == b)
    }
}

impl<'a> IntoIterator for NodePath<'a> {
    type Item = u16;
    type IntoIter =
        core::iter::Map<core::iter::Copied<core::slice::Iter<'a, [u8; 2]>>, fn([u8; 2]) -> u16>;

    fn into_iter(self) -> Self::IntoIter {
        let (pairs, _odd_tail) = self.bytes.as_chunks::<2>();
        pairs.iter().copied().map(u16::from_le_bytes as _)
    }
}

impl fmt::Debug for NodePath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl fmt::Display for NodePath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("/");
        }
        for component in self.iter() {
            write!(f, "/{component}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    fn path_bytes(components: &[u16]) -> Vec<u8> {
        components
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<_>>()
    }

    #[test]
    fn root_is_empty() {
        let root = NodePath::root();
        assert!(root.is_empty());
        assert_eq!(root.len(), 0);
        assert_eq!(root.get(0), None);
        assert_eq!(root.iter().count(), 0);
        assert_eq!(root.to_string(), "/");
    }

    #[test]
    fn components_round_trip_little_endian() {
        let components = [0u16, 1, 258, 0xFFFF, 7];
        let bytes = path_bytes(&components);
        let path = NodePath::from_le_bytes(&bytes);

        assert_eq!(path.len(), components.len());
        assert!(!path.is_empty());
        assert_eq!(path.iter().collect::<Vec<_>>(), components);
        assert_eq!(path.into_iter().collect::<Vec<_>>(), components);
        for (i, want) in components.iter().enumerate() {
            assert_eq!(path.get(i), Some(*want));
        }
        assert_eq!(path.get(components.len()), None);
        assert_eq!(path.as_le_bytes(), bytes.as_slice());
    }

    #[test]
    fn odd_length_truncates_instead_of_panicking() {
        // Not produced by the decoder, but the type must stay total.
        let path = NodePath::from_le_bytes(&[1, 0, 2]);
        assert_eq!(path.len(), 1);
        assert_eq!(path.iter().collect::<Vec<_>>(), [1]);
        assert_eq!(path.get(1), None);
    }

    #[test]
    fn get_rejects_indices_that_would_overflow() {
        let bytes = path_bytes(&[1, 2]);
        let path = NodePath::from_le_bytes(&bytes);
        assert_eq!(path.get(usize::MAX), None);
        assert_eq!(path.get(usize::MAX / 2), None);
    }

    #[test]
    fn starts_with_matches_ancestry() {
        let child = path_bytes(&[1, 2, 3]);
        let parent = path_bytes(&[1, 2]);
        let sibling = path_bytes(&[1, 3]);
        let deeper = path_bytes(&[1, 2, 3, 4]);

        let child = NodePath::from_le_bytes(&child);
        let parent = NodePath::from_le_bytes(&parent);
        let sibling = NodePath::from_le_bytes(&sibling);
        let deeper = NodePath::from_le_bytes(&deeper);

        assert!(child.starts_with(parent));
        assert!(child.starts_with(child));
        assert!(child.starts_with(NodePath::root()));
        assert!(!child.starts_with(sibling));
        assert!(!parent.starts_with(child));
        assert!(!child.starts_with(deeper));
    }

    #[test]
    fn display_and_debug_are_readable() {
        let bytes = path_bytes(&[0, 12, 5]);
        let path = NodePath::from_le_bytes(&bytes);
        assert_eq!(path.to_string(), "/0/12/5");
        assert_eq!(alloc::format!("{path:?}"), "[0, 12, 5]");
    }

    #[test]
    fn equality_is_by_value() {
        let a = path_bytes(&[1, 2]);
        let b = path_bytes(&[1, 2]);
        let c = path_bytes(&[1, 3]);
        assert_eq!(NodePath::from_le_bytes(&a), NodePath::from_le_bytes(&b));
        assert_ne!(NodePath::from_le_bytes(&a), NodePath::from_le_bytes(&c));
    }
}
