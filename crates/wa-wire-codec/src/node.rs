//! Parsing a frame into a navigable node tree.
//!
//! A node is `[list tag][size][tag][attr pairs...][content?]`. The size counts
//! every slot after the list header, so an even remainder means attributes
//! only and an odd one means a trailing body.
//!
//! Nothing is copied and nothing is allocated. A [`NodeRef`] holds the slice
//! starting at its own list tag and re-walks it on demand; the encoding is
//! self-delimiting, so a node does not need to know where it ends. Parsing
//! validates the whole tree up front, which is what makes every accessor
//! afterwards infallible.

use crate::error::ParseError;
use crate::jid::{Jid, User};
use crate::packed::{Alphabet, Packed};
use crate::reader::Reader;
use crate::token::{
    self, BINARY_8, BINARY_20, BINARY_32, DICTIONARY_0, DICTIONARY_3, HEX_8, JID_FB, JID_INTEROP,
    JID_PAIR, JID_USER, LIST_8, LIST_16, LIST_EMPTY, NIBBLE_8, TokenTable,
};
use crate::value::Value;

/// How deep a frame may nest before the parser refuses it.
///
/// Real stanzas reach depth 9 at the extreme, so this is generous. It exists to
/// bound recursion on a hostile frame, not to constrain real ones.
pub const DEFAULT_MAX_DEPTH: usize = 64;

/// What a node carries after its attributes.
///
/// Not `Copy`: the child variant holds an iterator, and a silently duplicated
/// iterator is a bug waiting to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content<'a> {
    /// No body at all.
    None,
    /// A scalar body.
    Value(Value<'a>),
    /// Child nodes.
    Children(Children<'a>),
}

impl<'a> Content<'a> {
    /// Whether the node has no body.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// The scalar body, if there is one.
    #[must_use]
    pub const fn as_value(&self) -> Option<Value<'a>> {
        match self {
            Self::Value(value) => Some(*value),
            _ => None,
        }
    }

    /// The body's bytes, when it is a raw payload — the `<enc>` case.
    #[must_use]
    pub const fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Self::Value(value) => value.as_bytes(),
            _ => None,
        }
    }

    /// The children, if the body is a list.
    #[must_use]
    pub fn as_children(self) -> Option<Children<'a>> {
        match self {
            Self::Children(children) => Some(children),
            _ => None,
        }
    }
}

/// A node, borrowing from the frame it was parsed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRef<'a> {
    table: TokenTable<'a>,
    /// From this node's list tag to the end of the buffer. The encoding is
    /// self-delimiting, so the tail beyond this node is harmless.
    bytes: &'a [u8],
    tag: Value<'a>,
    attr_count: usize,
    has_content: bool,
    depth_budget: usize,
}

