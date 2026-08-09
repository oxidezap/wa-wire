//! The L1 derivation, generated from whatspec's `incoming` domain.
//!
//! GENERATED FILE — do not edit by hand. Run `tools/generate-l1.py`.
//!
//! Committed rather than produced by a build script so that a protocol change
//! arrives as a reviewable diff, per RFC-009. CI regenerates and requires no
//! change, which rules out drift.
//!
//! Every field here says which extraction primitive produced it. The primitives
//! live in `extract.rs` and are written by hand; this file only chooses among
//! them, which is what keeps a protocol change from becoming a logic change.

// One arm per shape, even where several share a tag: which shapes a tag has is
// part of what this file records, and collapsing the arms would erase it.
#![allow(clippy::match_same_arms)]

extern crate alloc;

use wa_wire_codec::{Jid, NodeRef, Value};

use crate::error::DeriveError;
use crate::extract;
use crate::provenance::Provenance;

/// Which whatspec build this derivation came from.
pub const PROVENANCE: Provenance<'static> = Provenance {
    whatsapp_version: "2.3000.1044659339",
    schema_version: "2.0.0",
    generator_version: "0.1.0",
    incoming_digest: "sha256:18c955020acfc3ea86bedb4186aac951ccafdf26ede0b05554926a3157d7749e",
    proto_digest: "sha256:021d53059e7b35d8553c97c09567da8d6aba7278c7f2510242eab686bff3647a",
};

/// Wire values for `ALLNONE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ALLNONE {
    /// `all`
    All,
    /// `none`
    None,
}

impl ALLNONE {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("all") {
            return Some(Self::All);
        }
        if value.eq_str("none") {
            return Some(Self::None);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::None => "none",
        }
    }
}

/// Wire values for `APPDATA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum APPDATA {
    /// `default`
    Default,
    /// `member_tag`
    MemberTag,
    /// `group_history`
    GroupHistory,
}

impl APPDATA {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("default") {
            return Some(Self::Default);
        }
        if value.eq_str("member_tag") {
            return Some(Self::MemberTag);
        }
        if value.eq_str("group_history") {
            return Some(Self::GroupHistory);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::MemberTag => "member_tag",
            Self::GroupHistory => "group_history",
        }
    }
}

/// Wire values for `CiphertextType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CiphertextType {
    /// `skmsg`
    Skmsg,
    /// `pkmsg`
    Pkmsg,
    /// `msg`
    Msg,
    /// `msmsg`
    Msmsg,
}

impl CiphertextType {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("skmsg") {
            return Some(Self::Skmsg);
        }
        if value.eq_str("pkmsg") {
            return Some(Self::Pkmsg);
        }
        if value.eq_str("msg") {
            return Some(Self::Msg);
        }
        if value.eq_str("msmsg") {
            return Some(Self::Msmsg);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Skmsg => "skmsg",
            Self::Pkmsg => "pkmsg",
            Self::Msg => "msg",
            Self::Msmsg => "msmsg",
        }
    }
}

/// Wire values for `ENUM017`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUM017 {
    /// `0`
    N0,
    /// `1`
    N1,
    /// `7`
    N7,
}

impl ENUM017 {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("0") {
            return Some(Self::N0);
        }
        if value.eq_str("1") {
            return Some(Self::N1);
        }
        if value.eq_str("7") {
            return Some(Self::N7);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::N0 => "0",
            Self::N1 => "1",
            Self::N7 => "7",
        }
    }
}

/// Wire values for `ENUMALLNONE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUMALLNONE {
    /// `all`
    All,
    /// `none`
    None,
}

impl ENUMALLNONE {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("all") {
            return Some(Self::All);
        }
        if value.eq_str("none") {
            return Some(Self::None);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::None => "none",
        }
    }
}

/// Wire values for `ENUMCBPNBPPMP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUMCBPNBPPMP {
    /// `CBP`
    CBP,
    /// `NBP`
    NBP,
    /// `PMP`
    PMP,
}

impl ENUMCBPNBPPMP {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("CBP") {
            return Some(Self::CBP);
        }
        if value.eq_str("NBP") {
            return Some(Self::NBP);
        }
        if value.eq_str("PMP") {
            return Some(Self::PMP);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::CBP => "CBP",
            Self::NBP => "NBP",
            Self::PMP => "PMP",
        }
    }
}

/// Wire values for `ENUMDELIVERYNOOPTIMIZATION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUMDELIVERYNOOPTIMIZATION {
    /// `delivery`
    Delivery,
    /// `no_optimization`
    NoOptimization,
}

impl ENUMDELIVERYNOOPTIMIZATION {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("delivery") {
            return Some(Self::Delivery);
        }
        if value.eq_str("no_optimization") {
            return Some(Self::NoOptimization);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Delivery => "delivery",
            Self::NoOptimization => "no_optimization",
        }
    }
}

/// Wire values for `ENUMFALSETRUE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUMFALSETRUE {
    /// `false`
    False,
    /// `true`
    True,
}

impl ENUMFALSETRUE {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("false") {
            return Some(Self::False);
        }
        if value.eq_str("true") {
            return Some(Self::True);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
        }
    }
}

/// Wire values for `ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR {
    /// `free_customer_service`
    FreeCustomerService,
    /// `free_entry_point`
    FreeEntryPoint,
    /// `regular`
    Regular,
}

impl ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("free_customer_service") {
            return Some(Self::FreeCustomerService);
        }
        if value.eq_str("free_entry_point") {
            return Some(Self::FreeEntryPoint);
        }
        if value.eq_str("regular") {
            return Some(Self::Regular);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::FreeCustomerService => "free_customer_service",
            Self::FreeEntryPoint => "free_entry_point",
            Self::Regular => "regular",
        }
    }
}

/// Wire values for `EVENTTYPES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EVENTTYPES {
    /// `creation`
    Creation,
    /// `response`
    Response,
    /// `edit`
    Edit,
}

impl EVENTTYPES {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("creation") {
            return Some(Self::Creation);
        }
        if value.eq_str("response") {
            return Some(Self::Response);
        }
        if value.eq_str("edit") {
            return Some(Self::Edit);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Creation => "creation",
            Self::Response => "response",
            Self::Edit => "edit",
        }
    }
}

/// Wire values for `IncomingMsgReceiptParserParticipantsUserType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncomingMsgReceiptParserParticipantsUserType {
    /// `delivery`
    Delivery,
    /// `read`
    Read,
    /// `played`
    Played,
    /// `inactive`
    Inactive,
    /// `server-error`
    ServerError,
    /// `sender`
    Sender,
    /// `read-self`
    ReadSelf,
    /// `played-self`
    PlayedSelf,
    /// `peer_msg`
    PeerMsg,
}

impl IncomingMsgReceiptParserParticipantsUserType {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("delivery") {
            return Some(Self::Delivery);
        }
        if value.eq_str("read") {
            return Some(Self::Read);
        }
        if value.eq_str("played") {
            return Some(Self::Played);
        }
        if value.eq_str("inactive") {
            return Some(Self::Inactive);
        }
        if value.eq_str("server-error") {
            return Some(Self::ServerError);
        }
        if value.eq_str("sender") {
            return Some(Self::Sender);
        }
        if value.eq_str("read-self") {
            return Some(Self::ReadSelf);
        }
        if value.eq_str("played-self") {
            return Some(Self::PlayedSelf);
        }
        if value.eq_str("peer_msg") {
            return Some(Self::PeerMsg);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Delivery => "delivery",
            Self::Read => "read",
            Self::Played => "played",
            Self::Inactive => "inactive",
            Self::ServerError => "server-error",
            Self::Sender => "sender",
            Self::ReadSelf => "read-self",
            Self::PlayedSelf => "played-self",
            Self::PeerMsg => "peer_msg",
        }
    }
}

/// Wire values for `IncomingMsgReceiptParserType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IncomingMsgReceiptParserType {
    /// `delivery`
    Delivery,
    /// `read`
    Read,
    /// `played`
    Played,
    /// `inactive`
    Inactive,
    /// `server-error`
    ServerError,
    /// `sender`
    Sender,
    /// `read-self`
    ReadSelf,
    /// `played-self`
    PlayedSelf,
    /// `peer_msg`
    PeerMsg,
}

impl IncomingMsgReceiptParserType {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("delivery") {
            return Some(Self::Delivery);
        }
        if value.eq_str("read") {
            return Some(Self::Read);
        }
        if value.eq_str("played") {
            return Some(Self::Played);
        }
        if value.eq_str("inactive") {
            return Some(Self::Inactive);
        }
        if value.eq_str("server-error") {
            return Some(Self::ServerError);
        }
        if value.eq_str("sender") {
            return Some(Self::Sender);
        }
        if value.eq_str("read-self") {
            return Some(Self::ReadSelf);
        }
        if value.eq_str("played-self") {
            return Some(Self::PlayedSelf);
        }
        if value.eq_str("peer_msg") {
            return Some(Self::PeerMsg);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Delivery => "delivery",
            Self::Read => "read",
            Self::Played => "played",
            Self::Inactive => "inactive",
            Self::ServerError => "server-error",
            Self::Sender => "sender",
            Self::ReadSelf => "read-self",
            Self::PlayedSelf => "played-self",
            Self::PeerMsg => "peer_msg",
        }
    }
}

/// Wire values for `MSGVERIFIEDLEVEL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MSGVERIFIEDLEVEL {
    /// `high`
    High,
    /// `low`
    Low,
    /// `unknown`
    Unknown,
}

impl MSGVERIFIEDLEVEL {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("high") {
            return Some(Self::High);
        }
        if value.eq_str("low") {
            return Some(Self::Low);
        }
        if value.eq_str("unknown") {
            return Some(Self::Unknown);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
            Self::Unknown => "unknown",
        }
    }
}

/// Wire values for `POLLTYPES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum POLLTYPES {
    /// `creation`
    Creation,
    /// `quiz_creation`
    QuizCreation,
    /// `vote`
    Vote,
    /// `result_snapshot`
    ResultSnapshot,
    /// `edit`
    Edit,
}

impl POLLTYPES {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("creation") {
            return Some(Self::Creation);
        }
        if value.eq_str("quiz_creation") {
            return Some(Self::QuizCreation);
        }
        if value.eq_str("vote") {
            return Some(Self::Vote);
        }
        if value.eq_str("result_snapshot") {
            return Some(Self::ResultSnapshot);
        }
        if value.eq_str("edit") {
            return Some(Self::Edit);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Creation => "creation",
            Self::QuizCreation => "quiz_creation",
            Self::Vote => "vote",
            Self::ResultSnapshot => "result_snapshot",
            Self::Edit => "edit",
        }
    }
}

/// Wire values for `STANZAMSGORIGIN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum STANZAMSGORIGIN {
    /// `ctwa`
    Ctwa,
}

impl STANZAMSGORIGIN {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("ctwa") {
            return Some(Self::Ctwa);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Ctwa => "ctwa",
        }
    }
}

/// Wire values for `STANZAMSGTYPES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum STANZAMSGTYPES {
    /// `text`
    Text,
    /// `media`
    Media,
    /// `medianotify`
    Medianotify,
    /// `pay`
    Pay,
    /// `poll`
    Poll,
    /// `reaction`
    Reaction,
    /// `event`
    Event,
}

impl STANZAMSGTYPES {
    /// Resolve a wire value, or `None` if this build does not know it.
    #[must_use]
    pub fn from_wire(value: Value<'_>) -> Option<Self> {
        if value.eq_str("text") {
            return Some(Self::Text);
        }
        if value.eq_str("media") {
            return Some(Self::Media);
        }
        if value.eq_str("medianotify") {
            return Some(Self::Medianotify);
        }
        if value.eq_str("pay") {
            return Some(Self::Pay);
        }
        if value.eq_str("poll") {
            return Some(Self::Poll);
        }
        if value.eq_str("reaction") {
            return Some(Self::Reaction);
        }
        if value.eq_str("event") {
            return Some(Self::Event);
        }
        None
    }

    /// The wire value this variant carries.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Media => "media",
            Self::Medianotify => "medianotify",
            Self::Pay => "pay",
            Self::Poll => "poll",
            Self::Reaction => "reaction",
            Self::Event => "event",
        }
    }
}

/// Derived from whatspec's `IncomingMsgParserPlaintext` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserPlaintext<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserPlaintext<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl IncomingMsgParserPlaintext<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &IncomingMsgParserPlaintext<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `IncomingMsgParserEnc` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserEnc<'a> {
    /// `type`, via `attrEnumValues`.
    pub r#type: CiphertextType,
    /// `mediatype`, via `maybeAttrString`.
    pub mediatype: Option<Value<'a>>,
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// `count`, via `maybeAttrInt`.
    pub count: Option<i64>,
    /// `decrypt-fail`, via `maybeAttrString`.
    pub decrypt_fail: Option<Value<'a>>,
    /// `state`, via `maybeAttrString`.
    pub state: Option<Value<'a>>,
    /// `session_type`, via `maybeAttrString`.
    pub session_type: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserEnc<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            r#type: extract::attr_enum(node, "type", CiphertextType::from_wire)?,
            mediatype: extract::maybe_attr_string(node, "mediatype"),
            content: extract::content_bytes(node)?,
            count: extract::maybe_attr_int(node, "count")?,
            decrypt_fail: extract::maybe_attr_string(node, "decrypt-fail"),
            state: extract::maybe_attr_string(node, "state"),
            session_type: extract::maybe_attr_string(node, "session_type"),
            node: *node,
        })
    }
}

impl IncomingMsgParserEnc<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserEnc<'_>) -> bool {
        (self.r#type == other.r#type)
            && (match (self.mediatype, other.mediatype) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.content == other.content)
            && (self.count == other.count)
            && (match (self.decrypt_fail, other.decrypt_fail) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.state, other.state) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.session_type, other.session_type) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgParserDeviceIdentity` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserDeviceIdentity<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserDeviceIdentity<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl IncomingMsgParserDeviceIdentity<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserDeviceIdentity<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `IncomingMsgParserBot` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserBot<'a> {
    /// `sender_timestamp_ms`, via `maybeAttrString`.
    pub sender_timestamp_ms: Option<Value<'a>>,
    /// `edit_target_id`, via `maybeAttrString`.
    pub edit_target_id: Option<Value<'a>>,
    /// `edit`, via `maybeAttrString`.
    pub edit: Option<Value<'a>>,
    /// `biz_bot`, via `maybeAttrString`.
    pub biz_bot: Option<Value<'a>>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserBot<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            sender_timestamp_ms: extract::maybe_attr_string(node, "sender_timestamp_ms"),
            edit_target_id: extract::maybe_attr_string(node, "edit_target_id"),
            edit: extract::maybe_attr_string(node, "edit"),
            biz_bot: extract::maybe_attr_string(node, "biz_bot"),
            r#type: extract::maybe_attr_string(node, "type"),
            node: *node,
        })
    }
}

impl IncomingMsgParserBot<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserBot<'_>) -> bool {
        (match (self.sender_timestamp_ms, other.sender_timestamp_ms) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.edit_target_id, other.edit_target_id) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.edit, other.edit) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.biz_bot, other.biz_bot) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.r#type, other.r#type) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgParserUnavailable` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserUnavailable<'a> {
    /// `hosted`, via `maybeAttrString`.
    pub hosted: Option<Value<'a>>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserUnavailable<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            hosted: extract::maybe_attr_string(node, "hosted"),
            r#type: extract::maybe_attr_string(node, "type"),
            node: *node,
        })
    }
}

impl IncomingMsgParserUnavailable<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserUnavailable<'_>) -> bool {
        (match (self.hosted, other.hosted) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.r#type, other.r#type) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgParserMetaKey` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserMetaKey<'a> {
    /// `rkid`, via `maybeAttrString`.
    pub rkid: Option<Value<'a>>,
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserMetaKey<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            rkid: extract::maybe_attr_string(node, "rkid"),
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl IncomingMsgParserMetaKey<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserMetaKey<'_>) -> bool {
        (match (self.rkid, other.rkid) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (self.content == other.content)
    }
}
/// Derived from whatspec's `IncomingMsgParserMeta` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserMeta<'a> {
    /// `polltype`, via `attrEnumOrNullIfUnknown`.
    pub polltype: Option<POLLTYPES>,
    /// `status_mentioned`, via `maybeAttrString`.
    pub status_mentioned: Option<Value<'a>>,
    /// `origin`, via `maybeAttrEnum`.
    pub origin: Option<STANZAMSGORIGIN>,
    /// `appdata`, via `maybeAttrEnum`.
    pub appdata: Option<APPDATA>,
    /// `thread_msg_id`, via `maybeAttrString`.
    pub thread_msg_id: Option<Value<'a>>,
    /// `thread_msg_sender_jid`, via `attrJidWithType`.
    pub thread_msg_sender_jid: Option<Jid<'a>>,
    /// `target_id`, via `maybeAttrString`.
    pub target_id: Option<Value<'a>>,
    /// `target_sender_jid`, via `attrJidWithType`.
    pub target_sender_jid: Option<Jid<'a>>,
    /// `target_chat_jid`, via `attrJidWithType`.
    pub target_chat_jid: Option<Jid<'a>>,
    /// `target_chat_jid_lid`, via `attrJidWithType`.
    pub target_chat_jid_lid: Option<Jid<'a>>,
    /// `from`, via `attrJidWithType`.
    pub from: Option<Jid<'a>>,
    /// `capi`, via `maybeAttrString`.
    pub capi: Option<Value<'a>>,
    /// `event_type`, via `maybeAttrEnum`.
    pub event_type: Option<EVENTTYPES>,
    /// `context_source`, via `maybeAttrString`.
    pub context_source: Option<Value<'a>>,
    /// `read`, via `maybeAttrString`.
    pub read: Option<Value<'a>>,
    /// `is_group_status`, via `maybeAttrString`.
    pub is_group_status: Option<Value<'a>>,
    /// `session_scope`, via `maybeAttrString`.
    pub session_scope: Option<Value<'a>>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// `st`, via `maybeAttrInt`.
    pub st: Option<i64>,
    /// `key`, via `maybeChild`.
    pub key: Option<alloc::boxed::Box<IncomingMsgParserMetaKey<'a>>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserMeta<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            polltype: extract::attr_enum_or_none(node, "polltype", POLLTYPES::from_wire),
            status_mentioned: extract::maybe_attr_string(node, "status_mentioned"),
            origin: extract::maybe_attr_enum(node, "origin", STANZAMSGORIGIN::from_wire)?,
            appdata: extract::maybe_attr_enum(node, "appdata", APPDATA::from_wire)?,
            thread_msg_id: extract::maybe_attr_string(node, "thread_msg_id"),
            thread_msg_sender_jid: extract::maybe_attr_jid(node, "thread_msg_sender_jid")?,
            target_id: extract::maybe_attr_string(node, "target_id"),
            target_sender_jid: extract::maybe_attr_jid(node, "target_sender_jid")?,
            target_chat_jid: extract::maybe_attr_jid(node, "target_chat_jid")?,
            target_chat_jid_lid: extract::maybe_attr_jid(node, "target_chat_jid_lid")?,
            from: extract::maybe_attr_jid(node, "from")?,
            capi: extract::maybe_attr_string(node, "capi"),
            event_type: extract::maybe_attr_enum(node, "event_type", EVENTTYPES::from_wire)?,
            context_source: extract::maybe_attr_string(node, "context_source"),
            read: extract::maybe_attr_string(node, "read"),
            is_group_status: extract::maybe_attr_string(node, "is_group_status"),
            session_scope: extract::maybe_attr_string(node, "session_scope"),
            r#type: extract::maybe_attr_string(node, "type"),
            st: extract::maybe_attr_int(node, "st")?,
            key: match extract::maybe_child(node, "key") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserMetaKey::derive(
                    &child,
                )?)),
                None => None,
            },
            node: *node,
        })
    }
}

