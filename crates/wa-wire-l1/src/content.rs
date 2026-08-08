//! What a decrypted payload says.
//!
//! The other half of L1. [`derive`](crate::derive) reads the stanza; this
//! reads what the stanza's `<enc>` children decrypted to, which is where the
//! message itself lives. Until this existed the boundary carried plaintexts
//! that nothing read.
//!
//! # Written, not generated
//!
//! Everything in [`generated`](crate::generated) comes from whatspec, because
//! whatspec records how WhatsApp Web parses a *stanza* and guessing at that
//! would be guessing at the spec (D-039). It records nothing about the
//! protobuf inside an `<enc>`, so there is no oracle to generate from. The
//! oracle here is whatspec's `WAProto.proto`, which it extracts from the WA Web
//! bundle and pins by SHA-256, so the numbers are generated
//! ([`field`]) and only the rules are written.
//!
//! # Deliberately small
//!
//! `waE2E.Message` has over a hundred variants and dozens of nested types.
//! This does not model them. It answers the two questions a consumer asks
//! before anything else, and answers them for *every* payload:
//!
//! - which kind of message is this, and
//! - what does it say, when it says anything.
//!
//! A variant with no name here still crosses as [`MessageKind::Unmodelled`] with
//! its field number, for the same reason L0 is total: a protocol change must
//! narrow what is understood, never what is delivered.

use wa_wire_proto::{Reader, Value};

// Declared here rather than from `generated/mod.rs`, which is itself emitted by
// a different generator and would drop the line on its next run.
#[path = "generated/content_fields.rs"]
pub mod field;

/// How deep a wrapper chain may go before it is treated as malformed.
///
/// Real traffic wraps twice at the extreme (a view-once inside a device-sent).
/// The bound exists because the wrappers nest by construction, so a crafted
/// payload could otherwise walk as far as it liked.
const MAX_WRAPPERS: usize = 8;

/// Which kind of message a payload carries.
///
/// Named where naming it means something to a consumer, and numbered
/// otherwise. The unnamed case is not a failure: it is a variant this build
/// does not model, crossing intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MessageKind {
    /// Plain text, with no context.
    Conversation,
    /// Text with context: a reply, a link preview, a mention.
    ExtendedText,
    /// An image, possibly captioned.
    Image,
    /// A video, possibly captioned.
    Video,
    /// A document, possibly captioned.
    Document,
    /// Voice or audio.
    Audio,
    /// A sticker.
    Sticker,
    /// A shared contact.
    Contact,
    /// A location.
    Location,
    /// A reaction to another message.
    Reaction,
    /// Protocol machinery: revokes, ephemeral settings, app-state syncs.
    Protocol,
    /// Call signalling carried inside a message.
    Call,
    /// The payload's first field is one this build does not model.
    ///
    /// Carries its number, because that number is how a new variant is
    /// discovered: a consumer counting these learns which fields real traffic
    /// is carrying that nothing here reads yet.
    ///
    /// It may be a message variant or it may be metadata. Telling those apart
    /// needs the whole `waE2E.Message` schema, and claiming to know which
    /// would be claiming more than this reader can see.
    Unmodelled(u32),
    /// The payload carried no fields at all.
    Empty,
}

