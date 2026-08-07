//! Building a node path while walking a stanza.
//!
//! An adapter discovers a path as it descends: push a child index, recurse,
//! pop. The contract wants that path as little-endian bytes, so it is built in
//! that form directly rather than converted afterwards.
//!
//! Fixed capacity, no allocation. Real stanzas nest to depth 9 at the extreme
//! and the parser refuses more than 64, so a buffer that holds 64 components
//! cannot be the thing that fails first.

use core::fmt;

use wa_wire_contract::NodePath;

/// How many components a path may hold.
///
/// Matches the codec's default nesting limit: a path deeper than the parser
/// will accept cannot address anything.
pub const MAX_DEPTH: usize = 64;

const BUFFER_LEN: usize = MAX_DEPTH * 2;

/// A node path under construction.
#[derive(Clone, Copy)]
pub struct NodePathBuf {
    bytes: [u8; BUFFER_LEN],
    /// Number of components, not bytes.
    len: u8,
}

/// A push went past [`MAX_DEPTH`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathTooDeep {
    /// The limit that was hit.
    pub limit: usize,
}

impl fmt::Display for PathTooDeep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node path deeper than the limit of {}", self.limit)
    }
}

impl core::error::Error for PathTooDeep {}

impl NodePathBuf {
    /// An empty path, addressing the root.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; BUFFER_LEN],
            len: 0,
        }
    }

    /// Number of components.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Whether the path addresses the root.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Descend into the child at `index`.
    ///
    /// The single bounds check is the depth limit itself: the buffer is sized
    /// to hold exactly `MAX_DEPTH` components, so a push that clears the limit
    /// necessarily has a slot. Adding a second check would create an arm no
    /// test could reach.
    pub fn push(&mut self, index: u16) -> Result<(), PathTooDeep> {
        let start = self.len().saturating_mul(2);
        let Some(slot) = self.bytes.get_mut(start..start.saturating_add(2)) else {
            return Err(PathTooDeep { limit: MAX_DEPTH });
        };
        slot.copy_from_slice(&index.to_le_bytes());
        self.len = self.len.saturating_add(1);
        Ok(())
    }

    /// Ascend to the parent, returning the index that was popped.
    pub fn pop(&mut self) -> Option<u16> {
        let at = self.len().checked_sub(1)?;
        let start = at.saturating_mul(2);
        let slot = self.bytes.get(start..start.saturating_add(2))?;
        let (lo, hi) = (*slot.first()?, *slot.get(1)?);
        // `at` came from `self.len`, so it necessarily fits.
        self.len = u8::try_from(at).unwrap_or(0);
        Some(u16::from_le_bytes([lo, hi]))
    }

    /// Drop every component.
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// Borrow as a contract path.
    #[must_use]
    pub fn as_path(&self) -> NodePath<'_> {
        let end = self.len().saturating_mul(2);
        NodePath::from_le_bytes(self.bytes.get(..end).unwrap_or(&[]))
    }

    /// The components, from the root downwards.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.as_path().iter()
    }
}

impl Default for NodePathBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for NodePathBuf {
    fn eq(&self, other: &Self) -> bool {
        self.as_path() == other.as_path()
    }
}

impl Eq for NodePathBuf {}

impl fmt::Debug for NodePathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.as_path(), f)
    }
}

impl fmt::Display for NodePathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.as_path(), f)
    }
}