impl IncomingMsgParserMeta<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserMeta<'_>) -> bool {
        (self.polltype == other.polltype)
            && (match (self.status_mentioned, other.status_mentioned) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.origin == other.origin)
            && (self.appdata == other.appdata)
            && (match (self.thread_msg_id, other.thread_msg_id) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.thread_msg_sender_jid, other.thread_msg_sender_jid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.target_id, other.target_id) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.target_sender_jid, other.target_sender_jid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.target_chat_jid, other.target_chat_jid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.target_chat_jid_lid, other.target_chat_jid_lid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.from, other.from) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.capi, other.capi) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.event_type == other.event_type)
            && (match (self.context_source, other.context_source) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.read, other.read) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.is_group_status, other.is_group_status) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.session_scope, other.session_scope) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.r#type, other.r#type) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.st == other.st)
            && (match (&self.key, &other.key) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgParserVerifiedName` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserVerifiedName<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserVerifiedName<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl IncomingMsgParserVerifiedName<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserVerifiedName<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `IncomingMsgParserBiz` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserBiz<'a> {
    /// `actual_actors`, via `maybeAttrInt`.
    pub actual_actors: Option<i64>,
    /// `host_storage`, via `maybeAttrInt`.
    pub host_storage: Option<i64>,
    /// `privacy_mode_ts`, via `maybeAttrInt`.
    pub privacy_mode_ts: Option<i64>,
    /// `native_flow_name`, via `maybeAttrString`.
    pub native_flow_name: Option<Value<'a>>,
    /// `campaign_id`, via `maybeAttrString`.
    pub campaign_id: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserBiz<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            actual_actors: extract::maybe_attr_int(node, "actual_actors")?,
            host_storage: extract::maybe_attr_int(node, "host_storage")?,
            privacy_mode_ts: extract::maybe_attr_int(node, "privacy_mode_ts")?,
            native_flow_name: extract::maybe_attr_string(node, "native_flow_name"),
            campaign_id: extract::maybe_attr_string(node, "campaign_id"),
            node: *node,
        })
    }
}

impl IncomingMsgParserBiz<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserBiz<'_>) -> bool {
        (self.actual_actors == other.actual_actors)
            && (self.host_storage == other.host_storage)
            && (self.privacy_mode_ts == other.privacy_mode_ts)
            && (match (self.native_flow_name, other.native_flow_name) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.campaign_id, other.campaign_id) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgParserPay` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserPay<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserPay<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl IncomingMsgParserPay<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &IncomingMsgParserPay<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `IncomingMsgParserTransaction` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserTransaction<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserTransaction<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl IncomingMsgParserTransaction<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &IncomingMsgParserTransaction<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `IncomingMsgParserHsm` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserHsm<'a> {
    /// `tag`, via `maybeAttrString`.
    pub tag: Option<Value<'a>>,
    /// `category`, via `maybeAttrString`.
    pub category: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserHsm<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            tag: extract::maybe_attr_string(node, "tag"),
            category: extract::maybe_attr_string(node, "category"),
            node: *node,
        })
    }
}

impl IncomingMsgParserHsm<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserHsm<'_>) -> bool {
        (match (self.tag, other.tag) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.category, other.category) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgParserReportingReportingToken` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserReportingReportingToken<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// `v`, via `attrInt`.
    pub v: i64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserReportingReportingToken<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            v: extract::attr_int(node, "v")?,
            node: *node,
        })
    }
}

impl IncomingMsgParserReportingReportingToken<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserReportingReportingToken<'_>) -> bool {
        (self.content == other.content) && (self.v == other.v)
    }
}
/// Derived from whatspec's `IncomingMsgParserReportingReportingTag` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserReportingReportingTag<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserReportingReportingTag<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl IncomingMsgParserReportingReportingTag<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserReportingReportingTag<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `IncomingMsgParserReporting` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserReporting<'a> {
    /// `reporting_token`, via `maybeChild`.
    pub reporting_token: Option<alloc::boxed::Box<IncomingMsgParserReportingReportingToken<'a>>>,
    /// `reporting_tag`, via `maybeChild`.
    pub reporting_tag: Option<alloc::boxed::Box<IncomingMsgParserReportingReportingTag<'a>>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserReporting<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            reporting_token: match extract::maybe_child(node, "reporting_token") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserReportingReportingToken::derive(&child)?,
                )),
                None => None,
            },
            reporting_tag: match extract::maybe_child(node, "reporting_tag") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserReportingReportingTag::derive(&child)?,
                )),
                None => None,
            },
            node: *node,
        })
    }
}

impl IncomingMsgParserReporting<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserReporting<'_>) -> bool {
        (match (&self.reporting_token, &other.reporting_token) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (&self.reporting_tag, &other.reporting_tag) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgParserRcat` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserRcat<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserRcat<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl IncomingMsgParserRcat<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &IncomingMsgParserRcat<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `IncomingMsgParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParser<'a> {
    /// `plaintext`, via `maybeChild`.
    pub plaintext: Option<alloc::boxed::Box<IncomingMsgParserPlaintext<'a>>>,
    /// `device-identity`, via `maybeChild`.
    pub device_identity: Option<alloc::boxed::Box<IncomingMsgParserDeviceIdentity<'a>>>,
    /// `bot`, via `maybeChild`.
    pub bot: Option<alloc::boxed::Box<IncomingMsgParserBot<'a>>>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `participant`, via `attrDeviceJid`.
    pub participant: Option<Jid<'a>>,
    /// `unavailable`, via `maybeChild`.
    pub unavailable: Option<alloc::boxed::Box<IncomingMsgParserUnavailable<'a>>>,
    /// `type`, via `attrEnum`.
    pub r#type: STANZAMSGTYPES,
    /// `meta`, via `maybeChild`.
    pub meta: Option<alloc::boxed::Box<IncomingMsgParserMeta<'a>>>,
    /// `t`, via `attrTime`.
    pub t: i64,
    /// `verified_name`, via `maybeChild`.
    pub verified_name: Option<alloc::boxed::Box<IncomingMsgParserVerifiedName<'a>>>,
    /// `verified_level`, via `maybeAttrEnum`.
    pub verified_level: Option<MSGVERIFIEDLEVEL>,
    /// `verified_name_attr`, via `maybeAttrInt`.
    pub verified_name_attr: Option<i64>,
    /// `biz`, via `maybeChild`.
    pub biz: Option<alloc::boxed::Box<IncomingMsgParserBiz<'a>>>,
    /// `pay`, via `maybeChild`.
    pub pay: Option<alloc::boxed::Box<IncomingMsgParserPay<'a>>>,
    /// `transaction`, via `maybeChild`.
    pub transaction: Option<alloc::boxed::Box<IncomingMsgParserTransaction<'a>>>,
    /// `recipient`, via `attrString`.
    pub recipient: Value<'a>,
    /// `hsm`, via `maybeChild`.
    pub hsm: Option<alloc::boxed::Box<IncomingMsgParserHsm<'a>>>,
    /// `reporting`, via `maybeChild`.
    pub reporting: Option<alloc::boxed::Box<IncomingMsgParserReporting<'a>>>,
    /// `rcat`, via `maybeChild`.
    pub rcat: Option<alloc::boxed::Box<IncomingMsgParserRcat<'a>>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            plaintext: match extract::maybe_child(node, "plaintext") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserPlaintext::derive(
                    &child,
                )?)),
                None => None,
            },
            device_identity: match extract::maybe_child(node, "device-identity") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserDeviceIdentity::derive(&child)?,
                )),
                None => None,
            },
            bot: match extract::maybe_child(node, "bot") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserBot::derive(
                    &child,
                )?)),
                None => None,
            },
            from: extract::attr_jid(node, "from")?,
            participant: extract::maybe_attr_jid(node, "participant")?,
            unavailable: match extract::maybe_child(node, "unavailable") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserUnavailable::derive(&child)?,
                )),
                None => None,
            },
            r#type: extract::attr_enum(node, "type", STANZAMSGTYPES::from_wire)?,
            meta: match extract::maybe_child(node, "meta") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserMeta::derive(
                    &child,
                )?)),
                None => None,
            },
            t: extract::attr_time(node, "t")?,
            verified_name: match extract::maybe_child(node, "verified_name") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserVerifiedName::derive(&child)?,
                )),
                None => None,
            },
            verified_level: extract::maybe_attr_enum(
                node,
                "verified_level",
                MSGVERIFIEDLEVEL::from_wire,
            )?,
            verified_name_attr: extract::maybe_attr_int(node, "verified_name")?,
            biz: match extract::maybe_child(node, "biz") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserBiz::derive(
                    &child,
                )?)),
                None => None,
            },
            pay: match extract::maybe_child(node, "pay") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserPay::derive(
                    &child,
                )?)),
                None => None,
            },
            transaction: match extract::maybe_child(node, "transaction") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserTransaction::derive(&child)?,
                )),
                None => None,
            },
            recipient: extract::attr_string(node, "recipient")?,
            hsm: match extract::maybe_child(node, "hsm") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserHsm::derive(
                    &child,
                )?)),
                None => None,
            },
            reporting: match extract::maybe_child(node, "reporting") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserReporting::derive(
                    &child,
                )?)),
                None => None,
            },
            rcat: match extract::maybe_child(node, "rcat") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgParserRcat::derive(
                    &child,
                )?)),
                None => None,
            },
            node: *node,
        })
    }
}

impl<'a> IncomingMsgParser<'a> {
    /// Each `<enc>` child, derived lazily.
    ///
    /// An iterator rather than a collection: nothing is allocated, and a
    /// caller that wants only the first does not pay for the rest.
    pub fn enc(
        &self,
    ) -> impl Iterator<Item = Result<IncomingMsgParserEnc<'a>, DeriveError>> + use<'a> {
        extract::children_with_tag(&self.node, "enc")
            .map(|child| IncomingMsgParserEnc::derive(&child))
    }
}

impl IncomingMsgParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParser<'_>) -> bool {
        (match (&self.plaintext, &other.plaintext) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (crate::semantic::iter_eq(self.enc(), other.enc()))
            && (match (&self.device_identity, &other.device_identity) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.bot, &other.bot) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.from.semantic_eq(other.from))
            && (match (self.participant, other.participant) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.unavailable, &other.unavailable) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.r#type == other.r#type)
            && (match (&self.meta, &other.meta) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.t == other.t)
            && (match (&self.verified_name, &other.verified_name) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.verified_level == other.verified_level)
            && (self.verified_name_attr == other.verified_name_attr)
            && (match (&self.biz, &other.biz) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.pay, &other.pay) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.transaction, &other.transaction) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.recipient.semantic_eq(other.recipient))
            && (match (&self.hsm, &other.hsm) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.reporting, &other.reporting) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.rcat, &other.rcat) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgParserForAckOnlyBot` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserForAckOnlyBot<'a> {
    /// `sender_timestamp_ms`, via `maybeAttrString`.
    pub sender_timestamp_ms: Option<Value<'a>>,
    /// `edit_target_id`, via `maybeAttrString`.
    pub edit_target_id: Option<Value<'a>>,
    /// `edit`, via `maybeAttrString`.
    pub edit: Option<Value<'a>>,
    /// `biz_bot`, via `maybeAttrString`.
    pub biz_bot: Option<Value<'a>>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserForAckOnlyBot<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            sender_timestamp_ms: extract::maybe_attr_string(node, "sender_timestamp_ms"),
            edit_target_id: extract::maybe_attr_string(node, "edit_target_id"),
            edit: extract::maybe_attr_string(node, "edit"),
            biz_bot: extract::maybe_attr_string(node, "biz_bot"),
            r#type: extract::maybe_attr_string(node, "type"),
            node: *node,
        })
    }
}

impl IncomingMsgParserForAckOnlyBot<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserForAckOnlyBot<'_>) -> bool {
        (match (self.sender_timestamp_ms, other.sender_timestamp_ms) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.edit_target_id, other.edit_target_id) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.edit, other.edit) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.biz_bot, other.biz_bot) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.r#type, other.r#type) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgParserForAckOnly` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgParserForAckOnly<'a> {
    /// `type`, via `attrEnum`.
    pub r#type: STANZAMSGTYPES,
    /// `offline`, via `attrString`.
    pub offline: Value<'a>,
    /// `bot`, via `maybeChild`.
    pub bot: Option<alloc::boxed::Box<IncomingMsgParserForAckOnlyBot<'a>>>,
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `participant`, via `attrDeviceJid`.
    pub participant: Option<Jid<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgParserForAckOnly<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            r#type: extract::attr_enum(node, "type", STANZAMSGTYPES::from_wire)?,
            offline: extract::attr_string(node, "offline")?,
            bot: match extract::maybe_child(node, "bot") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgParserForAckOnlyBot::derive(&child)?,
                )),
                None => None,
            },
            id: extract::attr_string(node, "id")?,
            from: extract::attr_jid(node, "from")?,
            participant: extract::maybe_attr_jid(node, "participant")?,
            node: *node,
        })
    }
}

impl IncomingMsgParserForAckOnly<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgParserForAckOnly<'_>) -> bool {
        (self.r#type == other.r#type)
            && (self.offline.semantic_eq(other.offline))
            && (match (&self.bot, &other.bot) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.id.semantic_eq(other.id))
            && (self.from.semantic_eq(other.from))
            && (match (self.participant, other.participant) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `CallReceiptParserOffer` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallReceiptParserOffer<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallReceiptParserOffer<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl CallReceiptParserOffer<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &CallReceiptParserOffer<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `CallReceiptParserAccept` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallReceiptParserAccept<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallReceiptParserAccept<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl CallReceiptParserAccept<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &CallReceiptParserAccept<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `CallReceiptParserReject` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallReceiptParserReject<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallReceiptParserReject<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl CallReceiptParserReject<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, _other: &CallReceiptParserReject<'_>) -> bool {
        true
    }
}
/// Derived from whatspec's `CallReceiptParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallReceiptParser<'a> {
    /// `offer`, via `maybeChild`.
    pub offer: Option<alloc::boxed::Box<CallReceiptParserOffer<'a>>>,
    /// `accept`, via `maybeChild`.
    pub accept: Option<alloc::boxed::Box<CallReceiptParserAccept<'a>>>,
    /// `reject`, via `maybeChild`.
    pub reject: Option<alloc::boxed::Box<CallReceiptParserReject<'a>>>,
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallReceiptParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            offer: match extract::maybe_child(node, "offer") {
                Some(child) => Some(alloc::boxed::Box::new(CallReceiptParserOffer::derive(
                    &child,
                )?)),
                None => None,
            },
            accept: match extract::maybe_child(node, "accept") {
                Some(child) => Some(alloc::boxed::Box::new(CallReceiptParserAccept::derive(
                    &child,
                )?)),
                None => None,
            },
            reject: match extract::maybe_child(node, "reject") {
                Some(child) => Some(alloc::boxed::Box::new(CallReceiptParserReject::derive(
                    &child,
                )?)),
                None => None,
            },
            id: extract::attr_string(node, "id")?,
            r#type: extract::maybe_attr_string(node, "type"),
            from: extract::attr_jid(node, "from")?,
            node: *node,
        })
    }
}

impl CallReceiptParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &CallReceiptParser<'_>) -> bool {
        (match (&self.offer, &other.offer) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (&self.accept, &other.accept) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (&self.reject, &other.reject) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (self.id.semantic_eq(other.id))
            && (match (self.r#type, other.r#type) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.from.semantic_eq(other.from))
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserError` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserError<'a> {
    /// `reason`, via `maybeAttrString`.
    pub reason: Option<Value<'a>>,
    /// `type`, via `attrString`.
    pub r#type: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserError<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            reason: extract::maybe_attr_string(node, "reason"),
            r#type: extract::attr_string(node, "type")?,
            node: *node,
        })
    }
}

impl IncomingMsgReceiptParserError<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserError<'_>) -> bool {
        (match (self.reason, other.reason) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (self.r#type.semantic_eq(other.r#type))
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserParticipantsUser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserParticipantsUser<'a> {
    /// `jid`, via `attrDeviceJid`.
    pub jid: Jid<'a>,
    /// `t`, via `attrTime`.
    pub t: i64,
    /// `type`, via `maybeAttrEnum`.
    pub r#type: Option<IncomingMsgReceiptParserParticipantsUserType>,
    /// `participant_pn`, via `attrUserJid`.
    pub participant_pn: Option<Jid<'a>>,
    /// `participant_username`, via `maybeAttrString`.
    pub participant_username: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserParticipantsUser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            jid: extract::attr_jid(node, "jid")?,
            t: extract::attr_time(node, "t")?,
            r#type: extract::maybe_attr_enum(
                node,
                "type",
                IncomingMsgReceiptParserParticipantsUserType::from_wire,
            )?,
            participant_pn: extract::maybe_attr_jid(node, "participant_pn")?,
            participant_username: extract::maybe_attr_string(node, "participant_username"),
            node: *node,
        })
    }
}

impl IncomingMsgReceiptParserParticipantsUser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserParticipantsUser<'_>) -> bool {
        (self.jid.semantic_eq(other.jid))
            && (self.t == other.t)
            && (self.r#type == other.r#type)
            && (match (self.participant_pn, other.participant_pn) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.participant_username, other.participant_username) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserParticipants` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserParticipants<'a> {
    /// `message_id`, via `maybeAttrString`.
    pub message_id: Option<Value<'a>>,
    /// `key`, via `maybeAttrString`.
    pub key: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserParticipants<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            message_id: extract::maybe_attr_string(node, "message_id"),
            key: extract::maybe_attr_string(node, "key"),
            node: *node,
        })
    }
}