impl<'a> NodeRef<'a> {
    /// The node's tag.
    #[must_use]
    pub const fn tag(&self) -> Value<'a> {
        self.tag
    }

    /// How many attributes the node carries.
    #[must_use]
    pub const fn attr_count(&self) -> usize {
        self.attr_count
    }

    /// Whether the node has a body.
    #[must_use]
    pub const fn has_content(&self) -> bool {
        self.has_content
    }

    /// The attributes, in the order the frame carried them.
    #[must_use]
    pub fn attrs(&self) -> Attrs<'a> {
        let mut reader = Reader::new(self.bytes);
        // The tree was validated at parse time, so re-walking the header
        // cannot fail. A zeroed count on the impossible branch yields an empty
        // iterator rather than a panic.
        let remaining = skip_header(&mut reader, self.table).map_or(0, |()| self.attr_count);
        Attrs {
            table: self.table,
            reader,
            remaining,
        }
    }

    /// The first value for `key`.
    ///
    /// Attribute keys are tokens or short strings, so a linear scan over the
    /// handful a stanza carries beats building an index.
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<Value<'a>> {
        self.attrs()
            .find(|(name, _)| name.eq_str(key))
            .map(|(_, value)| value)
    }

    /// Whether an attribute equals `expected` — the comparison L1 derivation
    /// leans on, done without allocating.
    #[must_use]
    pub fn attr_eq(&self, key: &str, expected: &str) -> bool {
        self.attr(key).is_some_and(|value| value.eq_str(expected))
    }

    /// The node's body.
    #[must_use]
    pub fn content(&self) -> Content<'a> {
        self.content_inner().unwrap_or(Content::None)
    }

    fn content_inner(&self) -> Option<Content<'a>> {
        if !self.has_content {
            return Some(Content::None);
        }
        let mut reader = Reader::new(self.bytes);
        skip_header(&mut reader, self.table).ok()?;
        for _ in 0..self.attr_count {
            skip_value(&mut reader, self.table).ok()?;
            skip_value(&mut reader, self.table).ok()?;
        }
        parse_content(&mut reader, self.table, self.depth_budget).ok()
    }

    /// The child nodes, or an empty iterator when the body is not a list.
    #[must_use]
    pub fn children(&self) -> Children<'a> {
        self.content().as_children().unwrap_or(Children::empty())
    }

    /// The child at `index`.
    #[must_use]
    pub fn child_at(&self, index: usize) -> Option<NodeRef<'a>> {
        self.children().nth(index)
    }

    /// The first child whose tag is `tag`.
    #[must_use]
    pub fn child(&self, tag: &str) -> Option<NodeRef<'a>> {
        self.children().find(|child| child.tag().eq_str(tag))
    }

    /// Follow a path of child indices from this node.
    ///
    /// This is how an envelope's plaintext entries find the node they belong
    /// to: the path in a [`PlaintextEntry`] addresses exactly this walk.
    ///
    /// [`PlaintextEntry`]: https://docs.rs/wa-wire-contract
    #[must_use]
    pub fn at_path<I>(&self, path: I) -> Option<NodeRef<'a>>
    where
        I: IntoIterator<Item = u16>,
    {
        let mut node = *self;
        for component in path {
            node = node.child_at(usize::from(component))?;
        }
        Some(node)
    }
}

/// Iterator over a node's attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attrs<'a> {
    table: TokenTable<'a>,
    reader: Reader<'a>,
    remaining: usize,
}

impl<'a> Iterator for Attrs<'a> {
    type Item = (Value<'a>, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.checked_sub(1)?;
        // Validated at parse time; the `ok()` arms are unreachable from a
        // parsed tree and are exercised directly in this module's tests.
        let key = parse_value_next(&mut self.reader, self.table).ok()?;
        let value = parse_value_next(&mut self.reader, self.table).ok()?;
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Attrs<'_> {}

/// Iterator over a node's children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Children<'a> {
    table: TokenTable<'a>,
    reader: Reader<'a>,
    remaining: usize,
    depth_budget: usize,
}

impl Children<'_> {
    const fn empty() -> Self {
        Self {
            table: TokenTable::empty(),
            reader: Reader::new(&[]),
            remaining: 0,
            depth_budget: 0,
        }
    }

    /// How many children are left.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.remaining
    }

    /// Whether there are no children left.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.remaining == 0
    }
}

impl<'a> Iterator for Children<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.remaining = self.remaining.checked_sub(1)?;
        let node = parse_node(&mut self.reader, self.table, self.depth_budget).ok()?;
        // Skipping past the child leaves the cursor on its sibling.
        skip_node_body(&mut self.reader, self.table, &node).ok()?;
        Some(node)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Children<'_> {}

/// Parses frames into node trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parser<'a> {
    table: TokenTable<'a>,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    /// A parser over `table`, at the default depth limit.
    #[must_use]
    pub const fn new(table: TokenTable<'a>) -> Self {
        Self {
            table,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// Set how deep a frame may nest.
    #[must_use]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// The token table in use.
    #[must_use]
    pub const fn table(&self) -> TokenTable<'a> {
        self.table
    }

    /// The depth limit in use.
    #[must_use]
    pub const fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Parse one node from `frame`, requiring it to consume the whole buffer.
    ///
    /// The whole tree is validated here, so every accessor on the result is
    /// infallible.
    pub fn parse(&self, frame: &'a [u8]) -> Result<NodeRef<'a>, ParseError> {
        let mut reader = Reader::new(frame);
        let node = parse_node(&mut reader, self.table, self.max_depth)?;
        skip_node_body(&mut reader, self.table, &node)?;
        if !reader.is_empty() {
            return Err(ParseError::TrailingBytes(reader.remaining()));
        }
        Ok(node)
    }

    /// Parse the first node in `frame`, allowing bytes to follow.
    ///
    /// Useful when a caller frames several nodes back to back and wants to
    /// handle the remainder itself.
    pub fn parse_prefix(&self, frame: &'a [u8]) -> Result<(NodeRef<'a>, &'a [u8]), ParseError> {
        let mut reader = Reader::new(frame);
        let node = parse_node(&mut reader, self.table, self.max_depth)?;
        skip_node_body(&mut reader, self.table, &node)?;
        Ok((node, reader.tail()))
    }
}

// ---------------------------------------------------------------------------
// Parsing primitives
// ---------------------------------------------------------------------------

/// Read a node's list header and tag, leaving the cursor on its attributes.
fn parse_node<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
    depth_budget: usize,
) -> Result<NodeRef<'a>, ParseError> {
    let depth_budget = depth_budget
        .checked_sub(1)
        .ok_or(ParseError::DepthLimitExceeded {
            limit: DEFAULT_MAX_DEPTH,
        })?;

    let bytes = reader.tail();
    let size = parse_list_size(reader)?;
    let slots = size.checked_sub(1).ok_or(ParseError::EmptyNode)?;

    let tag = parse_value_next(reader, table)?;
    if !tag.is_textual() {
        return Err(ParseError::NonStringTag);
    }

    let attr_count = slots / 2;
    let has_content = slots % 2 == 1;

    Ok(NodeRef {
        table,
        bytes,
        tag,
        attr_count,
        has_content,
        depth_budget,
    })
}

