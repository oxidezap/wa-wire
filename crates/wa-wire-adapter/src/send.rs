//! Sending a stanza, whichever engine is underneath.
//!
//! The inbound half of the boundary is a sink an adapter pushes into. This is
//! the other direction, and it is not symmetric: sending touches a socket, so
//! it is fallible and it takes time. That makes it the one part of the boundary
//! that has to be async, and the one that needs an error type consumers can
//! act on.
//!
//! # What crosses
//!
//! The same frame bytes as inbound — the buffer a decoder consumes. Deliberately
//! the same definition in both directions, because the useful consequence is
//! that a captured envelope can be sent back as it stands. Record and replay
//! stop being separate features.
//!
//! What an engine wants underneath differs (one takes marshalled bytes, another
//! a decoded node), and converting is the adapter's job. A consumer that had to
//! know which would be a consumer coupled to an engine.
//!
//! # What this is not
//!
//! Not request/response. Sending a stanza and correlating the reply the server
//! sends back is [`Capability::L0Request`], a separate claim — an engine can
//! perfectly well let you write to the socket without giving you its
//! correlation table.
//!
//! [`Capability::L0Request`]: wa_wire_contract::Capability::L0Request

extern crate alloc;

use alloc::boxed::Box;
use core::fmt;
use core::future::Future;
use core::pin::Pin;

/// A future an adapter's send returns.
///
/// Boxed because [`StanzaSender`] is used behind a trait object: a consumer
/// holds "some engine" and the whole point is not knowing which.
pub type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), SendError>> + Send + 'a>>;

/// Why a stanza could not be sent.
///
/// Deliberately few variants. A consumer's realistic choices are retry, give up,
/// or reconnect, and a taxonomy finer than that would be an engine's error
/// vocabulary leaking through a boundary built to hide it.
#[derive(Debug)]
pub enum SendError {
    /// There is no live connection to send on.
    ///
    /// Distinct from [`Engine`](Self::Engine) because it is the one a consumer
    /// can act on without knowing anything about the engine: wait, reconnect,
    /// try again.
    NotConnected,
    /// The engine refused or failed the send.
    ///
    /// Carries the engine's own error so a report can name what happened, while
    /// keeping the type opaque so a consumer cannot come to depend on which
    /// engine produced it.
    Engine(Box<dyn core::error::Error + Send + Sync>),
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => f.write_str("not connected"),
            Self::Engine(error) => write!(f, "engine refused the send: {error}"),
        }
    }
}

impl core::error::Error for SendError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::NotConnected => None,
            Self::Engine(error) => Some(error.as_ref()),
        }
    }
}

/// Puts a stanza on the wire.
///
/// The outbound half of what an adapter provides. An adapter that implements it
/// declares [`Capability::L0Outbound`], and a consumer that needs to send
/// requires that capability at setup rather than discovering at runtime that
/// its engine only listens.
///
/// [`Capability::L0Outbound`]: wa_wire_contract::Capability::L0Outbound
pub trait StanzaSender: Send + Sync {
    /// Send one stanza, as the frame bytes a decoder would consume.
    ///
    /// Resolving means the engine accepted the frame for delivery — not that
    /// the server acted on it. Nothing at L0 can promise the latter: the answer
    /// to a stanza is another stanza, and it arrives inbound.
    ///
    /// # Errors
    ///
    /// [`SendError::NotConnected`] when there is no live connection, and
    /// [`SendError::Engine`] for anything the engine itself refused.
    fn send_frame<'a>(&'a self, frame: &'a [u8]) -> SendFuture<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[derive(Debug)]
    struct Refused;

    impl fmt::Display for Refused {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("socket closed")
        }
    }

    impl core::error::Error for Refused {}

    #[test]
    fn not_connected_reads_as_itself() {
        assert_eq!(SendError::NotConnected.to_string(), "not connected");
        assert!(core::error::Error::source(&SendError::NotConnected).is_none());
    }

    #[test]
    fn an_engine_failure_names_what_happened_and_keeps_the_cause() {
        let error = SendError::Engine(Box::new(Refused));

        assert_eq!(error.to_string(), "engine refused the send: socket closed");
        assert!(
            core::error::Error::source(&error).is_some(),
            "the cause is reachable for a report"
        );
    }

    /// A sender that records what it was handed, which is all a test of the
    /// boundary can check: what reaches the socket is the engine's business.
    ///
    /// Counts rather than collects, because [`StanzaSender`] is `Sync` and this
    /// crate is `no_std` — there is no `Mutex` to reach for, and the bytes are
    /// checked on the way in instead.
    #[derive(Default)]
    struct Recording {
        sent: core::sync::atomic::AtomicUsize,
        bytes: core::sync::atomic::AtomicUsize,
    }

    impl StanzaSender for Recording {
        fn send_frame<'a>(&'a self, frame: &'a [u8]) -> SendFuture<'a> {
            Box::pin(async move {
                self.sent
                    .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                self.bytes
                    .fetch_add(frame.len(), core::sync::atomic::Ordering::Relaxed);
                Ok(())
            })
        }
    }

    #[test]
    fn a_sender_is_usable_behind_a_trait_object() {
        // The property the boxed future exists for: a consumer holds "some
        // engine" and does not know which.
        let recording = Recording::default();
        let sender: &dyn StanzaSender = &recording;

        let sent = block_on(sender.send_frame(b"frame"));

        assert!(sent.is_ok());
        assert_eq!(
            recording.sent.load(core::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            recording.bytes.load(core::sync::atomic::Ordering::Relaxed),
            5,
            "the frame crossed as it stands"
        );
    }

    /// Minimal executor: this crate has no runtime dependency and should not
    /// grow one for a test. The futures here never yield, so a no-op waker is
    /// all a poll loop needs.
    fn block_on<T>(future: impl Future<Output = T>) -> T {
        use core::task::{Context, Poll, Waker};

        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut future = Box::pin(future);
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }
}