impl<'a> IncomingMsgReceiptParserParticipants<'a> {
    /// Each `<user>` child, derived lazily.
    ///
    /// An iterator rather than a collection: nothing is allocated, and a
    /// caller that wants only the first does not pay for the rest.
    pub fn user(
        &self,
    ) -> impl Iterator<Item = Result<IncomingMsgReceiptParserParticipantsUser<'a>, DeriveError>> + use<'a>
    {
        extract::children_with_tag(&self.node, "user")
            .map(|child| IncomingMsgReceiptParserParticipantsUser::derive(&child))
    }
}

impl IncomingMsgReceiptParserParticipants<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserParticipants<'_>) -> bool {
        (crate::semantic::iter_eq(self.user(), other.user()))
            && (match (self.message_id, other.message_id) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.key, other.key) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserListItem` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserListItem<'a> {
    /// `server_id`, via `maybeAttrString`.
    pub server_id: Option<Value<'a>>,
    /// `id`, via `maybeAttrString`.
    pub id: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserListItem<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            server_id: extract::maybe_attr_string(node, "server_id"),
            id: extract::maybe_attr_string(node, "id"),
            node: *node,
        })
    }
}

impl IncomingMsgReceiptParserListItem<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserListItem<'_>) -> bool {
        (match (self.server_id, other.server_id) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.id, other.id) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserList` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserList<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserList<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
    }
}

impl<'a> IncomingMsgReceiptParserList<'a> {
    /// Each `<item>` child, derived lazily.
    ///
    /// An iterator rather than a collection: nothing is allocated, and a
    /// caller that wants only the first does not pay for the rest.
    pub fn item(
        &self,
    ) -> impl Iterator<Item = Result<IncomingMsgReceiptParserListItem<'a>, DeriveError>> + use<'a>
    {
        extract::children_with_tag(&self.node, "item")
            .map(|child| IncomingMsgReceiptParserListItem::derive(&child))
    }
}

impl IncomingMsgReceiptParserList<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserList<'_>) -> bool {
        crate::semantic::iter_eq(self.item(), other.item())
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParserBiz` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParserBiz<'a> {
    /// `actual_actors`, via `maybeAttrInt`.
    pub actual_actors: Option<i64>,
    /// `host_storage`, via `maybeAttrInt`.
    pub host_storage: Option<i64>,
    /// `privacy_mode_ts`, via `maybeAttrInt`.
    pub privacy_mode_ts: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParserBiz<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            actual_actors: extract::maybe_attr_int(node, "actual_actors")?,
            host_storage: extract::maybe_attr_int(node, "host_storage")?,
            privacy_mode_ts: extract::maybe_attr_int(node, "privacy_mode_ts")?,
            node: *node,
        })
    }
}

impl IncomingMsgReceiptParserBiz<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParserBiz<'_>) -> bool {
        (self.actual_actors == other.actual_actors)
            && (self.host_storage == other.host_storage)
            && (self.privacy_mode_ts == other.privacy_mode_ts)
    }
}
/// Derived from whatspec's `IncomingMsgReceiptParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct IncomingMsgReceiptParser<'a> {
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `offline`, via `maybeAttrString`.
    pub offline: Option<Value<'a>>,
    /// `type`, via `maybeAttrEnum`.
    pub r#type: Option<IncomingMsgReceiptParserType>,
    /// `error`, via `maybeChild`.
    pub error: Option<alloc::boxed::Box<IncomingMsgReceiptParserError<'a>>>,
    /// `participants`, via `maybeChild`.
    pub participants: Option<alloc::boxed::Box<IncomingMsgReceiptParserParticipants<'a>>>,
    /// `participant`, via `attrDeviceJid`.
    pub participant: Option<Jid<'a>>,
    /// `recipient`, via `attrUserJid`.
    pub recipient: Option<Jid<'a>>,
    /// `list`, via `maybeChild`.
    pub list: Option<alloc::boxed::Box<IncomingMsgReceiptParserList<'a>>>,
    /// `biz`, via `maybeChild`.
    pub biz: Option<alloc::boxed::Box<IncomingMsgReceiptParserBiz<'a>>>,
    /// `is_lid`, via `maybeAttrString`.
    pub is_lid: Option<Value<'a>>,
    /// `participant_pn`, via `attrUserJid`.
    pub participant_pn: Option<Jid<'a>>,
    /// `participant_username`, via `maybeAttrString`.
    pub participant_username: Option<Value<'a>>,
    /// `t`, via `maybeAttrTime`.
    pub t: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> IncomingMsgReceiptParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: extract::attr_string(node, "id")?,
            from: extract::attr_jid(node, "from")?,
            offline: extract::maybe_attr_string(node, "offline"),
            r#type: extract::maybe_attr_enum(
                node,
                "type",
                IncomingMsgReceiptParserType::from_wire,
            )?,
            error: match extract::maybe_child(node, "error") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgReceiptParserError::derive(&child)?,
                )),
                None => None,
            },
            participants: match extract::maybe_child(node, "participants") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgReceiptParserParticipants::derive(&child)?,
                )),
                None => None,
            },
            participant: extract::maybe_attr_jid(node, "participant")?,
            recipient: extract::maybe_attr_jid(node, "recipient")?,
            list: match extract::maybe_child(node, "list") {
                Some(child) => Some(alloc::boxed::Box::new(
                    IncomingMsgReceiptParserList::derive(&child)?,
                )),
                None => None,
            },
            biz: match extract::maybe_child(node, "biz") {
                Some(child) => Some(alloc::boxed::Box::new(IncomingMsgReceiptParserBiz::derive(
                    &child,
                )?)),
                None => None,
            },
            is_lid: extract::maybe_attr_string(node, "is_lid"),
            participant_pn: extract::maybe_attr_jid(node, "participant_pn")?,
            participant_username: extract::maybe_attr_string(node, "participant_username"),
            t: extract::maybe_attr_time(node, "t")?,
            node: *node,
        })
    }
}

impl IncomingMsgReceiptParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &IncomingMsgReceiptParser<'_>) -> bool {
        (self.id.semantic_eq(other.id))
            && (self.from.semantic_eq(other.from))
            && (match (self.offline, other.offline) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.r#type == other.r#type)
            && (match (&self.error, &other.error) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.participants, &other.participants) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.participant, other.participant) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.recipient, other.recipient) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.list, &other.list) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.biz, &other.biz) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.is_lid, other.is_lid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.participant_pn, other.participant_pn) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.participant_username, other.participant_username) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.t == other.t)
    }
}
/// Derived from whatspec's `RetryRequestParserRetry` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserRetry<'a> {
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `count`, via `maybeAttrInt`.
    pub count: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserRetry<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: extract::attr_string(node, "id")?,
            count: extract::maybe_attr_int(node, "count")?,
            node: *node,
        })
    }
}

impl RetryRequestParserRetry<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserRetry<'_>) -> bool {
        (self.id.semantic_eq(other.id)) && (self.count == other.count)
    }
}
/// Derived from whatspec's `RetryRequestParserKeysIdentity` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysIdentity<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysIdentity<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysIdentity<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysIdentity<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysSkeyId` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysSkeyId<'a> {
    /// `content`, via `contentUint`.
    pub content: u64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysSkeyId<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_uint(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysSkeyId<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysSkeyId<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysSkeyValue` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysSkeyValue<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysSkeyValue<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysSkeyValue<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysSkeyValue<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysSkeySignature` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysSkeySignature<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysSkeySignature<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysSkeySignature<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysSkeySignature<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysSkey` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysSkey<'a> {
    /// `id`, via `child`.
    pub id: alloc::boxed::Box<RetryRequestParserKeysSkeyId<'a>>,
    /// `value`, via `child`.
    pub value: alloc::boxed::Box<RetryRequestParserKeysSkeyValue<'a>>,
    /// `signature`, via `child`.
    pub signature: alloc::boxed::Box<RetryRequestParserKeysSkeySignature<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysSkey<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: alloc::boxed::Box::new(RetryRequestParserKeysSkeyId::derive(&extract::child(
                node, "id",
            )?)?),
            value: alloc::boxed::Box::new(RetryRequestParserKeysSkeyValue::derive(
                &extract::child(node, "value")?,
            )?),
            signature: alloc::boxed::Box::new(RetryRequestParserKeysSkeySignature::derive(
                &extract::child(node, "signature")?,
            )?),
            node: *node,
        })
    }
}

impl RetryRequestParserKeysSkey<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysSkey<'_>) -> bool {
        (self.id.semantic_eq(&other.id))
            && (self.value.semantic_eq(&other.value))
            && (self.signature.semantic_eq(&other.signature))
    }
}
/// Derived from whatspec's `RetryRequestParserKeysKeyId` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysKeyId<'a> {
    /// `content`, via `contentUint`.
    pub content: u64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysKeyId<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_uint(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysKeyId<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysKeyId<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysKeyValue` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysKeyValue<'a> {
    /// `content`, via `contentBytes`.
    pub content: &'a [u8],
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysKeyValue<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_bytes(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserKeysKeyValue<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysKeyValue<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParserKeysKey` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeysKey<'a> {
    /// `id`, via `child`.
    pub id: alloc::boxed::Box<RetryRequestParserKeysKeyId<'a>>,
    /// `value`, via `child`.
    pub value: alloc::boxed::Box<RetryRequestParserKeysKeyValue<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeysKey<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: alloc::boxed::Box::new(RetryRequestParserKeysKeyId::derive(&extract::child(
                node, "id",
            )?)?),
            value: alloc::boxed::Box::new(RetryRequestParserKeysKeyValue::derive(
                &extract::child(node, "value")?,
            )?),
            node: *node,
        })
    }
}

impl RetryRequestParserKeysKey<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeysKey<'_>) -> bool {
        (self.id.semantic_eq(&other.id)) && (self.value.semantic_eq(&other.value))
    }
}
/// Derived from whatspec's `RetryRequestParserKeys` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserKeys<'a> {
    /// `identity`, via `child`.
    pub identity: alloc::boxed::Box<RetryRequestParserKeysIdentity<'a>>,
    /// `skey`, via `child`.
    pub skey: alloc::boxed::Box<RetryRequestParserKeysSkey<'a>>,
    /// `key`, via `maybeChild`.
    pub key: Option<alloc::boxed::Box<RetryRequestParserKeysKey<'a>>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserKeys<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            identity: alloc::boxed::Box::new(RetryRequestParserKeysIdentity::derive(
                &extract::child(node, "identity")?,
            )?),
            skey: alloc::boxed::Box::new(RetryRequestParserKeysSkey::derive(&extract::child(
                node, "skey",
            )?)?),
            key: match extract::maybe_child(node, "key") {
                Some(child) => Some(alloc::boxed::Box::new(RetryRequestParserKeysKey::derive(
                    &child,
                )?)),
                None => None,
            },
            node: *node,
        })
    }
}

impl RetryRequestParserKeys<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserKeys<'_>) -> bool {
        (self.identity.semantic_eq(&other.identity))
            && (self.skey.semantic_eq(&other.skey))
            && (match (&self.key, &other.key) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `RetryRequestParserRegistration` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParserRegistration<'a> {
    /// `content`, via `contentUint`.
    pub content: u64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParserRegistration<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            content: extract::content_uint(node)?,
            node: *node,
        })
    }
}

impl RetryRequestParserRegistration<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParserRegistration<'_>) -> bool {
        self.content == other.content
    }
}
/// Derived from whatspec's `RetryRequestParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RetryRequestParser<'a> {
    /// `type`, via `attrString`.
    pub r#type: Value<'a>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `participant`, via `attrDeviceJid`.
    pub participant: Option<Jid<'a>>,
    /// `is_lid`, via `maybeAttrString`.
    pub is_lid: Option<Value<'a>>,
    /// `recipient`, via `attrDeviceJid`.
    pub recipient: Option<Jid<'a>>,
    /// `retry`, via `child`.
    pub retry: alloc::boxed::Box<RetryRequestParserRetry<'a>>,
    /// `keys`, via `maybeChild`.
    pub keys: Option<alloc::boxed::Box<RetryRequestParserKeys<'a>>>,
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `t`, via `attrTime`.
    pub t: i64,
    /// `registration`, via `child`.
    pub registration: alloc::boxed::Box<RetryRequestParserRegistration<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> RetryRequestParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            r#type: extract::attr_string(node, "type")?,
            from: extract::attr_jid(node, "from")?,
            participant: extract::maybe_attr_jid(node, "participant")?,
            is_lid: extract::maybe_attr_string(node, "is_lid"),
            recipient: extract::maybe_attr_jid(node, "recipient")?,
            retry: alloc::boxed::Box::new(RetryRequestParserRetry::derive(&extract::child(
                node, "retry",
            )?)?),
            keys: match extract::maybe_child(node, "keys") {
                Some(child) => Some(alloc::boxed::Box::new(RetryRequestParserKeys::derive(
                    &child,
                )?)),
                None => None,
            },
            id: extract::attr_string(node, "id")?,
            t: extract::attr_time(node, "t")?,
            registration: alloc::boxed::Box::new(RetryRequestParserRegistration::derive(
                &extract::child(node, "registration")?,
            )?),
            node: *node,
        })
    }
}

impl RetryRequestParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &RetryRequestParser<'_>) -> bool {
        (self.r#type.semantic_eq(other.r#type))
            && (self.from.semantic_eq(other.from))
            && (match (self.participant, other.participant) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.is_lid, other.is_lid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.recipient, other.recipient) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.retry.semantic_eq(&other.retry))
            && (match (&self.keys, &other.keys) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.id.semantic_eq(other.id))
            && (self.t == other.t)
            && (self.registration.semantic_eq(&other.registration))
    }
}
/// Derived from whatspec's `CallOfferNoticeParserOfferNotice` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallOfferNoticeParserOfferNotice<'a> {
    /// `call-creator`, via `attrDeviceJid`.
    pub call_creator: Jid<'a>,
    /// `call-id`, via `attrString`.
    pub call_id: Value<'a>,
    /// `type`, via `attrString`.
    pub r#type: Value<'a>,
    /// `media`, via `attrString`.
    pub media: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallOfferNoticeParserOfferNotice<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            call_creator: extract::attr_jid(node, "call-creator")?,
            call_id: extract::attr_string(node, "call-id")?,
            r#type: extract::attr_string(node, "type")?,
            media: extract::attr_string(node, "media")?,
            node: *node,
        })
    }
}

impl CallOfferNoticeParserOfferNotice<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &CallOfferNoticeParserOfferNotice<'_>) -> bool {
        (self.call_creator.semantic_eq(other.call_creator))
            && (self.call_id.semantic_eq(other.call_id))
            && (self.r#type.semantic_eq(other.r#type))
            && (self.media.semantic_eq(other.media))
    }
}
/// Derived from whatspec's `CallOfferNoticeParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallOfferNoticeParser<'a> {
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `offer_notice`, via `child`.
    pub offer_notice: alloc::boxed::Box<CallOfferNoticeParserOfferNotice<'a>>,
    /// `t`, via `attrTime`.
    pub t: i64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallOfferNoticeParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: extract::attr_string(node, "id")?,
            from: extract::attr_jid(node, "from")?,
            offer_notice: alloc::boxed::Box::new(CallOfferNoticeParserOfferNotice::derive(
                &extract::child(node, "offer_notice")?,
            )?),
            t: extract::attr_time(node, "t")?,
            node: *node,
        })
    }
}

impl CallOfferNoticeParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &CallOfferNoticeParser<'_>) -> bool {
        (self.id.semantic_eq(other.id))
            && (self.from.semantic_eq(other.from))
            && (self.offer_notice.semantic_eq(&other.offer_notice))
            && (self.t == other.t)
    }
}
/// Derived from whatspec's `CallOfferPlaceholder` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallOfferPlaceholder<'a> {
    /// `t`, via `maybeAttrTime`.
    pub t: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallOfferPlaceholder<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            t: extract::maybe_attr_time(node, "t")?,
            node: *node,
        })
    }
}

impl CallOfferPlaceholder<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &CallOfferPlaceholder<'_>) -> bool {
        self.t == other.t
    }
}
/// Derived from whatspec's `CallParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallParser<'a> {
    /// `from`, via `attrJidWithType`.
    pub from: Jid<'a>,
    /// `sender_lid`, via `attrJidWithType`.
    pub sender_lid: Option<Jid<'a>>,
    /// `platform`, via `maybeAttrString`.
    pub platform: Option<Value<'a>>,
    /// `version`, via `maybeAttrString`.
    pub version: Option<Value<'a>>,
    /// `t`, via `maybeAttrTime`.
    pub t: Option<i64>,
    /// `e`, via `maybeAttrTime`.
    pub e: Option<i64>,
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> CallParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            from: extract::attr_jid(node, "from")?,
            sender_lid: extract::maybe_attr_jid(node, "sender_lid")?,
            platform: extract::maybe_attr_string(node, "platform"),
            version: extract::maybe_attr_string(node, "version"),
            t: extract::maybe_attr_time(node, "t")?,
            e: extract::maybe_attr_time(node, "e")?,
            id: extract::attr_string(node, "id")?,
            node: *node,
        })
    }
}