/// Consume a node's attributes and body, leaving the cursor on its sibling.
fn skip_node_body<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
    node: &NodeRef<'a>,
) -> Result<(), ParseError> {
    for _ in 0..node.attr_count {
        let key = parse_value_next(reader, table)?;
        let value = parse_value_next(reader, table)?;
        if !key.is_textual() || !value.is_textual() {
            return Err(ParseError::NonStringAttr);
        }
    }
    if node.has_content {
        match parse_content(reader, table, node.depth_budget)? {
            Content::Children(children) => {
                // Walked here rather than through `Children`'s iterator so a
                // malformed child is reported instead of ending iteration.
                let mut cursor = children.reader;
                for _ in 0..children.remaining {
                    let child = parse_node(&mut cursor, table, children.depth_budget)?;
                    skip_node_body(&mut cursor, table, &child)?;
                }
                *reader = cursor;
            }
            Content::None | Content::Value(_) => {}
        }
    }
    Ok(())
}

/// Re-read a node's list header and tag without producing them.
fn skip_header<'a>(reader: &mut Reader<'a>, table: TokenTable<'a>) -> Result<(), ParseError> {
    parse_list_size(reader)?;
    parse_value_next(reader, table)?;
    Ok(())
}

fn skip_value<'a>(reader: &mut Reader<'a>, table: TokenTable<'a>) -> Result<(), ParseError> {
    parse_value_next(reader, table)?;
    Ok(())
}

fn parse_list_size(reader: &mut Reader<'_>) -> Result<usize, ParseError> {
    let tag = reader.u8()?;
    match tag {
        LIST_8 => Ok(usize::from(reader.u8()?)),
        LIST_16 => Ok(usize::from(reader.u16()?)),
        found => Err(ParseError::ExpectedList { found }),
    }
}

fn parse_content<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
    depth_budget: usize,
) -> Result<Content<'a>, ParseError> {
    let tag = reader.u8()?;
    match tag {
        LIST_EMPTY => Ok(Content::None),
        LIST_8 | LIST_16 => {
            let count = if tag == LIST_8 {
                usize::from(reader.u8()?)
            } else {
                usize::from(reader.u16()?)
            };
            Ok(Content::Children(Children {
                table,
                reader: *reader,
                remaining: count,
                depth_budget,
            }))
        }
        other => Ok(Content::Value(parse_value(reader, table, other)?)),
    }
}

fn parse_value_next<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<Value<'a>, ParseError> {
    let tag = reader.u8()?;
    parse_value(reader, table, tag)
}

fn parse_value<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
    tag: u8,
) -> Result<Value<'a>, ParseError> {
    match tag {
        LIST_EMPTY => Ok(Value::Nil),
        JID_PAIR => Ok(Value::Jid(parse_jid_pair(reader, table)?)),
        JID_USER => Ok(Value::Jid(parse_jid_user(reader, table)?)),
        JID_INTEROP => Ok(Value::Jid(parse_jid_interop(reader, table)?)),
        JID_FB => Ok(Value::Jid(parse_jid_messenger(reader, table)?)),
        BINARY_8 | BINARY_20 | BINARY_32 => Ok(Value::Bytes(parse_binary(reader, tag)?)),
        NIBBLE_8 => Ok(Value::Packed(parse_packed(reader, Alphabet::Nibble)?)),
        HEX_8 => Ok(Value::Packed(parse_packed(reader, Alphabet::Hex)?)),
        _ => Ok(Value::Token(parse_token(reader, table, tag)?)),
    }
}

