//! Tag bytes and the token dictionaries they index into.
//!
//! The table is a parameter rather than a constant. WhatsApp's dictionaries
//! move with the client version, and RFC-009 makes that a matter of
//! provenance, not of contract version — so a host generated from a different
//! `whatspec` build supplies its own table instead of needing a different
//! parser.

/// An empty value, and the terminator for an absent node body.
pub const LIST_EMPTY: u8 = 0;
/// A bare stream-end frame.
pub const STREAM_END: u8 = 2;

/// First double-byte dictionary tag.
pub const DICTIONARY_0: u8 = 236;
/// Last double-byte dictionary tag.
pub const DICTIONARY_3: u8 = 239;

/// An interop JID.
pub const JID_INTEROP: u8 = 245;
/// A Messenger JID.
pub const JID_FB: u8 = 246;
/// A user JID with an explicit domain type and device.
pub const JID_USER: u8 = 247;
/// A list whose length fits in one byte.
pub const LIST_8: u8 = 248;
/// A list whose length needs two bytes.
pub const LIST_16: u8 = 249;
/// A `user@server` JID.
pub const JID_PAIR: u8 = 250;
/// A run of packed hexadecimal digits.
pub const HEX_8: u8 = 251;
/// Bytes with an 8-bit length.
pub const BINARY_8: u8 = 252;
/// Bytes with a 20-bit length.
pub const BINARY_20: u8 = 253;
/// Bytes with a 32-bit length.
pub const BINARY_32: u8 = 254;
/// A run of packed nibble digits.
pub const NIBBLE_8: u8 = 255;

/// Domain type byte: a phone-number JID.
pub const DOMAIN_TYPE_PN: u8 = 0x00;
/// Domain type byte: a LID JID.
pub const DOMAIN_TYPE_LID: u8 = 0x01;
/// Domain type byte: a hosted LID JID.
pub const DOMAIN_TYPE_HOSTED_LID: u8 = 0x81;
/// Mask selecting the hosted bit of a domain type.
pub const DOMAIN_TYPE_HOSTED_MASK: u8 = 0x80;
/// Mask selecting the LID bit of a domain type.
pub const DOMAIN_TYPE_LID_MASK: u8 = 0x01;

/// The default server for phone-number JIDs.
pub const SERVER_PN: &str = "s.whatsapp.net";
/// The server for LID JIDs.
pub const SERVER_LID: &str = "lid";
/// The server for hosted phone-number JIDs.
pub const SERVER_HOSTED: &str = "hosted";
/// The server for hosted LID JIDs.
pub const SERVER_HOSTED_LID: &str = "hosted.lid";
/// The server for interop JIDs.
pub const SERVER_INTEROP: &str = "interop";
/// The server for Messenger JIDs.
pub const SERVER_MSGR: &str = "msgr";

/// Digits a packed nibble run can encode. The last four slots are unassigned
/// and decode to the replacement character rather than failing, matching what
/// every engine already does.
pub const NIBBLE_ALPHABET: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '.', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}',
    '\u{FFFD}',
];

/// Digits a packed hex run can encode.
pub const HEX_ALPHABET: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

/// The token dictionaries a frame's tags index into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenTable<'a> {
    single_byte: &'a [&'a str],
    dictionaries: &'a [&'a [&'a str]],
}

impl<'a> TokenTable<'a> {
    /// Build a table from a single-byte list and the double-byte dictionaries.
    #[must_use]
    pub const fn new(single_byte: &'a [&'a str], dictionaries: &'a [&'a [&'a str]]) -> Self {
        Self {
            single_byte,
            dictionaries,
        }
    }

    /// An empty table. Every token lookup fails, which is what a host wants
    /// while it is still deciding which spec build to load.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            single_byte: &[],
            dictionaries: &[],
        }
    }

    /// The single-byte token for `tag`.
    ///
    /// The tag byte *is* the index. Slot 0 is never a real token — that byte
    /// means `LIST_EMPTY` on the wire — so a table carries a placeholder there
    /// rather than shifting everything by one. Getting this off by one silently
    /// resolves every token to its neighbour, so it is pinned by a test against
    /// the engine's own table.
    #[must_use]
    pub fn single_byte(&self, tag: u8) -> Option<&'a str> {
        self.single_byte.get(usize::from(tag)).copied()
    }

    /// The token at `index` of dictionary `dictionary`.
    #[must_use]
    pub fn dictionary(&self, dictionary: u8, index: u8) -> Option<&'a str> {
        let dictionary = self.dictionaries.get(usize::from(dictionary))?;
        dictionary.get(usize::from(index)).copied()
    }

    /// How many single-byte tokens the table holds.
    #[must_use]
    pub const fn single_byte_len(&self) -> usize {
        self.single_byte.len()
    }

    /// How many double-byte dictionaries the table holds.
    #[must_use]
    pub const fn dictionary_count(&self) -> usize {
        self.dictionaries.len()
    }
}