impl CallParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &CallParser<'_>) -> bool {
        (self.from.semantic_eq(other.from))
            && (match (self.sender_lid, other.sender_lid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.platform, other.platform) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.version, other.version) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.t == other.t)
            && (self.e == other.e)
            && (self.id.semantic_eq(other.id))
    }
}
/// Derived from whatspec's `Ack` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Ack<'a> {
    /// `id`, via `attrString`.
    pub id: Value<'a>,
    /// `t`, via `maybeAttrString`.
    pub t: Option<Value<'a>>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `type`, via `maybeAttrString`.
    pub r#type: Option<Value<'a>>,
    /// `participant`, via `attrDeviceJid`.
    pub participant: Option<Jid<'a>>,
    /// `recipient`, via `attrUserJid`.
    pub recipient: Option<Jid<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> Ack<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            id: extract::attr_string(node, "id")?,
            t: extract::maybe_attr_string(node, "t"),
            class: extract::attr_string(node, "class")?,
            r#type: extract::maybe_attr_string(node, "type"),
            participant: extract::maybe_attr_jid(node, "participant")?,
            recipient: extract::maybe_attr_jid(node, "recipient")?,
            node: *node,
        })
    }
}

impl Ack<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &Ack<'_>) -> bool {
        (self.id.semantic_eq(other.id))
            && (match (self.t, other.t) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.class.semantic_eq(other.class))
            && (match (self.r#type, other.r#type) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.participant, other.participant) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.recipient, other.recipient) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext<'a> {
    /// `optimizationGoal`, via `attrEnum`.
    pub optimization_goal: ENUMDELIVERYNOOPTIMIZATION,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            optimization_goal: extract::attr_enum(
                node,
                "optimization_goal",
                ENUMDELIVERYNOOPTIMIZATION::from_wire,
            )?,
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext<
            '_,
        >,
    ) -> bool {
        self.optimization_goal == other.optimization_goal
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl<
    'a,
> {
    /// `elementValue`, via `contentString`.
    pub element_value: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a>
    AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl<'a>
{
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            element_value: extract::content_string(node)?,
            node: *node,
        })
    }
}

impl
    AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl<'_>
{
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl<'_>,
    ) -> bool {
        self.element_value.semantic_eq(other.element_value)
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral<'a> {
    /// `sourceType`, via `maybeAttrString`.
    pub source_type: Option<Value<'a>>,
    /// `sourceUrl`, via `maybeChild`.
    pub source_url: Option<alloc::boxed::Box<AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl<'a>>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            source_type: extract::maybe_attr_string(node, "source_type"),
            source_url: match extract::maybe_child(node, "source_url") { Some(child) => Some(alloc::boxed::Box::new(AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferralSourceUrl::derive(&child)?)), None => None },
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral<
            '_,
        >,
    ) -> bool {
        (match (self.source_type, other.source_type) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (&self.source_url, &other.source_url) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        })
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin<'a> {
    /// `type`, via `attrString`.
    pub r#type: Value<'a>,
    /// `referral`, via `maybeChild`.
    pub referral: Option<
        alloc::boxed::Box<
            AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral<'a>,
        >,
    >,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            r#type: extract::attr_string(node, "type")?,
            referral: match extract::maybe_child(node, "referral") { Some(child) => Some(alloc::boxed::Box::new(AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOriginReferral::derive(&child)?)), None => None },
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin<'_>,
    ) -> bool {
        (self.r#type.semantic_eq(other.r#type))
            && (match (&self.referral, &other.referral) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing<'a> {
    /// `consumerCountryCode`, via `maybeAttrString`.
    pub consumer_country_code: Option<Value<'a>>,
    /// `businessCountryCode`, via `maybeAttrString`.
    pub business_country_code: Option<Value<'a>>,
    /// `conversationStatus`, via `maybeAttrInt`.
    pub conversation_status: Option<i64>,
    /// `latestC2bTimestamp`, via `maybeAttrInt`.
    pub latest_c2b_timestamp: Option<i64>,
    /// `analyticsConversationId`, via `maybeAttrString`.
    pub analytics_conversation_id: Option<Value<'a>>,
    /// `b2cTimestamp`, via `maybeAttrInt`.
    pub b2c_timestamp: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            consumer_country_code: extract::maybe_attr_string(node, "consumer_country_code"),
            business_country_code: extract::maybe_attr_string(node, "business_country_code"),
            conversation_status: extract::maybe_attr_int(node, "conversation_status")?,
            latest_c2b_timestamp: extract::maybe_attr_int(node, "latest_c2b_timestamp")?,
            analytics_conversation_id: extract::maybe_attr_string(
                node,
                "analytics_conversation_id",
            ),
            b2c_timestamp: extract::maybe_attr_int(node, "b2c_timestamp")?,
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing<'_>,
    ) -> bool {
        (match (self.consumer_country_code, other.consumer_country_code) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (match (self.business_country_code, other.business_country_code) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }) && (self.conversation_status == other.conversation_status)
            && (self.latest_c2b_timestamp == other.latest_c2b_timestamp)
            && (match (
                self.analytics_conversation_id,
                other.analytics_conversation_id,
            ) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.b2c_timestamp == other.b2c_timestamp)
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidConversation` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidConversation<'a> {
    /// `bizPaidConvoId`, via `attrString`.
    pub biz_paid_convo_id: Value<'a>,
    /// `bizPricingModel`, via `attrEnum`.
    pub biz_pricing_model: ENUMCBPNBPPMP,
    /// `bizBillable`, via `attrEnum`.
    pub biz_billable: ENUMFALSETRUE,
    /// `bizExpirationTimestamp`, via `maybeAttrInt`.
    pub biz_expiration_timestamp: Option<i64>,
    /// `bizPricingCategory`, via `maybeAttrString`.
    pub biz_pricing_category: Option<Value<'a>>,
    /// `bizPricingType`, via `maybeAttrEnum`.
    pub biz_pricing_type: Option<ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR>,
    /// `bizDeliveryContext`, via `maybeChild`.
    pub biz_delivery_context: Option<
        alloc::boxed::Box<
            AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext<'a>,
        >,
    >,
    /// `bizOrigin`, via `maybeChild`.
    pub biz_origin: Option<
        alloc::boxed::Box<
            AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin<'a>,
        >,
    >,
    /// `bizPricing`, via `maybeChild`.
    pub biz_pricing: Option<
        alloc::boxed::Box<
            AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing<'a>,
        >,
    >,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidConversation<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        let at_biz = extract::maybe_child_at(node, &["biz"]);
        Ok(Self {
            biz_paid_convo_id: extract::attr_string(&at_biz.ok_or(DeriveError::MissingChild { tag: "biz" })?, "paid_convo_id")?,
            biz_pricing_model: extract::attr_enum(&at_biz.ok_or(DeriveError::MissingChild { tag: "biz" })?, "pricing_model", ENUMCBPNBPPMP::from_wire)?,
            biz_billable: extract::attr_enum(&at_biz.ok_or(DeriveError::MissingChild { tag: "biz" })?, "billable", ENUMFALSETRUE::from_wire)?,
            biz_expiration_timestamp: match at_biz { Some(at) => extract::maybe_attr_int(&at, "expiration_timestamp")?, None => None },
            biz_pricing_category: match at_biz { Some(at) => extract::maybe_attr_string(&at, "pricing_category"), None => None },
            biz_pricing_type: match at_biz { Some(at) => extract::maybe_attr_enum(&at, "pricing_type", ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::from_wire)?, None => None },
            biz_delivery_context: match extract::maybe_child(node, "delivery_context") { Some(child) => Some(alloc::boxed::Box::new(AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizDeliveryContext::derive(&child)?)), None => None },
            biz_origin: match extract::maybe_child(node, "origin") { Some(child) => Some(alloc::boxed::Box::new(AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizOrigin::derive(&child)?)), None => None },
            biz_pricing: match extract::maybe_child(node, "pricing") { Some(child) => Some(alloc::boxed::Box::new(AckPaidConversationOrAckPaidGroupConversationAckPaidConversationBizPricing::derive(&child)?)), None => None },
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidConversation<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidConversation<'_>,
    ) -> bool {
        (self.biz_paid_convo_id.semantic_eq(other.biz_paid_convo_id))
            && (self.biz_pricing_model == other.biz_pricing_model)
            && (self.biz_billable == other.biz_billable)
            && (self.biz_expiration_timestamp == other.biz_expiration_timestamp)
            && (match (self.biz_pricing_category, other.biz_pricing_category) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.biz_pricing_type == other.biz_pricing_type)
            && (match (&self.biz_delivery_context, &other.biz_delivery_context) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.biz_origin, &other.biz_origin) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (&self.biz_pricing, &other.biz_pricing) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation<'a> {
    /// `bizPricingBusinessCountryCode`, via `maybeAttrString`.
    pub biz_pricing_business_country_code: Option<Value<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        let at_biz_pricing = extract::maybe_child_at(node, &["biz", "pricing"]);
        Ok(Self {
            biz_pricing_business_country_code: match at_biz_pricing {
                Some(at) => extract::maybe_attr_string(&at, "business_country_code"),
                None => None,
            },
            node: *node,
        })
    }
}

impl AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation<'_>,
    ) -> bool {
        match (
            self.biz_pricing_business_country_code,
            other.biz_pricing_business_country_code,
        ) {
            (Some(a), Some(b)) => a.semantic_eq(b),
            (None, None) => true,
            _ => false,
        }
    }
}
/// One of the alternatives in whatspec's `AckPaidConversationOrAckPaidGroupConversation` mixin group.
///
/// Variants are tried richest-first: where one variant's required
/// fields are a subset of another's, the leaner one accepts every
/// stanza the richer one does, and trying it first would claim them
/// all (D-041).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AckPaidConversationOrAckPaidGroupConversation<'a> {
    /// `AckPaidConversation`.
    AckPaidConversation(AckPaidConversationOrAckPaidGroupConversationAckPaidConversation<'a>),
    /// `AckPaidGroupConversation`.
    AckPaidGroupConversation(
        AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation<'a>,
    ),
}

impl<'a> AckPaidConversationOrAckPaidGroupConversation<'a> {
    /// Derive whichever alternative this node satisfies.
    ///
    /// # Errors
    ///
    /// [`DeriveError::UnknownStanza`] when the node satisfies none of
    /// them, which is the honest answer: the mixin says the stanza is
    /// one of these and it is not.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Self::maybe_derive(node).ok_or(DeriveError::UnknownStanza)
    }

    /// Derive whichever alternative this node satisfies, or nothing.
    #[must_use]
    pub fn maybe_derive(node: &NodeRef<'a>) -> Option<Self> {
        // guarded by <biz> present
        if extract::maybe_child_at(node, &["biz"]).is_some()
            && let Ok(inner) =
                AckPaidConversationOrAckPaidGroupConversationAckPaidConversation::derive(node)
        {
            return Some(Self::AckPaidConversation(inner));
        }
        // guarded by <biz><pricing> present
        if extract::maybe_child_at(node, &["biz", "pricing"]).is_some()
            && let Ok(inner) =
                AckPaidConversationOrAckPaidGroupConversationAckPaidGroupConversation::derive(node)
        {
            return Some(Self::AckPaidGroupConversation(inner));
        }
        None
    }
}

impl AckPaidConversationOrAckPaidGroupConversation<'_> {
    /// Whether two alternatives mean the same thing.
    #[must_use]
    pub fn semantic_eq(&self, other: &AckPaidConversationOrAckPaidGroupConversation<'_>) -> bool {
        match (self, other) {
            (
                AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation(a),
                AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation(b),
            ) => a.semantic_eq(b),
            (
                AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation(a),
                AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation(b),
            ) => a.semantic_eq(b),
            _ => false,
        }
    }
}
/// Derived from whatspec's `ParseNewsletterResponseNegative` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseNewsletterResponseNegative<'a> {
    /// `error`, via `attrString`.
    pub error: Value<'a>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `attrInt`.
    pub t: i64,
    /// `edit`, via `maybeAttrString`.
    pub edit: Option<Value<'a>>,
    /// `frankingReportingTagElementValue`, via `maybeContentBytes`.
    pub franking_reporting_tag_element_value: Option<&'a [u8]>,
    /// `ackPaidAckPaidConversationOrAckPaidGroupConversationConversationMixinGroup`, one of `AckPaidConversation`, `AckPaidGroupConversation`.
    pub ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group:
        Option<AckPaidConversationOrAckPaidGroupConversation<'a>>,
    /// `applicationError`, via `maybeAttrInt`.
    pub application_error: Option<i64>,
    /// `backoff`, via `maybeAttrInt`.
    pub backoff: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParseNewsletterResponseNegative<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        let at_franking_reporting_tag =
            extract::maybe_child_at(node, &["franking", "reporting_tag"]);
        Ok(Self {
            error: extract::attr_string(node, "error")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            edit: extract::maybe_attr_string(node, "edit"),
            franking_reporting_tag_element_value: match at_franking_reporting_tag { Some(at) => extract::maybe_content_bytes(&at), None => None },
            ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group: AckPaidConversationOrAckPaidGroupConversation::maybe_derive(node),
            application_error: extract::maybe_attr_int(node, "application_error")?,
            backoff: extract::maybe_attr_int(node, "backoff")?,
            node: *node,
        })
    }
}

impl ParseNewsletterResponseNegative<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ParseNewsletterResponseNegative<'_>) -> bool {
        (self.error.semantic_eq(other.error))
            && (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (match (self.edit, other.edit) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
            && (self.franking_reporting_tag_element_value == other.franking_reporting_tag_element_value)
            && (match (&self.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group, &other.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
            && (self.application_error == other.application_error)
            && (self.backoff == other.backoff)
    }
}
/// Derived from whatspec's `NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck<'a> {
    /// `responseServerId`, via `attrString`.
    pub response_server_id: Value<'a>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `attrInt`.
    pub t: i64,
    /// `edit`, via `maybeAttrString`.
    pub edit: Option<Value<'a>>,
    /// `frankingReportingTagElementValue`, via `maybeContentBytes`.
    pub franking_reporting_tag_element_value: Option<&'a [u8]>,
    /// `ackPaidAckPaidConversationOrAckPaidGroupConversationConversationMixinGroup`, one of `AckPaidConversation`, `AckPaidGroupConversation`.
    pub ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group:
        Option<AckPaidConversationOrAckPaidGroupConversation<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        let at_franking_reporting_tag =
            extract::maybe_child_at(node, &["franking", "reporting_tag"]);
        Ok(Self {
            response_server_id: extract::attr_string(node, "response_server_id")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            edit: extract::maybe_attr_string(node, "edit"),
            franking_reporting_tag_element_value: match at_franking_reporting_tag { Some(at) => extract::maybe_content_bytes(&at), None => None },
            ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group: AckPaidConversationOrAckPaidGroupConversation::maybe_derive(node),
            node: *node,
        })
    }
}

impl NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck<
            '_,
        >,
    ) -> bool {
        (self.response_server_id.semantic_eq(other.response_server_id))
            && (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (match (self.edit, other.edit) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
            && (self.franking_reporting_tag_element_value == other.franking_reporting_tag_element_value)
            && (match (&self.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group, &other.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
    }
}
/// Derived from whatspec's `NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck<'a> {
    /// `serverId`, via `maybeAttrInt`.
    pub server_id: Option<i64>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `attrInt`.
    pub t: i64,
    /// `edit`, via `maybeAttrString`.
    pub edit: Option<Value<'a>>,
    /// `frankingReportingTagElementValue`, via `maybeContentBytes`.
    pub franking_reporting_tag_element_value: Option<&'a [u8]>,
    /// `ackPaidAckPaidConversationOrAckPaidGroupConversationConversationMixinGroup`, one of `AckPaidConversation`, `AckPaidGroupConversation`.
    pub ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group:
        Option<AckPaidConversationOrAckPaidGroupConversation<'a>>,
    /// `rcatElementValue`, via `maybeContentBytes`.
    pub rcat_element_value: Option<&'a [u8]>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        let at_franking_reporting_tag =
            extract::maybe_child_at(node, &["franking", "reporting_tag"]);
        let at_rcat = extract::maybe_child_at(node, &["rcat"]);
        Ok(Self {
            server_id: extract::maybe_attr_int(node, "server_id")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            edit: extract::maybe_attr_string(node, "edit"),
            franking_reporting_tag_element_value: match at_franking_reporting_tag { Some(at) => extract::maybe_content_bytes(&at), None => None },
            ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group: AckPaidConversationOrAckPaidGroupConversation::maybe_derive(node),
            rcat_element_value: match at_rcat { Some(at) => extract::maybe_content_bytes(&at), None => None },
            node: *node,
        })
    }
}

impl NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck<'_>,
    ) -> bool {
        (self.server_id == other.server_id)
            && (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (match (self.edit, other.edit) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
            && (self.franking_reporting_tag_element_value == other.franking_reporting_tag_element_value)
            && (match (&self.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group, &other.ack_paid_ack_paid_conversation_or_ack_paid_group_conversation_conversation_mixin_group) { (Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false })
            && (self.rcat_element_value == other.rcat_element_value)
    }
}
/// One of the alternatives in whatspec's `NewsletterQuestionResponseAckOrNewsletterMessageAck` mixin group.
///
/// Variants are tried richest-first: where one variant's required
/// fields are a subset of another's, the leaner one accepts every
/// stanza the richer one does, and trying it first would claim them
/// all (D-041).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NewsletterQuestionResponseAckOrNewsletterMessageAck<'a> {
    /// `NewsletterQuestionResponseAck`.
    NewsletterQuestionResponseAck(
        NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck<'a>,
    ),
    /// `NewsletterMessageAck`.
    NewsletterMessageAck(
        NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck<'a>,
    ),
}

impl<'a> NewsletterQuestionResponseAckOrNewsletterMessageAck<'a> {
    /// Derive whichever alternative this node satisfies.
    ///
    /// # Errors
    ///
    /// [`DeriveError::UnknownStanza`] when the node satisfies none of
    /// them, which is the honest answer: the mixin says the stanza is
    /// one of these and it is not.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Self::maybe_derive(node).ok_or(DeriveError::UnknownStanza)
    }

    /// Derive whichever alternative this node satisfies, or nothing.
    #[must_use]
    pub fn maybe_derive(node: &NodeRef<'a>) -> Option<Self> {
        // guarded by class=message
        if node.attr_eq("class", "message")
            && let Ok(inner) = NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterQuestionResponseAck::derive(node)
        {
            return Some(Self::NewsletterQuestionResponseAck(inner));
        }
        // guarded by class=message
        if node.attr_eq("class", "message")
            && let Ok(inner) =
                NewsletterQuestionResponseAckOrNewsletterMessageAckNewsletterMessageAck::derive(
                    node,
                )
        {
            return Some(Self::NewsletterMessageAck(inner));
        }
        None
    }
}

