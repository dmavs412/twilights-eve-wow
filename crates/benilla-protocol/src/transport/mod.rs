//! The one connection the protocol talks through: [`Conn`], a byte pipe over `TcpStream`
//! ([`native`]).
//!
//! - **Reads are `async`** ([`ReadExactAsync`]). The future resolves immediately around a
//!   blocking `read_exact`, which is correct because the sequencer runs on its own thread under
//!   `block_on`.
//! - **Writes stay synchronous** ([`std::io::Write`]). That keeps every one of the writer's ~200
//!   verbs a plain sync call.
//!
//! `Conn` is pre-split (both directions on one value) for the handshake, and [`Conn::split`] hands
//! out the two halves the streaming phase needs: two `try_clone`d sockets.

mod native;
pub use native::{Conn, ConnReader, ConnWriter};

/// Read exactly `buf.len()` bytes, or fail — [`std::io::Read::read_exact`]'s contract, awaited.
///
/// `async fn` in a trait makes the returned futures un-nameable and un-`Send`, which the
/// `async_fn_in_trait` lint warns about; both are fine here: the only caller is the sequencer
/// future, which is polled on the thread that owns the connection.
#[allow(async_fn_in_trait)]
pub trait ReadExactAsync {
    /// Fill `buf` completely. On end of stream: [`std::io::ErrorKind::UnexpectedEof`]; on an
    /// expired read timeout: [`std::io::ErrorKind::TimedOut`] (or `WouldBlock`, which is what a
    /// native socket's own timeout raises).
    async fn read_exact_async(&mut self, buf: &mut [u8]) -> std::io::Result<()>;
}
