//! Write the conformance corpus: one frame per file, the input both engines read.
//!
//! The corpus is frames rather than envelopes on purpose. An envelope is what
//! an adapter *produced*; a frame is what an engine *received*, and feeding the
//! same frame to two engines is the only way their outputs are comparable at
//! all.
//!
//! Frames are marshalled here because `whatsapp-rust` has the encoder, not
//! because they belong to it: `zapo` decodes each one with its own decoder and
//! re-encodes with its own encoder, which is exactly the divergence the
//! conformance test exists to catch.
//!
//! ```sh
//! cargo run --example emit-corpus
//! ```
//!
//! Committed output, regenerated in CI with no diff allowed — so a change to
//! the corpus is a reviewed change, not a surprise in someone's working tree.

use std::path::PathBuf;

use whatsapp_rust::wacore_binary::builder::NodeBuilder;
use whatsapp_rust::wacore_binary::marshal;
use whatsapp_rust::wacore_binary::node::Node;

/// The stanzas the corpus covers.
///
/// Chosen for what the L1 derivation distinguishes rather than for variety: the
/// four tags it models, the attribute shapes that select between one reading
/// and another, and the encodings where two engines could legitimately differ
/// (short vs long integers, packed JIDs, nested children).
#[allow(clippy::too_many_lines)] // A data table; splitting it would only hide it.
fn corpus() -> Vec<(&'static str, Node)> {
    vec![
        (
            "01-receipt-read",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-READ-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "read")
                .attr("t", "1700000000")
                .build(),
        ),
        (
            // No `type`, which is the delivery reading. The absence is what
            // selects the shape, so an engine that invents a default diverges.
            "02-receipt-delivery",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-DELIV-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("t", "1700000001")
                .build(),
        ),
        (
            "03-receipt-group-participant",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-GRP-1")
                .attr("from", "120363000000000000@g.us")
                .attr("participant", "5511999998888@s.whatsapp.net")
                .attr("type", "read")
                .attr("t", "1700000002")
                .build(),
        ),
        (
            // Repeated children, which derive lazily and must keep their order.
            "04-receipt-with-list",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-LIST-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "read")
                .children([NodeBuilder::new("list")
                    .children([
                        NodeBuilder::new("item").attr("id", "A").build(),
                        NodeBuilder::new("item").attr("id", "B").build(),
                        NodeBuilder::new("item").attr("id", "C").build(),
                    ])
                    .build()])
                .build(),
        ),
        (
            "05-message-direct",
            NodeBuilder::new("message")
                .attr("id", "MSG-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                // `recipient` and a typed `t` are what whatspec marks required,
                // so a message without them falls outside the shape entirely.
                .attr("recipient", "5511777776666@s.whatsapp.net")
                .attr("type", "text")
                .attr("t", "1700000010")
                .children([NodeBuilder::new("enc")
                    .attr("v", "2")
                    .attr("type", "msg")
                    .bytes(b"ciphertext-one".to_vec())
                    .build()])
                .build(),
        ),
        (
            "06-message-group-multi-enc",
            NodeBuilder::new("message")
                .attr("id", "MSG-GRP-1")
                .attr("from", "120363000000000000@g.us")
                .attr("participant", "5511999998888@s.whatsapp.net")
                .attr("recipient", "5511777776666@s.whatsapp.net")
                .attr("type", "text")
                .attr("t", "1700000011")
                .children([
                    NodeBuilder::new("enc")
                        .attr("v", "2")
                        .attr("type", "pkmsg")
                        .bytes(b"sender-key".to_vec())
                        .build(),
                    NodeBuilder::new("enc")
                        .attr("v", "2")
                        .attr("type", "skmsg")
                        .bytes(b"group-ciphertext".to_vec())
                        .build(),
                ])
                .build(),
        ),
        (
            // A device-identity child ahead of the enc, so a path that counted
            // `<enc>` nodes instead of children lands on the wrong node.
            "07-message-with-device-identity",
            NodeBuilder::new("message")
                .attr("id", "MSG-DI-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("recipient", "5511777776666@s.whatsapp.net")
                .attr("type", "text")
                .attr("t", "1700000012")
                .children([
                    NodeBuilder::new("device-identity")
                        .bytes(b"identity-blob".to_vec())
                        .build(),
                    NodeBuilder::new("enc")
                        .attr("v", "2")
                        .attr("type", "msg")
                        .bytes(b"ciphertext-two".to_vec())
                        .build(),
                ])
                .build(),
        ),
        (
            "08-ack-message",
            NodeBuilder::new("ack")
                .attr("id", "ACK-1")
                .attr("class", "message")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("t", "1700000020")
                .build(),
        ),
        (
            "09-ack-receipt",
            NodeBuilder::new("ack")
                .attr("id", "ACK-2")
                .attr("class", "receipt")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("t", "1700000021")
                .build(),
        ),
        (
            "10-call-offer",
            NodeBuilder::new("call")
                .attr("id", "CALL-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("t", "1700000030")
                .children([NodeBuilder::new("offer-notice")
                    .attr("call-id", "CALLID-1")
                    .attr("call-creator", "5511999998888@s.whatsapp.net")
                    .attr("type", "offer")
                    .attr("media", "audio")
                    .build()])
                .build(),
        ),
        (
            // A tag the derivation does not model. Both engines must fail the
            // same way, which is agreement rather than a finding.
            "11-unmodelled-presence",
            NodeBuilder::new("presence")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "available")
                .build(),
        ),
        (
            // Values wide enough that a short/long integer choice differs
            // between encoders, which must not change the derived event.
            "12-receipt-large-values",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-BIG-000000000000000000000001")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "read")
                .attr("t", "2147483647")
                .build(),
        ),
        (
            // A receipt from the bare server. One encoder writes that as a JID
            // with no user, another as a dictionary token — the difference
            // found in captured traffic, and for a long time enough to make the
            // derivation read one engine and not the other.
            "14-receipt-from-bare-server",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-SRV-1")
                .attr("from", "s.whatsapp.net")
                .attr("type", "read")
                .attr("t", "1700000003")
                .build(),
        ),
        (
            // Multi-byte characters, where a length in bytes and a length in
            // characters are different numbers.
            "13-receipt-unicode",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-Ünïcödé-😀")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "read")
                .build(),
        ),
        // Below: one stanza per L1 shape the corpus did not reach. The four
        // engines only ever agreed about five of the sixteen the derivation
        // models, because agreement is only tested over what the corpus
        // contains. Each of these is the leanest stanza that falls to its
        // shape rather than to a richer sibling.
        (
            "15-receipt-retry",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-RETRY-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "retry")
                .attr("t", "1700000020")
                .children([
                    NodeBuilder::new("retry")
                        .attr("id", "RCPT-RETRY-1")
                        .attr("count", "1")
                        .build(),
                    NodeBuilder::new("registration")
                        .bytes(vec![0x00, 0x00, 0x00, 0x07])
                        .build(),
                ])
                .build(),
        ),
        (
            // `type="offer"` is not one of the message-receipt types, so that
            // shape rejects it and this falls to the call one. The two demand
            // the same two attributes and are told apart by nothing else.
            "16-receipt-call",
            NodeBuilder::new("receipt")
                .attr("id", "RCPT-CALL-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "offer")
                .children([NodeBuilder::new("offer").build()])
                .build(),
        ),
        (
            // No `id` and no `class`, so the generic ack shape cannot claim it,
            // and the sync fields are what this one is for.
            "26-ack-sync",
            NodeBuilder::new("ack")
                .attr("t", "1700000028")
                .attr("sync", "1")
                .attr("count", "3")
                .build(),
        ),
        (
            "17-call-offer-notice",
            NodeBuilder::new("call")
                .attr("id", "CALL-NOTICE-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("t", "1700000021")
                .children([NodeBuilder::new("offer_notice")
                    .attr("call-creator", "5511999998888:1@s.whatsapp.net")
                    .attr("call-id", "CALLID-1")
                    .attr("type", "offer")
                    .attr("media", "audio")
                    .build()])
                .build(),
        ),
        (
            // A `<call>` with nothing on it at all: the shape that exists for
            // exactly this, and the one a richer sibling would swallow.
            "18-call-placeholder",
            NodeBuilder::new("call").attr("t", "1700000022").build(),
        ),
        (
            "19-ack-plain",
            NodeBuilder::new("ack")
                .attr("id", "ACK-PLAIN-1")
                .attr("class", "notification")
                .attr("type", "w:gp2")
                .build(),
        ),
        (
            "20-ack-read-receipt",
            NodeBuilder::new("ack")
                .children([NodeBuilder::new("readreceipts").build()])
                .build(),
        ),
        (
            "21-ack-newsletter-success",
            NodeBuilder::new("ack")
                .attr("id", "ACK-NL-1")
                .attr("class", "message")
                .attr("t", "1700000023")
                .attr("response_server_id", "17")
                .build(),
        ),
        (
            "22-ack-newsletter-negative",
            NodeBuilder::new("ack")
                .attr("id", "ACK-NL-NEG-1")
                .attr("class", "message")
                .attr("t", "1700000024")
                .attr("error", "404")
                .build(),
        ),
        (
            "23-ack-status-success",
            NodeBuilder::new("ack")
                .attr("id", "ACK-ST-1")
                .attr("class", "status")
                .attr("t", "1700000025")
                .build(),
        ),
        (
            "24-ack-status-negative",
            NodeBuilder::new("ack")
                .attr("id", "ACK-ST-NEG-1")
                .attr("class", "status")
                .attr("t", "1700000026")
                .attr("error", "500")
                .build(),
        ),
        (
            // Enough to be acknowledged and not enough to be decrypted: no
            // `<enc>`, so the full message shape cannot claim it.
            "25-message-ack-only",
            NodeBuilder::new("message")
                .attr("id", "MSG-ACKONLY-1")
                .attr("from", "5511999998888@s.whatsapp.net")
                .attr("type", "text")
                .attr("offline", "0")
                .attr("t", "1700000027")
                .build(),
        ),
    ]
}

fn main() -> std::io::Result<()> {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/wa-wire-conformance/corpus");
    std::fs::create_dir_all(&dir)?;

    // Removed first, so a renamed entry does not leave its old file behind for
    // the readers to pick up as an extra stanza.
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "bin") {
            std::fs::remove_file(path)?;
        }
    }

    for (name, node) in corpus() {
        let encoded = marshal::marshal(&node).expect("corpus stanza marshals");
        // The decoder's buffer is the marshalled bytes minus the leading format
        // byte, which is what an adapter forwards as the frame.
        let frame = &encoded[1..];
        let path = dir.join(format!("{name}.bin"));
        std::fs::write(&path, frame)?;
        println!("{}: {} bytes", path.display(), frame.len());
    }
    Ok(())
}