impl MessageKind {
    /// A stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::ExtendedText => "extended-text",
            Self::Image => "image",
            Self::Video => "video",
            Self::Document => "document",
            Self::Audio => "audio",
            Self::Sticker => "sticker",
            Self::Contact => "contact",
            Self::Location => "location",
            Self::Reaction => "reaction",
            Self::Protocol => "protocol",
            Self::Call => "call",
            Self::Unmodelled(_) => "unmodelled",
            Self::Empty => "empty",
        }
    }

    /// The field number this variant occupies in `waE2E.Message`.
    #[must_use]
    pub const fn number(self) -> Option<u32> {
        Some(match self {
            Self::Conversation => field::CONVERSATION,
            Self::ExtendedText => field::EXTENDED_TEXT,
            Self::Image => field::IMAGE,
            Self::Video => field::VIDEO,
            Self::Document => field::DOCUMENT,
            Self::Audio => field::AUDIO,
            Self::Sticker => field::STICKER,
            Self::Contact => field::CONTACT,
            Self::Location => field::LOCATION,
            Self::Reaction => field::REACTION,
            Self::Protocol => field::PROTOCOL,
            Self::Call => field::CALL,
            Self::Unmodelled(number) => number,
            Self::Empty => return None,
        })
    }

    const fn from_number(number: u32) -> Self {
        match number {
            field::CONVERSATION => Self::Conversation,
            field::EXTENDED_TEXT => Self::ExtendedText,
            field::IMAGE => Self::Image,
            field::VIDEO => Self::Video,
            field::DOCUMENT => Self::Document,
            field::AUDIO => Self::Audio,
            field::STICKER => Self::Sticker,
            field::CONTACT => Self::Contact,
            field::LOCATION => Self::Location,
            field::REACTION => Self::Reaction,
            field::PROTOCOL => Self::Protocol,
            field::CALL => Self::Call,
            other => Self::Unmodelled(other),
        }
    }
}

impl core::fmt::Display for MessageKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unmodelled(number) => write!(f, "unmodelled({number})"),
            other => f.write_str(other.name()),
        }
    }
}

/// What one decrypted payload turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageContent<'a> {
    /// Which variant the payload carried.
    pub kind: MessageKind,
    /// Its human-readable text, when the variant has one.
    ///
    /// A caption counts. A variant with no text has `None`, which is not the
    /// same as an empty string: one means "this kind does not speak", the
    /// other means "it said nothing".
    pub text: Option<&'a str>,
    /// How many wrappers were unwrapped to reach it.
    ///
    /// A device-sent copy of a view-once message is two. Reported rather than
    /// hidden, because a consumer comparing engines wants to know the payload
    /// was nested rather than assume the shapes matched by luck.
    pub wrappers: usize,
}

/// Why a payload could not be read as a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentError {
    /// The protobuf itself is malformed.
    Malformed(wa_wire_proto::Error),
    /// Wrappers nested further than any real message does.
    TooDeeplyWrapped,
    /// A wrapper carried no message inside it.
    EmptyWrapper {
        /// The wrapper's field number.
        number: u32,
    },
}

impl core::fmt::Display for ContentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Malformed(error) => write!(f, "payload is not a readable message: {error}"),
            Self::TooDeeplyWrapped => {
                write!(
                    f,
                    "wrappers nested past {MAX_WRAPPERS}, which no real message does"
                )
            }
            Self::EmptyWrapper { number } => {
                write!(f, "wrapper {number} carried no message")
            }
        }
    }
}

impl core::error::Error for ContentError {}

impl From<wa_wire_proto::Error> for ContentError {
    fn from(error: wa_wire_proto::Error) -> Self {
        Self::Malformed(error)
    }
}

/// Read one decrypted payload as a `waE2E.Message`.
///
/// Unwraps the envelopes WhatsApp puts around a message before reporting the
/// kind, because a consumer asking "what did this say" does not mean "it was a
/// device-sent copy of an ephemeral wrapper".
///
/// # Errors
///
/// [`ContentError`] when the protobuf does not parse or the wrappers nest
/// absurdly. A variant this build does not model is **not** an error.
///
/// ```
/// use wa_wire_l1::content::{MessageKind, derive_content};
///
/// // waE2E.Message { conversation: "hi" }
/// let payload = [0x0a, 0x02, b'h', b'i'];
/// let content = derive_content(&payload)?;
/// assert_eq!(content.kind, MessageKind::Conversation);
/// assert_eq!(content.text, Some("hi"));
/// # Ok::<(), wa_wire_l1::content::ContentError>(())
/// ```
pub fn derive_content(payload: &[u8]) -> Result<MessageContent<'_>, ContentError> {
    let mut body = payload;
    let mut wrappers = 0usize;

    loop {
        let Some(field) = first_variant(body)? else {
            return Ok(MessageContent {
                kind: MessageKind::Empty,
                text: None,
                wrappers,
            });
        };

        // A wrapper is not the message; keep going until something is.
        if let Some(inner) = unwrap(field.number, field.value)? {
            wrappers = wrappers.saturating_add(1);
            if wrappers > MAX_WRAPPERS {
                return Err(ContentError::TooDeeplyWrapped);
            }
            body = inner;
            continue;
        }

        return Ok(MessageContent {
            kind: MessageKind::from_number(field.number),
            text: text_of(field.number, field.value)?,
            wrappers,
        });
    }
}

