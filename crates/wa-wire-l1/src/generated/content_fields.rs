//! Field numbers in `waE2E.Message`, from whatspec's `WAProto.proto`.
//!
//! GENERATED FILE — do not edit by hand. Run `tools/generate-content.py`.
//!
//! Committed rather than produced by a build script so that a protocol
//! change arrives as a reviewable diff, per RFC-009. CI regenerates and
//! requires no change, which rules out drift.
//!
//! The rules that read these live in [`crate::content`] and are written by
//! hand: which variant is worth naming, what unwrapping means, and which
//! field of a variant carries its text are not in the schema.

/// SHA-256 of the proto this module was generated from.
pub const PROTO_DIGEST: &str =
    "sha256:021d53059e7b35d8553c97c09567da8d6aba7278c7f2510242eab686bff3647a";

/// `Message.conversation`.
pub const CONVERSATION: u32 = 1;
/// `Message.imageMessage`.
pub const IMAGE: u32 = 3;
/// `Message.contactMessage`.
pub const CONTACT: u32 = 4;
/// `Message.locationMessage`.
pub const LOCATION: u32 = 5;
/// `Message.extendedTextMessage`.
pub const EXTENDED_TEXT: u32 = 6;
/// `Message.documentMessage`.
pub const DOCUMENT: u32 = 7;
/// `Message.audioMessage`.
pub const AUDIO: u32 = 8;
/// `Message.videoMessage`.
pub const VIDEO: u32 = 9;
/// `Message.call`.
pub const CALL: u32 = 10;
/// `Message.protocolMessage`.
pub const PROTOCOL: u32 = 12;
/// `Message.stickerMessage`.
pub const STICKER: u32 = 26;
/// `Message.reactionMessage`.
pub const REACTION: u32 = 46;

/// Every field of `Message` whose type holds another `Message`.
///
/// Collected by type rather than by name, so a wrapper added upstream
/// arrives here without anyone remembering to list it.
pub const WRAPPERS: [u32; 29] = [
    31,  // deviceSentMessage: DeviceSentMessage
    37,  // viewOnceMessage: FutureProofMessage
    40,  // ephemeralMessage: FutureProofMessage
    53,  // documentWithCaptionMessage: FutureProofMessage
    55,  // viewOnceMessageV2: FutureProofMessage
    58,  // editedMessage: FutureProofMessage
    59,  // viewOnceMessageV2Extension: FutureProofMessage
    62,  // groupMentionedMessage: FutureProofMessage
    67,  // botInvokeMessage: FutureProofMessage
    74,  // lottieStickerMessage: FutureProofMessage
    85,  // eventCoverImage: FutureProofMessage
    87,  // statusMentionMessage: FutureProofMessage
    90,  // pollCreationOptionImageMessage: FutureProofMessage
    91,  // associatedChildMessage: FutureProofMessage
    92,  // groupStatusMentionMessage: FutureProofMessage
    93,  // pollCreationMessageV4: FutureProofMessage
    95,  // statusAddYours: FutureProofMessage
    96,  // groupStatusMessage: FutureProofMessage
    99,  // limitSharingMessage: FutureProofMessage
    100, // botTaskMessage: FutureProofMessage
    101, // questionMessage: FutureProofMessage
    103, // groupStatusMessageV2: FutureProofMessage
    104, // botForwardedMessage: FutureProofMessage
    106, // questionReplyMessage: FutureProofMessage
    116, // newsletterAdminProfileMessage: FutureProofMessage
    117, // newsletterAdminProfileMessageV2: FutureProofMessage
    118, // spoilerMessage: FutureProofMessage
    126, // newsletterAdminProfileStatusMessage: FutureProofMessage
    131, // botPlatformRegistrationSuccessMessage: FutureProofMessage
];

/// The first `FutureProofMessage` wrapper, for a test that needs one.
pub const VIEW_ONCE: u32 = 37;

/// `Message.deviceSentMessage`, the one wrapper that is not a
/// `FutureProofMessage` and keeps its message elsewhere.
pub const DEVICE_SENT: u32 = 31;

/// `DeviceSentMessage.message`.
pub const DEVICE_SENT_INNER: u32 = 2;
/// `FutureProofMessage.message`.
pub const FUTURE_PROOF_INNER: u32 = 1;

/// Where each variant that speaks keeps its text.
/// `ExtendedTextMessage.text`.
pub const EXTENDED_TEXT_TEXT: u32 = 1;
/// `ReactionMessage.text`.
pub const REACTION_TEXT: u32 = 2;
/// `ImageMessage.caption`.
pub const IMAGE_TEXT: u32 = 3;
/// `VideoMessage.caption`.
pub const VIDEO_TEXT: u32 = 7;
/// `DocumentMessage.caption`.
pub const DOCUMENT_TEXT: u32 = 20;