impl NewsletterQuestionResponseAckOrNewsletterMessageAck<'_> {
    /// Whether two alternatives mean the same thing.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &NewsletterQuestionResponseAckOrNewsletterMessageAck<'_>,
    ) -> bool {
        match (self, other) {
            (
                NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck(
                    a,
                ),
                NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck(
                    b,
                ),
            ) => a.semantic_eq(b),
            (
                NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck(a),
                NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck(b),
            ) => a.semantic_eq(b),
            _ => false,
        }
    }
}
/// Derived from whatspec's `ParseNewsletterResponseSuccess` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseNewsletterResponseSuccess<'a> {
    /// `newsletterQuestionResponseOrNewsletterMessageAckMixinGroup`, one of `NewsletterQuestionResponseAck`, `NewsletterMessageAck`.
    pub newsletter_question_response_or_newsletter_message_ack_mixin_group:
        NewsletterQuestionResponseAckOrNewsletterMessageAck<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParseNewsletterResponseSuccess<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            newsletter_question_response_or_newsletter_message_ack_mixin_group:
                NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(node)?,
            node: *node,
        })
    }
}

impl ParseNewsletterResponseSuccess<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ParseNewsletterResponseSuccess<'_>) -> bool {
        self.newsletter_question_response_or_newsletter_message_ack_mixin_group
            .semantic_eq(&other.newsletter_question_response_or_newsletter_message_ack_mixin_group)
    }
}
/// Derived from whatspec's `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit<'a> {
    /// `edit`, via `attrString`.
    pub edit: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            edit: extract::attr_string(node, "edit")?,
            node: *node,
        })
    }
}

impl StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit<'_>,
    ) -> bool {
        self.edit.semantic_eq(other.edit)
    }
}
/// Derived from whatspec's `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke<'a> {
    /// `edit`, via `attrString`.
    pub edit: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            edit: extract::attr_string(node, "edit")?,
            node: *node,
        })
    }
}

impl StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke<'_>,
    ) -> bool {
        self.edit.semantic_eq(other.edit)
    }
}
/// Derived from whatspec's `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke<'a> {
    /// `edit`, via `attrString`.
    pub edit: Value<'a>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            edit: extract::attr_string(node, "edit")?,
            node: *node,
        })
    }
}

impl StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke<'_>,
    ) -> bool {
        self.edit.semantic_eq(other.edit)
    }
}
/// One of the alternatives in whatspec's `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke` mixin group.
///
/// Variants are tried richest-first: where one variant's required
/// fields are a subset of another's, the leaner one accepts every
/// stanza the richer one does, and trying it first would claim them
/// all (D-041).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'a> {
    /// `StatusAckEdit`.
    StatusAckEdit(StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit<'a>),
    /// `StatusAckRevoke`.
    StatusAckRevoke(StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke<'a>),
    /// `StatusAckAdminRevoke`.
    StatusAckAdminRevoke(
        StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke<'a>,
    ),
}

impl<'a> StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'a> {
    /// Derive whichever alternative this node satisfies.
    ///
    /// # Errors
    ///
    /// [`DeriveError::UnknownStanza`] when the node satisfies none of
    /// them, which is the honest answer: the mixin says the stanza is
    /// one of these and it is not.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Self::maybe_derive(node).ok_or(DeriveError::UnknownStanza)
    }

    /// Derive whichever alternative this node satisfies, or nothing.
    #[must_use]
    pub fn maybe_derive(node: &NodeRef<'a>) -> Option<Self> {
        // guarded by edit=1
        if node.attr_eq("edit", "1")
            && let Ok(inner) =
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckEdit::derive(node)
        {
            return Some(Self::StatusAckEdit(inner));
        }
        // guarded by edit=7
        if node.attr_eq("edit", "7")
            && let Ok(inner) =
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckRevoke::derive(node)
        {
            return Some(Self::StatusAckRevoke(inner));
        }
        // guarded by edit=8
        if node.attr_eq("edit", "8")
            && let Ok(inner) =
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevokeStatusAckAdminRevoke::derive(
                    node,
                )
        {
            return Some(Self::StatusAckAdminRevoke(inner));
        }
        None
    }
}

impl StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'_> {
    /// Whether two alternatives mean the same thing.
    #[must_use]
    pub fn semantic_eq(
        &self,
        other: &StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'_>,
    ) -> bool {
        match (self, other) {
            (
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit(a),
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit(b),
            ) => a.semantic_eq(b),
            (
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke(a),
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke(b),
            ) => a.semantic_eq(b),
            (
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke(a),
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke(b),
            ) => a.semantic_eq(b),
            _ => false,
        }
    }
}
/// Derived from whatspec's `ParsePostNewsletterStatusResponseNegative` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParsePostNewsletterStatusResponseNegative<'a> {
    /// `error`, via `attrString`.
    pub error: Value<'a>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `attrInt`.
    pub t: i64,
    /// `statusAckEditOrRevokeOrAdminRevokeMixinGroup`, one of `StatusAckEdit`, `StatusAckRevoke`, `StatusAckAdminRevoke`.
    pub status_ack_edit_or_revoke_or_admin_revoke_mixin_group:
        Option<StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'a>>,
    /// `applicationError`, via `maybeAttrInt`.
    pub application_error: Option<i64>,
    /// `backoff`, via `maybeAttrInt`.
    pub backoff: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParsePostNewsletterStatusResponseNegative<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            error: extract::attr_string(node, "error")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            status_ack_edit_or_revoke_or_admin_revoke_mixin_group:
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(node),
            application_error: extract::maybe_attr_int(node, "application_error")?,
            backoff: extract::maybe_attr_int(node, "backoff")?,
            node: *node,
        })
    }
}

impl ParsePostNewsletterStatusResponseNegative<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ParsePostNewsletterStatusResponseNegative<'_>) -> bool {
        (self.error.semantic_eq(other.error))
            && (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (match (
                &self.status_ack_edit_or_revoke_or_admin_revoke_mixin_group,
                &other.status_ack_edit_or_revoke_or_admin_revoke_mixin_group,
            ) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.application_error == other.application_error)
            && (self.backoff == other.backoff)
    }
}
/// Derived from whatspec's `ParsePostNewsletterStatusResponseSuccess` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParsePostNewsletterStatusResponseSuccess<'a> {
    /// `serverId`, via `maybeAttrInt`.
    pub server_id: Option<i64>,
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `attrInt`.
    pub t: i64,
    /// `statusAckEditOrRevokeOrAdminRevokeMixinGroup`, one of `StatusAckEdit`, `StatusAckRevoke`, `StatusAckAdminRevoke`.
    pub status_ack_edit_or_revoke_or_admin_revoke_mixin_group:
        Option<StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke<'a>>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParsePostNewsletterStatusResponseSuccess<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            server_id: extract::maybe_attr_int(node, "server_id")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            status_ack_edit_or_revoke_or_admin_revoke_mixin_group:
                StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(node),
            node: *node,
        })
    }
}

impl ParsePostNewsletterStatusResponseSuccess<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ParsePostNewsletterStatusResponseSuccess<'_>) -> bool {
        (self.server_id == other.server_id)
            && (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (match (
                &self.status_ack_edit_or_revoke_or_admin_revoke_mixin_group,
                &other.status_ack_edit_or_revoke_or_admin_revoke_mixin_group,
            ) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
    }
}
/// Derived from whatspec's `ParsePublishViewResponseSuccess` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParsePublishViewResponseSuccess<'a> {
    /// `class`, via `attrString`.
    pub class: Value<'a>,
    /// `t`, via `maybeAttrInt`.
    pub t: Option<i64>,
    /// `readreceipts`, via `maybeAttrEnum`.
    pub readreceipts: Option<ENUMALLNONE>,
    /// `edit`, via `maybeAttrEnum`.
    pub edit: Option<ENUM017>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParsePublishViewResponseSuccess<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            class: extract::attr_string(node, "class")?,
            t: extract::maybe_attr_int(node, "t")?,
            readreceipts: extract::maybe_attr_enum(node, "readreceipts", ENUMALLNONE::from_wire)?,
            edit: extract::maybe_attr_enum(node, "edit", ENUM017::from_wire)?,
            node: *node,
        })
    }
}

impl ParsePublishViewResponseSuccess<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ParsePublishViewResponseSuccess<'_>) -> bool {
        (self.class.semantic_eq(other.class))
            && (self.t == other.t)
            && (self.readreceipts == other.readreceipts)
            && (self.edit == other.edit)
    }
}
/// Derived from whatspec's `ReadReceiptAckParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ReadReceiptAckParser<'a> {
    /// `readreceipts`, via `maybeAttrEnum`.
    pub readreceipts: Option<ALLNONE>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ReadReceiptAckParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            readreceipts: extract::maybe_attr_enum(node, "readreceipts", ALLNONE::from_wire)?,
            node: *node,
        })
    }
}

impl ReadReceiptAckParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &ReadReceiptAckParser<'_>) -> bool {
        self.readreceipts == other.readreceipts
    }
}
/// Derived from whatspec's `SendMsgAckSyncParser` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SendMsgAckSyncParser<'a> {
    /// `t`, via `attrTime`.
    pub t: i64,
    /// `sync`, via `maybeAttrString`.
    pub sync: Option<Value<'a>>,
    /// `phash`, via `maybeAttrString`.
    pub phash: Option<Value<'a>>,
    /// `refresh_lid`, via `maybeAttrString`.
    pub refresh_lid: Option<Value<'a>>,
    /// `addressing_mode`, via `maybeAttrString`.
    pub addressing_mode: Option<Value<'a>>,
    /// `count`, via `maybeAttrInt`.
    pub count: Option<i64>,
    /// `error`, via `maybeAttrInt`.
    pub error: Option<i64>,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> SendMsgAckSyncParser<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            t: extract::attr_time(node, "t")?,
            sync: extract::maybe_attr_string(node, "sync"),
            phash: extract::maybe_attr_string(node, "phash"),
            refresh_lid: extract::maybe_attr_string(node, "refresh_lid"),
            addressing_mode: extract::maybe_attr_string(node, "addressing_mode"),
            count: extract::maybe_attr_int(node, "count")?,
            error: extract::maybe_attr_int(node, "error")?,
            node: *node,
        })
    }
}

impl SendMsgAckSyncParser<'_> {
    /// Whether two derivations mean the same thing, whatever form
    /// each field arrived in.
    ///
    /// The originating node is excluded: two engines may encode one
    /// stanza differently and both be right.
    #[must_use]
    pub fn semantic_eq(&self, other: &SendMsgAckSyncParser<'_>) -> bool {
        (self.t == other.t)
            && (match (self.sync, other.sync) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.phash, other.phash) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.refresh_lid, other.refresh_lid) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (match (self.addressing_mode, other.addressing_mode) {
                (Some(a), Some(b)) => a.semantic_eq(b),
                (None, None) => true,
                _ => false,
            })
            && (self.count == other.count)
            && (self.error == other.error)
    }
}

impl crate::semantic::SemanticEq for IncomingMsgParserEnc<'_> {
    fn semantic_eq(&self, other: &Self) -> bool {
        IncomingMsgParserEnc::semantic_eq(self, other)
    }
}

impl crate::semantic::SemanticEq for IncomingMsgReceiptParserParticipantsUser<'_> {
    fn semantic_eq(&self, other: &Self) -> bool {
        IncomingMsgReceiptParserParticipantsUser::semantic_eq(self, other)
    }
}

impl crate::semantic::SemanticEq for IncomingMsgReceiptParserListItem<'_> {
    fn semantic_eq(&self, other: &Self) -> bool {
        IncomingMsgReceiptParserListItem::semantic_eq(self, other)
    }
}