/// The inner message, if `number` names a wrapper.
fn unwrap(number: u32, value: Value<'_>) -> Result<Option<&[u8]>, ContentError> {
    let inner_field = if number == field::DEVICE_SENT {
        field::DEVICE_SENT_INNER
    } else if is_wrapper(number) {
        field::FUTURE_PROOF_INNER
    } else {
        return Ok(None);
    };

    let Some(message) = value.as_message() else {
        return Err(ContentError::EmptyWrapper { number });
    };
    let inner = message
        .find_last(inner_field)?
        .and_then(Value::as_bytes)
        .ok_or(ContentError::EmptyWrapper { number })?;
    Ok(Some(inner))
}

/// The text a variant carries, if it carries one.
fn text_of(number: u32, value: Value<'_>) -> Result<Option<&str>, ContentError> {
    let inner_field = match number {
        // The variant *is* the text.
        field::CONVERSATION => return Ok(value.as_str()),
        field::EXTENDED_TEXT => field::EXTENDED_TEXT_TEXT,
        field::REACTION => field::REACTION_TEXT,
        field::IMAGE => field::IMAGE_TEXT,
        field::VIDEO => field::VIDEO_TEXT,
        field::DOCUMENT => field::DOCUMENT_TEXT,
        _ => return Ok(None),
    };
    let Some(message) = value.as_message() else {
        return Ok(None);
    };
    Ok(message.find_last(inner_field)?.and_then(Value::as_str))
}

/// The first field that names a variant or a wrapper.
///
/// A `oneof` in practice: `waE2E.Message` does not declare one, but exactly one
/// variant is set, so the first recognised field is the message.
///
/// A predicate rather than a list of numbers, because a list is a second place
/// to add a variant and therefore a place to forget one.
fn first_variant(body: &[u8]) -> Result<Option<wa_wire_proto::Field<'_>>, ContentError> {
    let mut reader = Reader::new(body);
    let mut first = None;
    while let Some(field) = reader.next() {
        let field = field?;
        if is_variant(field.number) {
            return Ok(Some(field));
        }
        // Kept in case nothing is recognised: reporting the number that *is*
        // there beats reporting an empty payload, which is a different fact.
        first.get_or_insert(field);
    }
    Ok(first)
}

/// Whether `number` names a message variant or one of the wrappers.
fn is_variant(number: u32) -> bool {
    if is_wrapper(number) {
        return true;
    }
    matches!(
        MessageKind::from_number(number),
        MessageKind::Conversation
            | MessageKind::ExtendedText
            | MessageKind::Image
            | MessageKind::Video
            | MessageKind::Document
            | MessageKind::Audio
            | MessageKind::Sticker
            | MessageKind::Contact
            | MessageKind::Location
            | MessageKind::Reaction
            | MessageKind::Protocol
            | MessageKind::Call
    )
}

/// Whether `number` names an envelope that holds another message.
///
/// The list is generated from the spec by type, so a wrapper WhatsApp adds
/// arrives without anyone remembering to add it here. Writing it by hand cost
/// 22 of the 29 the spec actually has.
fn is_wrapper(number: u32) -> bool {
    field::WRAPPERS.contains(&number)
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