impl<'a> From<&'a NodePathBuf> for NodePath<'a> {
    fn from(value: &'a NodePathBuf) -> Self {
        value.as_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    #[test]
    fn a_new_path_is_the_root() {
        let path = NodePathBuf::new();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
        assert!(path.as_path().is_empty());
        assert_eq!(path.iter().count(), 0);
        assert_eq!(path.to_string(), "/");
        assert_eq!(path, NodePathBuf::default());
    }

    #[test]
    fn pushing_and_popping_mirror_a_descent() {
        let mut path = NodePathBuf::new();
        assert_eq!(path.push(0), Ok(()));
        assert_eq!(path.push(7), Ok(()));
        assert_eq!(path.push(300), Ok(()));

        assert_eq!(path.len(), 3);
        assert_eq!(path.iter().collect::<Vec<_>>(), [0, 7, 300]);
        assert_eq!(path.to_string(), "/0/7/300");

        assert_eq!(path.pop(), Some(300));
        assert_eq!(path.pop(), Some(7));
        assert_eq!(path.len(), 1);
        assert_eq!(path.iter().collect::<Vec<_>>(), [0]);
        assert_eq!(path.pop(), Some(0));
        assert_eq!(path.pop(), None, "popping the root yields nothing");
        assert!(path.is_empty());
    }

    #[test]
    fn a_pushed_path_matches_what_the_contract_expects() {
        let mut path = NodePathBuf::new();
        path.push(1).unwrap();
        path.push(258).unwrap();

        // The contract reads little-endian pairs, so the bytes must line up.
        let expected: Vec<u8> = [1u16, 258].iter().flat_map(|c| c.to_le_bytes()).collect();
        assert_eq!(path.as_path(), NodePath::from_le_bytes(&expected));
        assert_eq!(path.as_path().as_le_bytes(), expected.as_slice());
        assert_eq!(NodePath::from(&path), path.as_path());
    }

    #[test]
    fn every_component_value_round_trips() {
        for component in [0u16, 1, 255, 256, 4095, u16::MAX] {
            let mut path = NodePathBuf::new();
            path.push(component).unwrap();
            assert_eq!(path.iter().next(), Some(component));
            assert_eq!(path.pop(), Some(component));
        }
    }

    #[test]
    fn the_depth_limit_is_enforced_and_leaves_the_path_usable() {
        let mut path = NodePathBuf::new();
        for depth in 0..MAX_DEPTH {
            assert_eq!(path.push(depth as u16), Ok(()), "depth {depth}");
        }
        assert_eq!(path.len(), MAX_DEPTH);

        assert_eq!(path.push(0), Err(PathTooDeep { limit: MAX_DEPTH }));
        assert_eq!(path.len(), MAX_DEPTH, "a refused push changes nothing");
        assert_eq!(path.pop(), Some((MAX_DEPTH - 1) as u16));
        assert_eq!(path.push(1), Ok(()), "usable again after popping");
    }

    #[test]
    fn clearing_returns_to_the_root() {
        let mut path = NodePathBuf::new();
        path.push(1).unwrap();
        path.push(2).unwrap();
        path.clear();
        assert!(path.is_empty());
        assert_eq!(path.pop(), None);
    }

    #[test]
    fn reuse_does_not_leak_stale_components() {
        // The buffer is not zeroed on pop, so a shorter path must not expose
        // what a longer one left behind.
        let mut path = NodePathBuf::new();
        path.push(11).unwrap();
        path.push(22).unwrap();
        path.push(33).unwrap();
        path.pop();
        path.pop();
        path.push(99).unwrap();
        assert_eq!(path.iter().collect::<Vec<_>>(), [11, 99]);
    }

    #[test]
    fn paths_compare_by_components_not_by_buffer() {
        let mut short = NodePathBuf::new();
        short.push(5).unwrap();

        let mut also_short = NodePathBuf::new();
        also_short.push(9).unwrap();
        also_short.pop();
        also_short.push(5).unwrap();

        assert_eq!(short, also_short, "stale buffer bytes must not matter");

        let mut different = NodePathBuf::new();
        different.push(6).unwrap();
        assert_ne!(short, different);
    }

    #[test]
    fn the_error_reports_the_limit() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        let error = PathTooDeep { limit: MAX_DEPTH };
        assert!(error.to_string().contains("64"));
        assert_error(&error);
    }

    #[test]
    fn debug_renders_the_components() {
        let mut path = NodePathBuf::new();
        path.push(4).unwrap();
        path.push(2).unwrap();
        assert_eq!(alloc::format!("{path:?}"), "[4, 2]");
    }
}