fn parse_token<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
    tag: u8,
) -> Result<&'a str, ParseError> {
    if (DICTIONARY_0..=DICTIONARY_3).contains(&tag) {
        let dictionary = tag.wrapping_sub(DICTIONARY_0);
        let index = reader.u8()?;
        return table
            .dictionary(dictionary, index)
            .ok_or(ParseError::UnknownToken {
                dictionary: Some(dictionary),
                index: u16::from(index),
            });
    }
    table.single_byte(tag).ok_or(ParseError::UnknownToken {
        dictionary: None,
        index: u16::from(tag),
    })
}

fn parse_binary<'a>(reader: &mut Reader<'a>, tag: u8) -> Result<&'a [u8], ParseError> {
    let length = match tag {
        BINARY_8 => u32::from(reader.u8()?),
        BINARY_20 => reader.u20()?,
        // `parse_value` only routes the three binary tags here.
        _ => reader.u32()?,
    };
    reader.take(length as usize)
}

fn parse_packed<'a>(reader: &mut Reader<'a>, alphabet: Alphabet) -> Result<Packed<'a>, ParseError> {
    let length_byte = reader.u8()?;
    let (count, odd) = Packed::split_length_byte(length_byte)?;
    let bytes = reader.take(count)?;
    Ok(Packed::new(alphabet, bytes, odd))
}

/// A JID's user slot, which may be absent.
fn parse_jid_user_part<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<User<'a>, ParseError> {
    let tag = reader.u8()?;
    match parse_value(reader, table, tag)? {
        Value::Nil => Ok(User::None),
        Value::Token(token) => Ok(User::Token(token)),
        Value::Packed(packed) => Ok(User::Packed(packed)),
        Value::Bytes(bytes) => Ok(User::Bytes(bytes)),
        Value::Jid(_) => Err(ParseError::UnexpectedTag { tag }),
    }
}

fn parse_jid_pair<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<Jid<'a>, ParseError> {
    let user = parse_jid_user_part(reader, table)?;
    let server_tag = reader.u8()?;
    let server = parse_token(reader, table, server_tag)?;
    Ok(Jid::pair(user, server))
}

fn parse_jid_user<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<Jid<'a>, ParseError> {
    let domain_type = reader.u8()?;
    let device = u16::from(reader.u8()?);
    let user = parse_jid_user_part(reader, table)?;
    Ok(Jid::with_device(user, server_for(domain_type), device))
}

const fn server_for(domain_type: u8) -> &'static str {
    if domain_type == token::DOMAIN_TYPE_LID {
        token::SERVER_LID
    } else if domain_type == token::DOMAIN_TYPE_HOSTED_LID {
        token::SERVER_HOSTED_LID
    } else if domain_type & token::DOMAIN_TYPE_HOSTED_MASK != 0
        && domain_type & token::DOMAIN_TYPE_LID_MASK == 0
    {
        token::SERVER_HOSTED
    } else {
        token::SERVER_PN
    }
}

fn parse_jid_interop<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<Jid<'a>, ParseError> {
    let user = parse_jid_user_part(reader, table)?;
    let device = reader.u16()?;
    let integrator = reader.u16()?;
    // No trailing server token, unlike the Messenger form right below. The
    // client writes `tag, user, u16 device, u16 integrator` and stops
    // (`WA/Wap.js`, the `JID_INTEROP` arm); the Messenger arm beside it does
    // write one. Consuming a token that is not there swallows the next
    // attribute's key and desynchronises the rest of the frame.
    Ok(Jid::interop(user, device, integrator))
}

fn parse_jid_messenger<'a>(
    reader: &mut Reader<'a>,
    table: TokenTable<'a>,
) -> Result<Jid<'a>, ParseError> {
    let user = parse_jid_user_part(reader, table)?;
    let device = reader.u16()?;
    let server_tag = reader.u8()?;
    parse_token(reader, table, server_tag)?;
    Ok(Jid::messenger(user, device))
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