impl Default for TokenTable<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static SINGLE: [&str; 4] = ["<none>", "alpha", "beta", "gamma"];
    static DICT_A: [&str; 2] = ["one", "two"];
    static DICT_B: [&str; 1] = ["three"];
    static DICTS: [&[&str]; 2] = [&DICT_A, &DICT_B];

    fn table() -> TokenTable<'static> {
        TokenTable::new(&SINGLE, &DICTS)
    }

    #[test]
    fn the_tag_byte_indexes_the_table_directly() {
        let table = table();
        // Slot 0 is the placeholder for LIST_EMPTY; the engine's own table
        // carries one, and shifting instead would resolve every token to its
        // neighbour.
        assert_eq!(table.single_byte(0), Some("<none>"));
        assert_eq!(table.single_byte(1), Some("alpha"));
        assert_eq!(table.single_byte(2), Some("beta"));
        assert_eq!(table.single_byte(3), Some("gamma"));
        assert_eq!(table.single_byte(4), None);
        assert_eq!(table.single_byte(u8::MAX), None);
    }

    #[test]
    fn dictionary_lookups_are_bounded_on_both_axes() {
        let table = table();
        assert_eq!(table.dictionary(0, 0), Some("one"));
        assert_eq!(table.dictionary(0, 1), Some("two"));
        assert_eq!(table.dictionary(1, 0), Some("three"));

        assert_eq!(table.dictionary(0, 2), None, "index past the dictionary");
        assert_eq!(table.dictionary(1, 1), None);
        assert_eq!(table.dictionary(2, 0), None, "no such dictionary");
        assert_eq!(table.dictionary(u8::MAX, u8::MAX), None);
    }

    #[test]
    fn sizes_are_reported() {
        let table = table();
        assert_eq!(table.single_byte_len(), 4);
        assert_eq!(table.dictionary_count(), 2);
    }

    #[test]
    fn the_empty_table_resolves_nothing() {
        let empty = TokenTable::empty();
        assert_eq!(empty.single_byte_len(), 0);
        assert_eq!(empty.dictionary_count(), 0);
        assert_eq!(empty.single_byte(0), None);
        assert_eq!(empty.single_byte(1), None);
        assert_eq!(empty.dictionary(0, 0), None);
        assert_eq!(empty, TokenTable::default());
    }

    #[test]
    fn tag_constants_are_pinned() {
        // These are the wire; changing one breaks every frame.
        assert_eq!(LIST_EMPTY, 0);
        assert_eq!(STREAM_END, 2);
        assert_eq!(DICTIONARY_0, 236);
        assert_eq!(DICTIONARY_3, 239);
        assert_eq!(JID_INTEROP, 245);
        assert_eq!(JID_FB, 246);
        assert_eq!(JID_USER, 247);
        assert_eq!(LIST_8, 248);
        assert_eq!(LIST_16, 249);
        assert_eq!(JID_PAIR, 250);
        assert_eq!(HEX_8, 251);
        assert_eq!(BINARY_8, 252);
        assert_eq!(BINARY_20, 253);
        assert_eq!(BINARY_32, 254);
        assert_eq!(NIBBLE_8, 255);
        assert_eq!(usize::from(DICTIONARY_3 - DICTIONARY_0) + 1, 4);
    }

    #[test]
    fn alphabets_cover_a_full_nibble() {
        assert_eq!(NIBBLE_ALPHABET.len(), 16);
        assert_eq!(HEX_ALPHABET.len(), 16);
        assert_eq!(
            &NIBBLE_ALPHABET[..12],
            &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '-', '.']
        );
        assert!(NIBBLE_ALPHABET[12..].iter().all(|c| *c == '\u{FFFD}'));
        assert_eq!(HEX_ALPHABET[10], 'A');
        assert_eq!(HEX_ALPHABET[15], 'F');
    }

    #[test]
    fn domain_type_masks_classify_correctly() {
        assert_eq!(DOMAIN_TYPE_PN & DOMAIN_TYPE_HOSTED_MASK, 0);
        assert_eq!(DOMAIN_TYPE_LID & DOMAIN_TYPE_LID_MASK, 1);
        assert_ne!(DOMAIN_TYPE_HOSTED_LID & DOMAIN_TYPE_HOSTED_MASK, 0);
        assert_ne!(DOMAIN_TYPE_HOSTED_LID & DOMAIN_TYPE_LID_MASK, 0);
    }
}
