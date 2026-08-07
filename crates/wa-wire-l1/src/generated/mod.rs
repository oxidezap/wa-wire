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
    incoming_digest: "sha256:90adc75ead28f23a6bd801ced35f30f87f8aa72414433dcb7a2f379ec3666649",
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
    /// `content`, via `attrString`.
    pub content: Value<'a>,
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
            content: extract::attr_string(node, "content")?,
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
    /// `content`, via `attrJidWithType`.
    pub content: Jid<'a>,
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
            content: extract::attr_jid(node, "content")?,
            participant: extract::maybe_attr_jid(node, "participant")?,
            recipient: extract::maybe_attr_jid(node, "recipient")?,
            node: *node,
        })
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
    /// `edit`, via `attrString`.
    pub edit: Value<'a>,
    /// `frankingReportingTagElementValue`, via `contentBytes`.
    pub franking_reporting_tag_element_value: &'a [u8],
    /// `applicationError`, via `attrInt`.
    pub application_error: i64,
    /// `backoff`, via `attrInt`.
    pub backoff: i64,
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParseNewsletterResponseNegative<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            error: extract::attr_string(node, "error")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            edit: extract::attr_string(node, "edit")?,
            franking_reporting_tag_element_value: extract::content_bytes(node)?,
            application_error: extract::attr_int(node, "applicationError")?,
            backoff: extract::attr_int(node, "backoff")?,
            node: *node,
        })
    }
}
/// Derived from whatspec's `ParseNewsletterResponseSuccess` shape.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseNewsletterResponseSuccess<'a> {
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParseNewsletterResponseSuccess<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self { node: *node })
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
    /// `applicationError`, via `attrInt`.
    pub application_error: i64,
    /// `backoff`, via `attrInt`.
    pub backoff: i64,
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
            application_error: extract::attr_int(node, "applicationError")?,
            backoff: extract::attr_int(node, "backoff")?,
            node: *node,
        })
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
    /// The node this was derived from, for fields the shape does
    /// not model yet.
    pub node: NodeRef<'a>,
}

impl<'a> ParsePostNewsletterStatusResponseSuccess<'a> {
    /// Derive from a node already known to match this shape.
    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {
        Ok(Self {
            server_id: extract::maybe_attr_int(node, "serverId")?,
            class: extract::attr_string(node, "class")?,
            t: extract::attr_int(node, "t")?,
            node: *node,
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
            node: *node,
        })
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
        if let Ok(inner) = CallParser::derive(node) {
            return Ok(Event::CallParser(inner));
        }
        if let Ok(inner) = CallOfferNoticeParser::derive(node) {
            return Ok(Event::CallOfferNoticeParser(inner));
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
        if let Ok(inner) = IncomingMsgReceiptParser::derive(node) {
            return Ok(Event::IncomingMsgReceiptParser(inner));
        }
        if let Ok(inner) = RetryRequestParser::derive(node) {
            return Ok(Event::RetryRequestParser(inner));
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

/// Fields the generator could not express, named rather than dropped in
/// silence.
///
/// A derivation that quietly omitted a field would look complete and be
/// wrong, and no conformance run could tell — every engine would agree on
/// the same missing field.
pub const UNMODELLED_FIELDS: [&str; 15] = [
    "IncomingMsgParser.verified_name: maybeAttrInt conflicts with kept maybeChild",
    "ParseNewsletterResponseNegative.ackPaidAckPaidConversationOrAckPaidGroupConversationConversationMixinGroup: mixin",
    "ParseNewsletterResponseNegative: reference assertion on `from` needs request context",
    "ParseNewsletterResponseNegative: reference assertion on `id` needs request context",
    "ParseNewsletterResponseSuccess.newsletterQuestionResponseOrNewsletterMessageAckMixinGroup: mixin",
    "ParsePostNewsletterStatusResponseNegative.statusAckEditOrRevokeOrAdminRevokeMixinGroup: mixin",
    "ParsePostNewsletterStatusResponseNegative: reference assertion on `from` needs request context",
    "ParsePostNewsletterStatusResponseNegative: reference assertion on `id` needs request context",
    "ParsePostNewsletterStatusResponseSuccess.statusAckEditOrRevokeOrAdminRevokeMixinGroup: mixin",
    "ParsePostNewsletterStatusResponseSuccess: reference assertion on `from` needs request context",
    "ParsePostNewsletterStatusResponseSuccess: reference assertion on `id` needs request context",
    "ParsePublishViewResponseSuccess.edit: attrEnum",
    "ParsePublishViewResponseSuccess: reference assertion on `from` needs request context",
    "ParsePublishViewResponseSuccess: reference assertion on `id` needs request context",
    "ParsePublishViewResponseSuccess: reference assertion on `type` needs request context",
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
                    .attr("content", "x")
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

    /// `Ack` derives from a stanza carrying its required fields.
    #[test]
    fn ack_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("id", "x")
            .attr("class", "x")
            .jid_attr("content", "u")
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
            .jid_attr("content", "u")
            .jid_attr("participant", "u")
            .jid_attr("recipient", "u")
            .build();
        let node = parse(&stanza);
        let derived = Ack::derive(&node);
        assert!(derived.is_ok(), "Ack: {:?}", derived.err());
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
            .attr("applicationError", "1")
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
            .attr("applicationError", "1")
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

    /// `ParseNewsletterResponseSuccess` derives from a stanza carrying its required fields.
    #[test]
    fn parse_newsletter_response_success_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack").build();
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
        let stanza = Fixture::node("ack").build();
        let node = parse(&stanza);
        let derived = ParseNewsletterResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParseNewsletterResponseSuccess: {:?}",
            derived.err()
        );
    }

    /// `ParsePostNewsletterStatusResponseNegative` derives from a stanza carrying its required fields.
    #[test]
    fn parse_post_newsletter_status_response_negative_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack")
            .attr("error", "x")
            .attr("class", "x")
            .attr("t", "1")
            .attr("applicationError", "1")
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
            .attr("applicationError", "1")
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
            .attr("serverId", "1")
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
    }

    /// `ParsePublishViewResponseSuccess` derives from a stanza carrying its required fields.
    #[test]
    fn parse_publish_view_response_success_derives_from_its_required_fields() {
        let stanza = Fixture::node("ack").attr("class", "x").build();
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
            .build();
        let node = parse(&stanza);
        let derived = ParsePublishViewResponseSuccess::derive(&node);
        assert!(
            derived.is_ok(),
            "ParsePublishViewResponseSuccess: {:?}",
            derived.err()
        );
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