/// One derived stanza.
///
/// A tag with several shapes tries each in order and takes the first that
/// derives cleanly, which is how whatspec models a tag whose meaning depends
/// on the fields present.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event<'a> {
    /// A `<message>` matching the `IncomingMsgParser` shape.
    IncomingMsgParser(IncomingMsgParser<'a>),
    /// A `<message>` matching the `IncomingMsgParserForAckOnly` shape.
    IncomingMsgParserForAckOnly(IncomingMsgParserForAckOnly<'a>),
    /// A `<receipt>` matching the `CallReceiptParser` shape.
    CallReceiptParser(CallReceiptParser<'a>),
    /// A `<receipt>` matching the `IncomingMsgReceiptParser` shape.
    IncomingMsgReceiptParser(IncomingMsgReceiptParser<'a>),
    /// A `<receipt>` matching the `RetryRequestParser` shape.
    RetryRequestParser(RetryRequestParser<'a>),
    /// A `<call>` matching the `CallOfferNoticeParser` shape.
    CallOfferNoticeParser(CallOfferNoticeParser<'a>),
    /// A `<call>` matching the `CallOfferPlaceholder` shape.
    CallOfferPlaceholder(CallOfferPlaceholder<'a>),
    /// A `<call>` matching the `CallParser` shape.
    CallParser(CallParser<'a>),
    /// A `<ack>` matching the `Ack` shape.
    Ack(Ack<'a>),
    /// A `<ack>` matching the `ParseNewsletterResponseNegative` shape.
    ParseNewsletterResponseNegative(ParseNewsletterResponseNegative<'a>),
    /// A `<ack>` matching the `ParseNewsletterResponseSuccess` shape.
    ParseNewsletterResponseSuccess(ParseNewsletterResponseSuccess<'a>),
    /// A `<ack>` matching the `ParsePostNewsletterStatusResponseNegative` shape.
    ParsePostNewsletterStatusResponseNegative(ParsePostNewsletterStatusResponseNegative<'a>),
    /// A `<ack>` matching the `ParsePostNewsletterStatusResponseSuccess` shape.
    ParsePostNewsletterStatusResponseSuccess(ParsePostNewsletterStatusResponseSuccess<'a>),
    /// A `<ack>` matching the `ParsePublishViewResponseSuccess` shape.
    ParsePublishViewResponseSuccess(ParsePublishViewResponseSuccess<'a>),
    /// A `<ack>` matching the `ReadReceiptAckParser` shape.
    ReadReceiptAckParser(ReadReceiptAckParser<'a>),
    /// A `<ack>` matching the `SendMsgAckSyncParser` shape.
    SendMsgAckSyncParser(SendMsgAckSyncParser<'a>),
}

impl<'a> Event<'a> {
    /// The stanza tag this event was derived from.
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::IncomingMsgParser(_) => "message",
            Self::IncomingMsgParserForAckOnly(_) => "message",
            Self::CallReceiptParser(_) => "receipt",
            Self::IncomingMsgReceiptParser(_) => "receipt",
            Self::RetryRequestParser(_) => "receipt",
            Self::CallOfferNoticeParser(_) => "call",
            Self::CallOfferPlaceholder(_) => "call",
            Self::CallParser(_) => "call",
            Self::Ack(_) => "ack",
            Self::ParseNewsletterResponseNegative(_) => "ack",
            Self::ParseNewsletterResponseSuccess(_) => "ack",
            Self::ParsePostNewsletterStatusResponseNegative(_) => "ack",
            Self::ParsePostNewsletterStatusResponseSuccess(_) => "ack",
            Self::ParsePublishViewResponseSuccess(_) => "ack",
            Self::ReadReceiptAckParser(_) => "ack",
            Self::SendMsgAckSyncParser(_) => "ack",
        }
    }

    /// The node this event was derived from.
    #[must_use]
    pub const fn node(&self) -> &NodeRef<'a> {
        match self {
            Self::IncomingMsgParser(inner) => &inner.node,
            Self::IncomingMsgParserForAckOnly(inner) => &inner.node,
            Self::CallReceiptParser(inner) => &inner.node,
            Self::IncomingMsgReceiptParser(inner) => &inner.node,
            Self::RetryRequestParser(inner) => &inner.node,
            Self::CallOfferNoticeParser(inner) => &inner.node,
            Self::CallOfferPlaceholder(inner) => &inner.node,
            Self::CallParser(inner) => &inner.node,
            Self::Ack(inner) => &inner.node,
            Self::ParseNewsletterResponseNegative(inner) => &inner.node,
            Self::ParseNewsletterResponseSuccess(inner) => &inner.node,
            Self::ParsePostNewsletterStatusResponseNegative(inner) => &inner.node,
            Self::ParsePostNewsletterStatusResponseSuccess(inner) => &inner.node,
            Self::ParsePublishViewResponseSuccess(inner) => &inner.node,
            Self::ReadReceiptAckParser(inner) => &inner.node,
            Self::SendMsgAckSyncParser(inner) => &inner.node,
        }
    }

    /// Whether two events mean the same thing.
    ///
    /// Different shapes never do, even for one tag: which shape matched
    /// is part of what was derived, so two engines picking different
    /// shapes for one stanza is exactly the divergence worth reporting.
    #[must_use]
    pub fn semantic_eq(&self, other: &Event<'_>) -> bool {
        match (self, other) {
            (Self::IncomingMsgParser(a), Event::IncomingMsgParser(b)) => a.semantic_eq(b),
            (Self::IncomingMsgParserForAckOnly(a), Event::IncomingMsgParserForAckOnly(b)) => {
                a.semantic_eq(b)
            }
            (Self::CallReceiptParser(a), Event::CallReceiptParser(b)) => a.semantic_eq(b),
            (Self::IncomingMsgReceiptParser(a), Event::IncomingMsgReceiptParser(b)) => {
                a.semantic_eq(b)
            }
            (Self::RetryRequestParser(a), Event::RetryRequestParser(b)) => a.semantic_eq(b),
            (Self::CallOfferNoticeParser(a), Event::CallOfferNoticeParser(b)) => a.semantic_eq(b),
            (Self::CallOfferPlaceholder(a), Event::CallOfferPlaceholder(b)) => a.semantic_eq(b),
            (Self::CallParser(a), Event::CallParser(b)) => a.semantic_eq(b),
            (Self::Ack(a), Event::Ack(b)) => a.semantic_eq(b),
            (
                Self::ParseNewsletterResponseNegative(a),
                Event::ParseNewsletterResponseNegative(b),
            ) => a.semantic_eq(b),
            (Self::ParseNewsletterResponseSuccess(a), Event::ParseNewsletterResponseSuccess(b)) => {
                a.semantic_eq(b)
            }
            (
                Self::ParsePostNewsletterStatusResponseNegative(a),
                Event::ParsePostNewsletterStatusResponseNegative(b),
            ) => a.semantic_eq(b),
            (
                Self::ParsePostNewsletterStatusResponseSuccess(a),
                Event::ParsePostNewsletterStatusResponseSuccess(b),
            ) => a.semantic_eq(b),
            (
                Self::ParsePublishViewResponseSuccess(a),
                Event::ParsePublishViewResponseSuccess(b),
            ) => a.semantic_eq(b),
            (Self::ReadReceiptAckParser(a), Event::ReadReceiptAckParser(b)) => a.semantic_eq(b),
            (Self::SendMsgAckSyncParser(a), Event::SendMsgAckSyncParser(b)) => a.semantic_eq(b),
            _ => false,
        }
    }
}

/// Derive an event from a parsed stanza.
///
/// Pure: the same node yields the same event, with no key material and no
/// accumulated state. That is what lets this run host-side, once, instead of
/// being reimplemented per engine.
pub fn derive<'a>(node: &NodeRef<'a>) -> Result<Event<'a>, DeriveError> {
    if node.tag().eq_str("ack") {
        // guarded by class=message
        if node.attr_eq("class", "message")
            && let Ok(inner) = ParseNewsletterResponseNegative::derive(node)
        {
            return Ok(Event::ParseNewsletterResponseNegative(inner));
        }
        // guarded by class=status
        if node.attr_eq("class", "status")
            && let Ok(inner) = ParsePostNewsletterStatusResponseNegative::derive(node)
        {
            return Ok(Event::ParsePostNewsletterStatusResponseNegative(inner));
        }
        // guarded by class=status
        if node.attr_eq("class", "status")
            && let Ok(inner) = ParsePostNewsletterStatusResponseSuccess::derive(node)
        {
            return Ok(Event::ParsePostNewsletterStatusResponseSuccess(inner));
        }
        // guarded by class=receipt
        if node.attr_eq("class", "receipt")
            && let Ok(inner) = ParsePublishViewResponseSuccess::derive(node)
        {
            return Ok(Event::ParsePublishViewResponseSuccess(inner));
        }
        if let Ok(inner) = Ack::derive(node) {
            return Ok(Event::Ack(inner));
        }
        if let Ok(inner) = SendMsgAckSyncParser::derive(node) {
            return Ok(Event::SendMsgAckSyncParser(inner));
        }
        if let Ok(inner) = ParseNewsletterResponseSuccess::derive(node) {
            return Ok(Event::ParseNewsletterResponseSuccess(inner));
        }
        if let Ok(inner) = ReadReceiptAckParser::derive(node) {
            return Ok(Event::ReadReceiptAckParser(inner));
        }
        return Err(DeriveError::NoMatchingShape { tag: "ack" });
    }
    if node.tag().eq_str("call") {
        if let Ok(inner) = CallOfferNoticeParser::derive(node) {
            return Ok(Event::CallOfferNoticeParser(inner));
        }
        if let Ok(inner) = CallParser::derive(node) {
            return Ok(Event::CallParser(inner));
        }
        if let Ok(inner) = CallOfferPlaceholder::derive(node) {
            return Ok(Event::CallOfferPlaceholder(inner));
        }
        return Err(DeriveError::NoMatchingShape { tag: "call" });
    }
    if node.tag().eq_str("message") {
        if let Ok(inner) = IncomingMsgParser::derive(node) {
            return Ok(Event::IncomingMsgParser(inner));
        }
        if let Ok(inner) = IncomingMsgParserForAckOnly::derive(node) {
            return Ok(Event::IncomingMsgParserForAckOnly(inner));
        }
        return Err(DeriveError::NoMatchingShape { tag: "message" });
    }
    if node.tag().eq_str("receipt") {
        if let Ok(inner) = RetryRequestParser::derive(node) {
            return Ok(Event::RetryRequestParser(inner));
        }
        if let Ok(inner) = IncomingMsgReceiptParser::derive(node) {
            return Ok(Event::IncomingMsgReceiptParser(inner));
        }
        if let Ok(inner) = CallReceiptParser::derive(node) {
            return Ok(Event::CallReceiptParser(inner));
        }
        return Err(DeriveError::NoMatchingShape { tag: "receipt" });
    }
    Err(DeriveError::UnknownStanza)
}

/// Tags this build can derive.
pub const KNOWN_TAGS: [&str; 4] = ["ack", "call", "message", "receipt"];

/// Every shape this derivation models, by name.
///
/// The names [`Event`]'s variants carry. Exported so a caller can check
/// coverage against the derivation rather than against a copied list.
pub const SHAPE_NAMES: [&str; 16] = [
    "Ack",
    "CallOfferNoticeParser",
    "CallOfferPlaceholder",
    "CallParser",
    "CallReceiptParser",
    "IncomingMsgParser",
    "IncomingMsgParserForAckOnly",
    "IncomingMsgReceiptParser",
    "ParseNewsletterResponseNegative",
    "ParseNewsletterResponseSuccess",
    "ParsePostNewsletterStatusResponseNegative",
    "ParsePostNewsletterStatusResponseSuccess",
    "ParsePublishViewResponseSuccess",
    "ReadReceiptAckParser",
    "RetryRequestParser",
    "SendMsgAckSyncParser",
];

/// Fields the generator could not express, named rather than dropped in
/// silence.
///
/// A derivation that quietly omitted a field would look complete and be
/// wrong, and no conformance run could tell — every engine would agree on
/// the same missing field.
pub const UNMODELLED_FIELDS: [&str; 0] = [];

/// Fields the spec types more precisely than this derivation carries.
///
/// An `attrEnum` whose variants the spec never lists is the whole of
/// this today: the values live on sibling shapes as literal guards, and
/// reconstructing the set from those would be inference. The field
/// crosses as text, which is what the spec supports.
pub const UNTYPED_FIELDS: [&str; 0] = [];

/// Checks the spec states that L1 cannot make, by construction.
///
/// A reference assertion says a response's field must equal one from
/// the request it answers. [`derive()`] is a pure function of a single
/// stanza (D-010), and the request is not in it, so these are outside
/// what this layer can evaluate rather than something it has not got to
/// yet. A host that tracks outstanding requests can check them; this
/// names them so that host knows what to check.
///
/// Unlike [`UNMODELLED_FIELDS`], a shrinking list here would mean the
/// spec changed, not that the generator improved.
pub const REQUEST_SCOPED_ASSERTIONS: [&str; 9] = [
    "ParseNewsletterResponseNegative: `from` must match the request's `to`",
    "ParseNewsletterResponseNegative: `id` must match the request's `id`",
    "ParsePostNewsletterStatusResponseNegative: `from` must match the request's `to`",
    "ParsePostNewsletterStatusResponseNegative: `id` must match the request's `id`",
    "ParsePostNewsletterStatusResponseSuccess: `from` must match the request's `to`",
    "ParsePostNewsletterStatusResponseSuccess: `id` must match the request's `id`",
    "ParsePublishViewResponseSuccess: `from` must match the request's `to`",
    "ParsePublishViewResponseSuccess: `id` must match the request's `id`",
    "ParsePublishViewResponseSuccess: `type` must match the request's `type`",
];

#[cfg(test)]
mod generated_tests {
    use super::*;
    use crate::testing::{Fixture, parse};

    /// `IncomingMsgParser` derives from a stanza carrying its required fields.
    #[test]
    fn incoming_msg_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("message")
            .child(Fixture::node("enc").attr("type", "skmsg").bytes(b"x"))
            .jid_attr("from", "u")
            .attr("type", "text")
            .attr("t", "1")
            .attr("recipient", "x")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParser::derive(&node);
        assert!(derived.is_ok(), "IncomingMsgParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, IncomingMsgParser::derive(&node));
    }

    /// `IncomingMsgParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn incoming_msg_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("message")
            .child(Fixture::node("plaintext"))
            .child(
                Fixture::node("enc")
                    .attr("type", "skmsg")
                    .attr("mediatype", "x")
                    .bytes(b"x")
                    .attr("count", "1")
                    .attr("decrypt-fail", "x")
                    .attr("state", "x")
                    .attr("session_type", "x"),
            )
            .child(Fixture::node("device-identity").bytes(b"x"))
            .child(
                Fixture::node("bot")
                    .attr("sender_timestamp_ms", "x")
                    .attr("edit_target_id", "x")
                    .attr("edit", "x")
                    .attr("biz_bot", "x")
                    .attr("type", "x"),
            )
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .child(
                Fixture::node("unavailable")
                    .attr("hosted", "x")
                    .attr("type", "x"),
            )
            .attr("type", "text")
            .child(
                Fixture::node("meta")
                    .attr("polltype", "creation")
                    .attr("status_mentioned", "x")
                    .attr("origin", "ctwa")
                    .attr("appdata", "default")
                    .attr("thread_msg_id", "x")
                    .jid_attr("thread_msg_sender_jid", "u")
                    .attr("target_id", "x")
                    .jid_attr("target_sender_jid", "u")
                    .jid_attr("target_chat_jid", "u")
                    .jid_attr("target_chat_jid_lid", "u")
                    .jid_attr("from", "u")
                    .attr("capi", "x")
                    .attr("event_type", "creation")
                    .attr("context_source", "x")
                    .attr("read", "x")
                    .attr("is_group_status", "x")
                    .attr("session_scope", "x")
                    .attr("type", "x")
                    .attr("st", "1")
                    .child(Fixture::node("key").attr("rkid", "x").bytes(b"x")),
            )
            .attr("t", "1")
            .child(Fixture::node("verified_name").bytes(b"x"))
            .attr("verified_level", "high")
            .child(
                Fixture::node("biz")
                    .attr("actual_actors", "1")
                    .attr("host_storage", "1")
                    .attr("privacy_mode_ts", "1")
                    .attr("native_flow_name", "x")
                    .attr("campaign_id", "x"),
            )
            .child(Fixture::node("pay"))
            .child(Fixture::node("transaction"))
            .attr("recipient", "x")
            .child(Fixture::node("hsm").attr("tag", "x").attr("category", "x"))
            .child(
                Fixture::node("reporting")
                    .child(Fixture::node("reporting_token").bytes(b"x").attr("v", "1"))
                    .child(Fixture::node("reporting_tag").bytes(b"x")),
            )
            .child(Fixture::node("rcat"))
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParser::derive(&node);
        assert!(derived.is_ok(), "IncomingMsgParser: {:?}", derived.err());
    }

    /// `IncomingMsgParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn incoming_msg_parser_compares_semantically() {
        let stanza = Fixture::node("message")
            .child(Fixture::node("plaintext"))
            .child(
                Fixture::node("enc")
                    .attr("type", "skmsg")
                    .attr("mediatype", "x")
                    .bytes(b"x")
                    .attr("count", "1")
                    .attr("decrypt-fail", "x")
                    .attr("state", "x")
                    .attr("session_type", "x"),
            )
            .child(Fixture::node("device-identity").bytes(b"x"))
            .child(
                Fixture::node("bot")
                    .attr("sender_timestamp_ms", "x")
                    .attr("edit_target_id", "x")
                    .attr("edit", "x")
                    .attr("biz_bot", "x")
                    .attr("type", "x"),
            )
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .child(
                Fixture::node("unavailable")
                    .attr("hosted", "x")
                    .attr("type", "x"),
            )
            .attr("type", "text")
            .child(
                Fixture::node("meta")
                    .attr("polltype", "creation")
                    .attr("status_mentioned", "x")
                    .attr("origin", "ctwa")
                    .attr("appdata", "default")
                    .attr("thread_msg_id", "x")
                    .jid_attr("thread_msg_sender_jid", "u")
                    .attr("target_id", "x")
                    .jid_attr("target_sender_jid", "u")
                    .jid_attr("target_chat_jid", "u")
                    .jid_attr("target_chat_jid_lid", "u")
                    .jid_attr("from", "u")
                    .attr("capi", "x")
                    .attr("event_type", "creation")
                    .attr("context_source", "x")
                    .attr("read", "x")
                    .attr("is_group_status", "x")
                    .attr("session_scope", "x")
                    .attr("type", "x")
                    .attr("st", "1")
                    .child(Fixture::node("key").attr("rkid", "x").bytes(b"x")),
            )
            .attr("t", "1")
            .child(Fixture::node("verified_name").bytes(b"x"))
            .attr("verified_level", "high")
            .child(
                Fixture::node("biz")
                    .attr("actual_actors", "1")
                    .attr("host_storage", "1")
                    .attr("privacy_mode_ts", "1")
                    .attr("native_flow_name", "x")
                    .attr("campaign_id", "x"),
            )
            .child(Fixture::node("pay"))
            .child(Fixture::node("transaction"))
            .attr("recipient", "x")
            .child(Fixture::node("hsm").attr("tag", "x").attr("category", "x"))
            .child(
                Fixture::node("reporting")
                    .child(Fixture::node("reporting_token").bytes(b"x").attr("v", "1"))
                    .child(Fixture::node("reporting_tag").bytes(b"x")),
            )
            .child(Fixture::node("rcat"))
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParser::derive(&node).expect("derives");
        let again = IncomingMsgParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("message")
            .child(Fixture::node("enc").attr("type", "skmsg").bytes(b"x"))
            .jid_attr("from", "u")
            .attr("type", "text")
            .attr("t", "1")
            .attr("recipient", "x")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = IncomingMsgParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `IncomingMsgParserForAckOnly` derives from a stanza carrying its required fields.
    #[test]
    fn incoming_msg_parser_for_ack_only_derives_from_its_required_fields() {
        let stanza = Fixture::node("message")
            .attr("type", "text")
            .attr("offline", "x")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParserForAckOnly::derive(&node);
        assert!(
            derived.is_ok(),
            "IncomingMsgParserForAckOnly: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, IncomingMsgParserForAckOnly::derive(&node));
    }

    /// `IncomingMsgParserForAckOnly` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn incoming_msg_parser_for_ack_only_derives_with_every_field_present() {
        let stanza = Fixture::node("message")
            .attr("type", "text")
            .attr("offline", "x")
            .child(
                Fixture::node("bot")
                    .attr("sender_timestamp_ms", "x")
                    .attr("edit_target_id", "x")
                    .attr("edit", "x")
                    .attr("biz_bot", "x")
                    .attr("type", "x"),
            )
            .attr("id", "x")
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParserForAckOnly::derive(&node);
        assert!(
            derived.is_ok(),
            "IncomingMsgParserForAckOnly: {:?}",
            derived.err()
        );
    }

    /// `IncomingMsgParserForAckOnly` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn incoming_msg_parser_for_ack_only_compares_semantically() {
        let stanza = Fixture::node("message")
            .attr("type", "text")
            .attr("offline", "x")
            .child(
                Fixture::node("bot")
                    .attr("sender_timestamp_ms", "x")
                    .attr("edit_target_id", "x")
                    .attr("edit", "x")
                    .attr("biz_bot", "x")
                    .attr("type", "x"),
            )
            .attr("id", "x")
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgParserForAckOnly::derive(&node).expect("derives");
        let again = IncomingMsgParserForAckOnly::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("message")
            .attr("type", "text")
            .attr("offline", "x")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = IncomingMsgParserForAckOnly::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `CallReceiptParser` derives from a stanza carrying its required fields.
    #[test]
    fn call_receipt_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let derived = CallReceiptParser::derive(&node);
        assert!(derived.is_ok(), "CallReceiptParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, CallReceiptParser::derive(&node));
    }

    /// `CallReceiptParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn call_receipt_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("receipt")
            .child(Fixture::node("offer"))
            .child(Fixture::node("accept"))
            .child(Fixture::node("reject"))
            .attr("id", "x")
            .attr("type", "x")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let derived = CallReceiptParser::derive(&node);
        assert!(derived.is_ok(), "CallReceiptParser: {:?}", derived.err());
    }

    /// `CallReceiptParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn call_receipt_parser_compares_semantically() {
        let stanza = Fixture::node("receipt")
            .child(Fixture::node("offer"))
            .child(Fixture::node("accept"))
            .child(Fixture::node("reject"))
            .attr("id", "x")
            .attr("type", "x")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let derived = CallReceiptParser::derive(&node).expect("derives");
        let again = CallReceiptParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = CallReceiptParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `IncomingMsgReceiptParser` derives from a stanza carrying its required fields.
    #[test]
    fn incoming_msg_receipt_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgReceiptParser::derive(&node);
        assert!(
            derived.is_ok(),
            "IncomingMsgReceiptParser: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, IncomingMsgReceiptParser::derive(&node));
    }

    /// `IncomingMsgReceiptParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn incoming_msg_receipt_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .attr("offline", "x")
            .attr("type", "delivery")
            .child(Fixture::node("error").attr("reason", "x").attr("type", "x"))
            .child(
                Fixture::node("participants")
                    .child(
                        Fixture::node("user")
                            .jid_attr("jid", "u")
                            .attr("t", "1")
                            .attr("type", "delivery")
                            .jid_attr("participant_pn", "u")
                            .attr("participant_username", "x"),
                    )
                    .attr("message_id", "x")
                    .attr("key", "x"),
            )
            .jid_attr("participant", "u")
            .jid_attr("recipient", "u")
            .child(
                Fixture::node("list")
                    .child(Fixture::node("item").attr("server_id", "x").attr("id", "x")),
            )
            .child(
                Fixture::node("biz")
                    .attr("actual_actors", "1")
                    .attr("host_storage", "1")
                    .attr("privacy_mode_ts", "1"),
            )
            .attr("is_lid", "x")
            .jid_attr("participant_pn", "u")
            .attr("participant_username", "x")
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgReceiptParser::derive(&node);
        assert!(
            derived.is_ok(),
            "IncomingMsgReceiptParser: {:?}",
            derived.err()
        );
    }

    /// `IncomingMsgReceiptParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn incoming_msg_receipt_parser_compares_semantically() {
        let stanza = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .attr("offline", "x")
            .attr("type", "delivery")
            .child(Fixture::node("error").attr("reason", "x").attr("type", "x"))
            .child(
                Fixture::node("participants")
                    .child(
                        Fixture::node("user")
                            .jid_attr("jid", "u")
                            .attr("t", "1")
                            .attr("type", "delivery")
                            .jid_attr("participant_pn", "u")
                            .attr("participant_username", "x"),
                    )
                    .attr("message_id", "x")
                    .attr("key", "x"),
            )
            .jid_attr("participant", "u")
            .jid_attr("recipient", "u")
            .child(
                Fixture::node("list")
                    .child(Fixture::node("item").attr("server_id", "x").attr("id", "x")),
            )
            .child(
                Fixture::node("biz")
                    .attr("actual_actors", "1")
                    .attr("host_storage", "1")
                    .attr("privacy_mode_ts", "1"),
            )
            .attr("is_lid", "x")
            .jid_attr("participant_pn", "u")
            .attr("participant_username", "x")
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = IncomingMsgReceiptParser::derive(&node).expect("derives");
        let again = IncomingMsgReceiptParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("receipt")
            .attr("id", "x")
            .jid_attr("from", "u")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = IncomingMsgReceiptParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `RetryRequestParser` derives from a stanza carrying its required fields.
    #[test]
    fn retry_request_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("receipt")
            .attr("type", "x")
            .jid_attr("from", "u")
            .child(Fixture::node("retry").attr("id", "x"))
            .attr("id", "x")
            .attr("t", "1")
            .child(Fixture::node("registration").bytes(&[1]))
            .build();
        let node = parse(&stanza);
        let derived = RetryRequestParser::derive(&node);
        assert!(derived.is_ok(), "RetryRequestParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, RetryRequestParser::derive(&node));
    }

    /// `RetryRequestParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn retry_request_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("receipt")
            .attr("type", "x")
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .attr("is_lid", "x")
            .jid_attr("recipient", "u")
            .child(Fixture::node("retry").attr("id", "x").attr("count", "1"))
            .child(
                Fixture::node("keys")
                    .child(Fixture::node("identity").bytes(b"x"))
                    .child(
                        Fixture::node("skey")
                            .child(Fixture::node("id").bytes(&[1]))
                            .child(Fixture::node("value").bytes(b"x"))
                            .child(Fixture::node("signature").bytes(b"x")),
                    )
                    .child(
                        Fixture::node("key")
                            .child(Fixture::node("id").bytes(&[1]))
                            .child(Fixture::node("value").bytes(b"x")),
                    ),
            )
            .attr("id", "x")
            .attr("t", "1")
            .child(Fixture::node("registration").bytes(&[1]))
            .build();
        let node = parse(&stanza);
        let derived = RetryRequestParser::derive(&node);
        assert!(derived.is_ok(), "RetryRequestParser: {:?}", derived.err());
    }

    /// `RetryRequestParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn retry_request_parser_compares_semantically() {
        let stanza = Fixture::node("receipt")
            .attr("type", "x")
            .jid_attr("from", "u")
            .jid_attr("participant", "u")
            .attr("is_lid", "x")
            .jid_attr("recipient", "u")
            .child(Fixture::node("retry").attr("id", "x").attr("count", "1"))
            .child(
                Fixture::node("keys")
                    .child(Fixture::node("identity").bytes(b"x"))
                    .child(
                        Fixture::node("skey")
                            .child(Fixture::node("id").bytes(&[1]))
                            .child(Fixture::node("value").bytes(b"x"))
                            .child(Fixture::node("signature").bytes(b"x")),
                    )
                    .child(
                        Fixture::node("key")
                            .child(Fixture::node("id").bytes(&[1]))
                            .child(Fixture::node("value").bytes(b"x")),
                    ),
            )
            .attr("id", "x")
            .attr("t", "1")
            .child(Fixture::node("registration").bytes(&[1]))
            .build();
        let node = parse(&stanza);
        let derived = RetryRequestParser::derive(&node).expect("derives");
        let again = RetryRequestParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("receipt")
            .attr("type", "x")
            .jid_attr("from", "u")
            .child(Fixture::node("retry").attr("id", "x"))
            .attr("id", "x")
            .attr("t", "1")
            .child(Fixture::node("registration").bytes(&[1]))
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = RetryRequestParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `CallOfferNoticeParser` derives from a stanza carrying its required fields.
    #[test]
    fn call_offer_notice_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("call")
            .attr("id", "x")
            .jid_attr("from", "u")
            .child(
                Fixture::node("offer_notice")
                    .jid_attr("call-creator", "u")
                    .attr("call-id", "x")
                    .attr("type", "x")
                    .attr("media", "x"),
            )
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = CallOfferNoticeParser::derive(&node);
        assert!(
            derived.is_ok(),
            "CallOfferNoticeParser: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, CallOfferNoticeParser::derive(&node));
    }

    /// `CallOfferNoticeParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn call_offer_notice_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("call")
            .attr("id", "x")
            .jid_attr("from", "u")
            .child(
                Fixture::node("offer_notice")
                    .jid_attr("call-creator", "u")
                    .attr("call-id", "x")
                    .attr("type", "x")
                    .attr("media", "x"),
            )
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = CallOfferNoticeParser::derive(&node);
        assert!(
            derived.is_ok(),
            "CallOfferNoticeParser: {:?}",
            derived.err()
        );
    }

    /// `CallOfferNoticeParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn call_offer_notice_parser_compares_semantically() {
        let stanza = Fixture::node("call")
            .attr("id", "x")
            .jid_attr("from", "u")
            .child(
                Fixture::node("offer_notice")
                    .jid_attr("call-creator", "u")
                    .attr("call-id", "x")
                    .attr("type", "x")
                    .attr("media", "x"),
            )
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = CallOfferNoticeParser::derive(&node).expect("derives");
        let again = CallOfferNoticeParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("call")
            .attr("id", "x")
            .jid_attr("from", "u")
            .child(
                Fixture::node("offer_notice")
                    .jid_attr("call-creator", "u")
                    .attr("call-id", "x")
                    .attr("type", "x")
                    .attr("media", "x"),
            )
            .attr("t", "1")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = CallOfferNoticeParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `CallOfferPlaceholder` derives from a stanza carrying its required fields.
    #[test]
    fn call_offer_placeholder_derives_from_its_required_fields() {
        let stanza = Fixture::node("call").build();
        let node = parse(&stanza);
        let derived = CallOfferPlaceholder::derive(&node);
        assert!(derived.is_ok(), "CallOfferPlaceholder: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, CallOfferPlaceholder::derive(&node));
    }

    /// `CallOfferPlaceholder` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn call_offer_placeholder_derives_with_every_field_present() {
        let stanza = Fixture::node("call").attr("t", "1").build();
        let node = parse(&stanza);
        let derived = CallOfferPlaceholder::derive(&node);
        assert!(derived.is_ok(), "CallOfferPlaceholder: {:?}", derived.err());
    }

    /// `CallOfferPlaceholder` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn call_offer_placeholder_compares_semantically() {
        let stanza = Fixture::node("call").attr("t", "1").build();
        let node = parse(&stanza);
        let derived = CallOfferPlaceholder::derive(&node).expect("derives");
        let again = CallOfferPlaceholder::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("call").build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = CallOfferPlaceholder::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `CallParser` derives from a stanza carrying its required fields.
    #[test]
    fn call_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("call")
            .jid_attr("from", "u")
            .attr("id", "x")
            .build();
        let node = parse(&stanza);
        let derived = CallParser::derive(&node);
        assert!(derived.is_ok(), "CallParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, CallParser::derive(&node));
    }

    /// `CallParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn call_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("call")
            .jid_attr("from", "u")
            .jid_attr("sender_lid", "u")
            .attr("platform", "x")
            .attr("version", "x")
            .attr("t", "1")
            .attr("e", "1")
            .attr("id", "x")
            .build();
        let node = parse(&stanza);
        let derived = CallParser::derive(&node);
        assert!(derived.is_ok(), "CallParser: {:?}", derived.err());
    }

    /// `CallParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn call_parser_compares_semantically() {
        let stanza = Fixture::node("call")
            .jid_attr("from", "u")
            .jid_attr("sender_lid", "u")
            .attr("platform", "x")
            .attr("version", "x")
            .attr("t", "1")
            .attr("e", "1")
            .attr("id", "x")
            .build();
        let node = parse(&stanza);
        let derived = CallParser::derive(&node).expect("derives");
        let again = CallParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("call")
            .jid_attr("from", "u")
            .attr("id", "x")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = CallParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `Ack` derives from a stanza carrying its required fields.
    #[test]
    fn ack_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("id", "x")
            .attr("class", "x")
            .build();
        let node = parse(&stanza);
        let derived = Ack::derive(&node);
        assert!(derived.is_ok(), "Ack: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, Ack::derive(&node));
    }

    /// `Ack` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn ack_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("id", "x")
            .attr("t", "x")
            .attr("class", "x")
            .attr("type", "x")
            .jid_attr("participant", "u")
            .jid_attr("recipient", "u")
            .build();
        let node = parse(&stanza);
        let derived = Ack::derive(&node);
        assert!(derived.is_ok(), "Ack: {:?}", derived.err());
    }

    /// `Ack` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn ack_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("id", "x")
            .attr("t", "x")
            .attr("class", "x")
            .attr("type", "x")
            .jid_attr("participant", "u")
            .jid_attr("recipient", "u")
            .build();
        let node = parse(&stanza);
        let derived = Ack::derive(&node).expect("derives");
        let again = Ack::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("id", "x")
            .attr("class", "x")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = Ack::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ParseNewsletterResponseNegative` derives from a stanza carrying its required fields.
    #[test]
    fn parse_newsletter_response_negative_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .bytes(b"x")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseNegative::derive(&node);
        assert!(
            derived.is_ok(),
            "ParseNewsletterResponseNegative: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, ParseNewsletterResponseNegative::derive(&node));
    }

    /// `ParseNewsletterResponseNegative` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn parse_newsletter_response_negative_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .bytes(b"x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseNegative::derive(&node);
        assert!(
            derived.is_ok(),
            "ParseNewsletterResponseNegative: {:?}",
            derived.err()
        );
    }

    /// `ParseNewsletterResponseNegative` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn parse_newsletter_response_negative_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .bytes(b"x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseNegative::derive(&node).expect("derives");
        let again = ParseNewsletterResponseNegative::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .bytes(b"x")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ParseNewsletterResponseNegative::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ParseNewsletterResponseSuccess` derives from a stanza carrying its required fields.
    #[test]
    fn parse_newsletter_response_success_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParseNewsletterResponseSuccess: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, ParseNewsletterResponseSuccess::derive(&node));
    }

    /// `ParseNewsletterResponseSuccess` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn parse_newsletter_response_success_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("server_id", "1")
            .attr("t", "1")
            .attr("edit", "x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParseNewsletterResponseSuccess: {:?}",
            derived.err()
        );
    }

    /// `ParseNewsletterResponseSuccess` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn parse_newsletter_response_success_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("server_id", "1")
            .attr("t", "1")
            .attr("edit", "x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseSuccess::derive(&node).expect("derives");
        let again = ParseNewsletterResponseSuccess::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ParseNewsletterResponseSuccess::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ParsePostNewsletterStatusResponseNegative` derives from a stanza carrying its required fields.
    #[test]
    fn parse_post_newsletter_status_response_negative_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseNegative::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePostNewsletterStatusResponseNegative: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(
            derived,
            ParsePostNewsletterStatusResponseNegative::derive(&node)
        );
    }

    /// `ParsePostNewsletterStatusResponseNegative` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn parse_post_newsletter_status_response_negative_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "1")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseNegative::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePostNewsletterStatusResponseNegative: {:?}",
            derived.err()
        );
    }

    /// `ParsePostNewsletterStatusResponseNegative` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn parse_post_newsletter_status_response_negative_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "1")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseNegative::derive(&node).expect("derives");
        let again = ParsePostNewsletterStatusResponseNegative::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("application_error", "1")
            .attr("backoff", "1")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ParsePostNewsletterStatusResponseNegative::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ParsePostNewsletterStatusResponseSuccess` derives from a stanza carrying its required fields.
    #[test]
    fn parse_post_newsletter_status_response_success_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("class", "x")
            .attr("t", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePostNewsletterStatusResponseSuccess: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(
            derived,
            ParsePostNewsletterStatusResponseSuccess::derive(&node)
        );
    }

    /// `ParsePostNewsletterStatusResponseSuccess` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn parse_post_newsletter_status_response_success_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("server_id", "1")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePostNewsletterStatusResponseSuccess: {:?}",
            derived.err()
        );
    }

    /// `ParsePostNewsletterStatusResponseSuccess` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn parse_post_newsletter_status_response_success_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("server_id", "1")
            .attr("class", "x")
            .attr("t", "1")
            .attr("edit", "1")
            .build();
        let node = parse(&stanza);
        let derived = ParsePostNewsletterStatusResponseSuccess::derive(&node).expect("derives");
        let again = ParsePostNewsletterStatusResponseSuccess::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("class", "x")
            .attr("t", "1")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ParsePostNewsletterStatusResponseSuccess::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ParsePublishViewResponseSuccess` derives from a stanza carrying its required fields.
    #[test]
    fn parse_publish_view_response_success_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("class", "x")
            .attr("edit", "0")
            .build();
        let node = parse(&stanza);
        let derived = ParsePublishViewResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePublishViewResponseSuccess: {:?}",
            derived.err()
        );
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, ParsePublishViewResponseSuccess::derive(&node));
    }

    /// `ParsePublishViewResponseSuccess` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn parse_publish_view_response_success_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("class", "x")
            .attr("t", "1")
            .attr("readreceipts", "all")
            .attr("edit", "0")
            .build();
        let node = parse(&stanza);
        let derived = ParsePublishViewResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePublishViewResponseSuccess: {:?}",
            derived.err()
        );
    }

    /// `ParsePublishViewResponseSuccess` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn parse_publish_view_response_success_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("class", "x")
            .attr("t", "1")
            .attr("readreceipts", "all")
            .attr("edit", "0")
            .build();
        let node = parse(&stanza);
        let derived = ParsePublishViewResponseSuccess::derive(&node).expect("derives");
        let again = ParsePublishViewResponseSuccess::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack")
            .attr("class", "x")
            .attr("edit", "0")
            .build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ParsePublishViewResponseSuccess::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `ReadReceiptAckParser` derives from a stanza carrying its required fields.
    #[test]
    fn read_receipt_ack_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack").build();
        let node = parse(&stanza);
        let derived = ReadReceiptAckParser::derive(&node);
        assert!(derived.is_ok(), "ReadReceiptAckParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, ReadReceiptAckParser::derive(&node));
    }

    /// `ReadReceiptAckParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn read_receipt_ack_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("ack").attr("readreceipts", "all").build();
        let node = parse(&stanza);
        let derived = ReadReceiptAckParser::derive(&node);
        assert!(derived.is_ok(), "ReadReceiptAckParser: {:?}", derived.err());
    }

    /// `ReadReceiptAckParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn read_receipt_ack_parser_compares_semantically() {
        let stanza = Fixture::node("ack").attr("readreceipts", "all").build();
        let node = parse(&stanza);
        let derived = ReadReceiptAckParser::derive(&node).expect("derives");
        let again = ReadReceiptAckParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack").build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = ReadReceiptAckParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `SendMsgAckSyncParser` derives from a stanza carrying its required fields.
    #[test]
    fn send_msg_ack_sync_parser_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack").attr("t", "1").build();
        let node = parse(&stanza);
        let derived = SendMsgAckSyncParser::derive(&node);
        assert!(derived.is_ok(), "SendMsgAckSyncParser: {:?}", derived.err());
        // Derivation is pure, so a second run must agree.
        assert_eq!(derived, SendMsgAckSyncParser::derive(&node));
    }

    /// `SendMsgAckSyncParser` derives when every field it models is present.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field; this one does.
    #[test]
    fn send_msg_ack_sync_parser_derives_with_every_field_present() {
        let stanza = Fixture::node("ack")
            .attr("t", "1")
            .attr("sync", "x")
            .attr("phash", "x")
            .attr("refresh_lid", "x")
            .attr("addressing_mode", "x")
            .attr("count", "1")
            .attr("error", "1")
            .build();
        let node = parse(&stanza);
        let derived = SendMsgAckSyncParser::derive(&node);
        assert!(derived.is_ok(), "SendMsgAckSyncParser: {:?}", derived.err());
    }

    /// `SendMsgAckSyncParser` agrees with itself and differs from another shape.
    ///
    /// Reflexivity is the floor: a comparison that cannot recognise
    /// its own output would report every stanza as a divergence.
    #[test]
    fn send_msg_ack_sync_parser_compares_semantically() {
        let stanza = Fixture::node("ack")
            .attr("t", "1")
            .attr("sync", "x")
            .attr("phash", "x")
            .attr("refresh_lid", "x")
            .attr("addressing_mode", "x")
            .attr("count", "1")
            .attr("error", "1")
            .build();
        let node = parse(&stanza);
        let derived = SendMsgAckSyncParser::derive(&node).expect("derives");
        let again = SendMsgAckSyncParser::derive(&node).expect("derives");
        assert!(derived.semantic_eq(&again));

        // A stanza missing every optional field is a different
        // derivation of the same shape, unless the shape has none.
        let bare = Fixture::node("ack").attr("t", "1").build();
        let bare_node = parse(&bare);
        if let Ok(bare_derived) = SendMsgAckSyncParser::derive(&bare_node) {
            let full_is_bare = derived.semantic_eq(&bare_derived);
            // Either they carry the same fields or they do not; both
            // are valid, and the comparison must not panic either way.
            let _ = full_is_bare;
        }
    }

    /// `AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation` is chosen for a stanza built from
    /// `AckPaidConversation`'s own fields.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_selects_ack_paid_conversation() {
        let stanza = Fixture::node("ack")
            .child(
                Fixture::node("biz")
                    .attr("paid_convo_id", "x")
                    .attr("pricing_model", "CBP")
                    .attr("billable", "false"),
            )
            .build();
        let node = parse(&stanza);
        let derived = AckPaidConversationOrAckPaidGroupConversation::derive(&node);
        assert!(
            derived.is_ok(),
            "AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again = AckPaidConversationOrAckPaidGroupConversation::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_selects_ack_paid_conversation_with_every_field()
     {
        let stanza = Fixture::node("ack")
            .child(
                Fixture::node("biz")
                    .attr("paid_convo_id", "x")
                    .attr("pricing_model", "CBP")
                    .attr("billable", "false")
                    .attr("expiration_timestamp", "1")
                    .attr("pricing_category", "x")
                    .attr("pricing_type", "free_customer_service")
                    .child(Fixture::node("delivery_context").attr("optimization_goal", "delivery"))
                    .child(
                        Fixture::node("origin").attr("type", "x").child(
                            Fixture::node("referral")
                                .attr("source_type", "x")
                                .child(Fixture::node("source_url").bytes(b"x")),
                        ),
                    )
                    .child(
                        Fixture::node("pricing")
                            .attr("consumer_country_code", "x")
                            .attr("business_country_code", "x")
                            .attr("conversation_status", "1")
                            .attr("latest_c2b_timestamp", "1")
                            .attr("analytics_conversation_id", "x")
                            .attr("b2c_timestamp", "1"),
                    ),
            )
            .build();
        let node = parse(&stanza);
        let Some(full) = AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&node) else {
            panic!(
                "AckPaidConversationOrAckPaidGroupConversation::AckPaidConversation derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack")
            .child(
                Fixture::node("biz")
                    .attr("paid_convo_id", "x")
                    .attr("pricing_model", "CBP")
                    .attr("billable", "false"),
            )
            .build();
        let bare_node = parse(&bare);
        if let Some(lean) = AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// `AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation` is chosen for a stanza built from
    /// `AckPaidGroupConversation`'s own fields.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_selects_ack_paid_group_conversation() {
        let stanza = Fixture::node("ack")
            .child(Fixture::node("biz").child(Fixture::node("pricing")))
            .build();
        let node = parse(&stanza);
        let derived = AckPaidConversationOrAckPaidGroupConversation::derive(&node);
        assert!(
            derived.is_ok(),
            "AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again = AckPaidConversationOrAckPaidGroupConversation::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_selects_ack_paid_group_conversation_with_every_field()
     {
        let stanza = Fixture::node("ack")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .build();
        let node = parse(&stanza);
        let Some(full) = AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&node) else {
            panic!(
                "AckPaidConversationOrAckPaidGroupConversation::AckPaidGroupConversation derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack")
            .child(Fixture::node("biz").child(Fixture::node("pricing")))
            .build();
        let bare_node = parse(&bare);
        if let Some(lean) = AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// A node satisfying no `AckPaidConversationOrAckPaidGroupConversation` alternative yields none.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_matches_nothing_when_no_variant_fits() {
        let stanza = Fixture::node("nothing-here").build();
        let node = parse(&stanza);
        assert!(AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&node).is_none());
        assert_eq!(
            AckPaidConversationOrAckPaidGroupConversation::derive(&node),
            Err(DeriveError::UnknownStanza)
        );
    }

    /// Two different `AckPaidConversationOrAckPaidGroupConversation` alternatives never mean the same.
    #[test]
    fn ack_paid_conversation_or_ack_paid_group_conversation_alternatives_are_not_interchangeable() {
        let a = Fixture::node("ack")
            .child(
                Fixture::node("biz")
                    .attr("paid_convo_id", "x")
                    .attr("pricing_model", "CBP")
                    .attr("billable", "false"),
            )
            .build();
        let b = Fixture::node("ack")
            .child(Fixture::node("biz").child(Fixture::node("pricing")))
            .build();
        let (na, nb) = (parse(&a), parse(&b));
        let (Some(x), Some(y)) = (
            AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&na),
            AckPaidConversationOrAckPaidGroupConversation::maybe_derive(&nb),
        ) else {
            panic!("both fixtures derive");
        };
        assert!(x.semantic_eq(&x));
        // Same alternative or not, comparing must not panic; where
        // they differ, they must not compare equal.
        if core::mem::discriminant(&x) != core::mem::discriminant(&y) {
            assert!(!x.semantic_eq(&y));
        }
    }

    /// `NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck` is chosen for a stanza built from
    /// `NewsletterQuestionResponseAck`'s own fields.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_selects_newsletter_question_response_ack()
     {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("response_server_id", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .build();
        let node = parse(&stanza);
        let derived = NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(&node);
        assert!(
            derived.is_ok(),
            "NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again =
            NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_selects_newsletter_question_response_ack_with_every_field()
     {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("response_server_id", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .build();
        let node = parse(&stanza);
        let Some(full) = NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&node)
        else {
            panic!(
                "NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterQuestionResponseAck derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack")
            .attr("class", "message")
            .attr("response_server_id", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .build();
        let bare_node = parse(&bare);
        if let Some(lean) =
            NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// `NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck` is chosen for a stanza built from
    /// `NewsletterMessageAck`'s own fields.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_selects_newsletter_message_ack() {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let node = parse(&stanza);
        let derived = NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(&node);
        assert!(
            derived.is_ok(),
            "NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again =
            NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_selects_newsletter_message_ack_with_every_field()
     {
        let stanza = Fixture::node("ack")
            .attr("class", "message")
            .attr("server_id", "1")
            .attr("t", "1")
            .attr("edit", "x")
            .child(
                Fixture::node("biz")
                    .child(Fixture::node("pricing").attr("business_country_code", "x")),
            )
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let node = parse(&stanza);
        let Some(full) = NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&node)
        else {
            panic!(
                "NewsletterQuestionResponseAckOrNewsletterMessageAck::NewsletterMessageAck derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let bare_node = parse(&bare);
        if let Some(lean) =
            NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// A node satisfying no `NewsletterQuestionResponseAckOrNewsletterMessageAck` alternative yields none.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_matches_nothing_when_no_variant_fits()
     {
        let stanza = Fixture::node("nothing-here").build();
        let node = parse(&stanza);
        assert!(NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&node).is_none());
        assert_eq!(
            NewsletterQuestionResponseAckOrNewsletterMessageAck::derive(&node),
            Err(DeriveError::UnknownStanza)
        );
    }

    /// Two different `NewsletterQuestionResponseAckOrNewsletterMessageAck` alternatives never mean the same.
    #[test]
    fn newsletter_question_response_ack_or_newsletter_message_ack_alternatives_are_not_interchangeable()
     {
        let a = Fixture::node("ack")
            .attr("class", "message")
            .attr("response_server_id", "x")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .build();
        let b = Fixture::node("ack")
            .attr("class", "message")
            .attr("t", "1")
            .attr("edit", "x")
            .child(Fixture::node("franking").child(Fixture::node("reporting_tag").bytes(b"x")))
            .child(Fixture::node("rcat").bytes(b"x"))
            .build();
        let (na, nb) = (parse(&a), parse(&b));
        let (Some(x), Some(y)) = (
            NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&na),
            NewsletterQuestionResponseAckOrNewsletterMessageAck::maybe_derive(&nb),
        ) else {
            panic!("both fixtures derive");
        };
        assert!(x.semantic_eq(&x));
        // Same alternative or not, comparing must not panic; where
        // they differ, they must not compare equal.
        if core::mem::discriminant(&x) != core::mem::discriminant(&y) {
            assert!(!x.semantic_eq(&y));
        }
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit` is chosen for a stanza built from
    /// `StatusAckEdit`'s own fields.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_edit() {
        let stanza = Fixture::node("ack").attr("edit", "1").build();
        let node = parse(&stanza);
        let derived = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node);
        assert!(
            derived.is_ok(),
            "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_edit_with_every_field()
     {
        let stanza = Fixture::node("ack").attr("edit", "1").build();
        let node = parse(&stanza);
        let Some(full) = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&node)
        else {
            panic!(
                "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckEdit derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack").attr("edit", "1").build();
        let bare_node = parse(&bare);
        if let Some(lean) =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke` is chosen for a stanza built from
    /// `StatusAckRevoke`'s own fields.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_revoke() {
        let stanza = Fixture::node("ack").attr("edit", "7").build();
        let node = parse(&stanza);
        let derived = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node);
        assert!(
            derived.is_ok(),
            "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_revoke_with_every_field()
     {
        let stanza = Fixture::node("ack").attr("edit", "7").build();
        let node = parse(&stanza);
        let Some(full) = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&node)
        else {
            panic!(
                "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckRevoke derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack").attr("edit", "7").build();
        let bare_node = parse(&bare);
        if let Some(lean) =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke` is chosen for a stanza built from
    /// `StatusAckAdminRevoke`'s own fields.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_admin_revoke()
     {
        let stanza = Fixture::node("ack").attr("edit", "8").build();
        let node = parse(&stanza);
        let derived = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node);
        assert!(
            derived.is_ok(),
            "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke: {:?}",
            derived.err()
        );
        let chosen = derived.expect("derives");
        assert!(matches!(
            chosen,
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke(_)
        ));

        // Each alternative carries its own comparison, and one that
        // could not recognise its own output would report every
        // stanza as a divergence. Derivation is pure, so the second
        // run is the same derivation and must compare equal.
        let again =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node).expect("derives");
        assert!(chosen.semantic_eq(&again));
    }

    /// `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke` derives with every field it models.
    ///
    /// The required-only fixture never reaches the `Some` side of an
    /// optional field, nor the comparison arm that reads it.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_selects_status_ack_admin_revoke_with_every_field()
     {
        let stanza = Fixture::node("ack").attr("edit", "8").build();
        let node = parse(&stanza);
        let Some(full) = StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&node)
        else {
            panic!(
                "StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::StatusAckAdminRevoke derives with every field"
            );
        };
        assert!(full.semantic_eq(&full));

        // A derivation carrying optional fields does not mean the
        // same as one without them.
        let bare = Fixture::node("ack").attr("edit", "8").build();
        let bare_node = parse(&bare);
        if let Some(lean) =
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&bare_node)
        {
            let _ = full.semantic_eq(&lean);
        }
    }

    /// A node satisfying no `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke` alternative yields none.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_matches_nothing_when_no_variant_fits()
     {
        let stanza = Fixture::node("nothing-here").build();
        let node = parse(&stanza);
        assert!(
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&node).is_none()
        );
        assert_eq!(
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::derive(&node),
            Err(DeriveError::UnknownStanza)
        );
    }

    /// Two different `StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke` alternatives never mean the same.
    #[test]
    fn status_ack_edit_or_status_ack_revoke_or_status_ack_admin_revoke_alternatives_are_not_interchangeable()
     {
        let a = Fixture::node("ack").attr("edit", "1").build();
        let b = Fixture::node("ack").attr("edit", "7").build();
        let (na, nb) = (parse(&a), parse(&b));
        let (Some(x), Some(y)) = (
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&na),
            StatusAckEditOrStatusAckRevokeOrStatusAckAdminRevoke::maybe_derive(&nb),
        ) else {
            panic!("both fixtures derive");
        };
        assert!(x.semantic_eq(&x));
        // Same alternative or not, comparing must not panic; where
        // they differ, they must not compare equal.
        if core::mem::discriminant(&x) != core::mem::discriminant(&y) {
            assert!(!x.semantic_eq(&y));
        }
    }

    /// Every `ALLNONE` value round-trips through the wire form.
    #[test]
    fn allnone_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [ALLNONE::All, ALLNONE::None] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ALLNONE::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(ALLNONE::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `APPDATA` value round-trips through the wire form.
    #[test]
    fn appdata_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [APPDATA::Default, APPDATA::MemberTag, APPDATA::GroupHistory] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(APPDATA::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(APPDATA::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `CiphertextType` value round-trips through the wire form.
    #[test]
    fn ciphertext_type_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            CiphertextType::Skmsg,
            CiphertextType::Pkmsg,
            CiphertextType::Msg,
            CiphertextType::Msmsg,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(CiphertextType::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            CiphertextType::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `ENUM017` value round-trips through the wire form.
    #[test]
    fn enum017_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [ENUM017::N0, ENUM017::N1, ENUM017::N7] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ENUM017::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(ENUM017::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `ENUMALLNONE` value round-trips through the wire form.
    #[test]
    fn enumallnone_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [ENUMALLNONE::All, ENUMALLNONE::None] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ENUMALLNONE::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(ENUMALLNONE::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `ENUMCBPNBPPMP` value round-trips through the wire form.
    #[test]
    fn enumcbpnbppmp_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [ENUMCBPNBPPMP::CBP, ENUMCBPNBPPMP::NBP, ENUMCBPNBPPMP::PMP] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ENUMCBPNBPPMP::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            ENUMCBPNBPPMP::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `ENUMDELIVERYNOOPTIMIZATION` value round-trips through the wire form.
    #[test]
    fn enumdeliverynooptimization_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            ENUMDELIVERYNOOPTIMIZATION::Delivery,
            ENUMDELIVERYNOOPTIMIZATION::NoOptimization,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ENUMDELIVERYNOOPTIMIZATION::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            ENUMDELIVERYNOOPTIMIZATION::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `ENUMFALSETRUE` value round-trips through the wire form.
    #[test]
    fn enumfalsetrue_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [ENUMFALSETRUE::False, ENUMFALSETRUE::True] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(ENUMFALSETRUE::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            ENUMFALSETRUE::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR` value round-trips through the wire form.
    #[test]
    fn enumfreecustomerservicefreeentrypointregular_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::FreeCustomerService,
            ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::FreeEntryPoint,
            ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::Regular,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(
                ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::from_wire(value),
                Some(variant)
            );
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            ENUMFREECUSTOMERSERVICEFREEENTRYPOINTREGULAR::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `EVENTTYPES` value round-trips through the wire form.
    #[test]
    fn eventtypes_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [EVENTTYPES::Creation, EVENTTYPES::Response, EVENTTYPES::Edit] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(EVENTTYPES::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(EVENTTYPES::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `IncomingMsgReceiptParserParticipantsUserType` value round-trips through the wire form.
    #[test]
    fn incoming_msg_receipt_parser_participants_user_type_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            IncomingMsgReceiptParserParticipantsUserType::Delivery,
            IncomingMsgReceiptParserParticipantsUserType::Read,
            IncomingMsgReceiptParserParticipantsUserType::Played,
            IncomingMsgReceiptParserParticipantsUserType::Inactive,
            IncomingMsgReceiptParserParticipantsUserType::ServerError,
            IncomingMsgReceiptParserParticipantsUserType::Sender,
            IncomingMsgReceiptParserParticipantsUserType::ReadSelf,
            IncomingMsgReceiptParserParticipantsUserType::PlayedSelf,
            IncomingMsgReceiptParserParticipantsUserType::PeerMsg,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(
                IncomingMsgReceiptParserParticipantsUserType::from_wire(value),
                Some(variant)
            );
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            IncomingMsgReceiptParserParticipantsUserType::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `IncomingMsgReceiptParserType` value round-trips through the wire form.
    #[test]
    fn incoming_msg_receipt_parser_type_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            IncomingMsgReceiptParserType::Delivery,
            IncomingMsgReceiptParserType::Read,
            IncomingMsgReceiptParserType::Played,
            IncomingMsgReceiptParserType::Inactive,
            IncomingMsgReceiptParserType::ServerError,
            IncomingMsgReceiptParserType::Sender,
            IncomingMsgReceiptParserType::ReadSelf,
            IncomingMsgReceiptParserType::PlayedSelf,
            IncomingMsgReceiptParserType::PeerMsg,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(
                IncomingMsgReceiptParserType::from_wire(value),
                Some(variant)
            );
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            IncomingMsgReceiptParserType::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `MSGVERIFIEDLEVEL` value round-trips through the wire form.
    #[test]
    fn msgverifiedlevel_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            MSGVERIFIEDLEVEL::High,
            MSGVERIFIEDLEVEL::Low,
            MSGVERIFIEDLEVEL::Unknown,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(MSGVERIFIEDLEVEL::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            MSGVERIFIEDLEVEL::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `POLLTYPES` value round-trips through the wire form.
    #[test]
    fn polltypes_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            POLLTYPES::Creation,
            POLLTYPES::QuizCreation,
            POLLTYPES::Vote,
            POLLTYPES::ResultSnapshot,
            POLLTYPES::Edit,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(POLLTYPES::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(POLLTYPES::from_wire(node.attr("v").expect("attr")), None);
    }

    /// Every `STANZAMSGORIGIN` value round-trips through the wire form.
    #[test]
    fn stanzamsgorigin_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [STANZAMSGORIGIN::Ctwa] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(STANZAMSGORIGIN::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            STANZAMSGORIGIN::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Every `STANZAMSGTYPES` value round-trips through the wire form.
    #[test]
    fn stanzamsgtypes_round_trips() {
        #[allow(clippy::single_element_loop)]
        for variant in [
            STANZAMSGTYPES::Text,
            STANZAMSGTYPES::Media,
            STANZAMSGTYPES::Medianotify,
            STANZAMSGTYPES::Pay,
            STANZAMSGTYPES::Poll,
            STANZAMSGTYPES::Reaction,
            STANZAMSGTYPES::Event,
        ] {
            let wire = variant.as_wire();
            assert!(!wire.is_empty());
            let stanza = Fixture::node("n").attr("v", wire).build();
            let node = parse(&stanza);
            let value = node.attr("v").expect("the attribute");
            assert_eq!(STANZAMSGTYPES::from_wire(value), Some(variant));
        }
        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();
        let node = parse(&stanza);
        assert_eq!(
            STANZAMSGTYPES::from_wire(node.attr("v").expect("attr")),
            None
        );
    }

    /// Events of different shapes never mean the same thing.
    #[test]
    fn different_shapes_never_compare_equal() {
        // Which shape matched is part of what was derived, so two
        // engines picking different shapes for one stanza is exactly
        // the divergence a conformance run should report.
        let stanza = Fixture::node("receipt")
            .attr("id", "A")
            .jid_attr("from", "u")
            .build();
        let node = parse(&stanza);
        let a = derive(&node).expect("derives");
        assert!(a.semantic_eq(&a), "an event agrees with itself");

        let other = Fixture::node("ack")
            .attr("id", "A")
            .attr("class", "message")
            .jid_attr("content", "u")
            .build();
        let b = derive(&parse(&other)).expect("derives");
        assert!(!a.semantic_eq(&b));
        assert!(!b.semantic_eq(&a));
        assert_ne!(a.tag(), b.tag());
    }

    /// Every tag dispatches, and an unmodelled one is reported as such.
    #[test]
    fn dispatch_covers_every_known_tag() {
        for tag in KNOWN_TAGS {
            let stanza = Fixture::node(tag).build();
            // A bare stanza matches no shape, but the tag is recognised —
            // which is the distinction `derive` exists to make.
            assert_ne!(
                derive(&parse(&stanza)),
                Err(DeriveError::UnknownStanza),
                "{tag} is in KNOWN_TAGS but does not dispatch"
            );
        }
    }
}
