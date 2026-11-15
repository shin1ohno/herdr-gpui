//! The TCP stream russh's session task owns, wrapped so the tunnel can end
//! that task without the host's cooperation and can tell when the task has
//! let go of it.
//!
//! russh spawns the session task during the handshake and only the
//! handshake's future can join it (`client::connect_stream`). Dropping that
//! future on a deadline would leave the task running with nobody able to
//! join it. Cutting the stream instead fails every read and write the task
//! makes, so it ends at once and the handshake's future completes after
//! joining it.
//!
//! The task owns the stream until its future finishes, so the stream's drop
//! also shows that the task has ended when its handle can no longer join it.

use std::{
    io,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    sync::Notify,
};

/// Shared between the tunnel's task and the stream inside the session task.
#[derive(Debug, Default)]
pub(crate) struct KillSwitch {
    killed: AtomicBool,
    /// Set when the stream is dropped, which happens only when the session
    /// task that owns it has ended (or never started).
    released: AtomicBool,
    dropped: Notify,
    reader: Mutex<Option<Waker>>,
    writer: Mutex<Option<Waker>>,
    /// A test fault: the stream ignores the kill, as a task stuck elsewhere
    /// would.
    #[cfg(test)]
    deaf: bool,
}

impl KillSwitch {
    /// `deaf` is a test fault and is ignored outside tests.
    #[cfg(test)]
    pub(crate) fn new(deaf: bool) -> Self {
        Self {
            deaf,
            ..Self::default()
        }
    }

    #[cfg(not(test))]
    pub(crate) fn new(_deaf: bool) -> Self {
        Self::default()
    }

    /// Fails every later read, write, flush, and shutdown, and wakes the
    /// task waiting on one.
    pub(crate) fn kill(&self) {
        self.killed.store(true, Ordering::SeqCst);
        for side in [&self.reader, &self.writer] {
            let waker = side.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    }

    pub(crate) fn released(&self) -> bool {
        self.released.load(Ordering::SeqCst)
    }

    /// Resolves once the stream has been dropped.
    pub(crate) async fn wait_released(&self) {
        loop {
            let mut dropped = pin!(self.dropped.notified());
            let _ = dropped.as_mut().enable();
            if self.released() {
                return;
            }
            dropped.await;
        }
    }

    /// Registers the caller to be woken by [`KillSwitch::kill`], then reports
    /// whether the stream is cut. Registering first means a kill in between
    /// is either seen here or wakes the caller.
    fn check(&self, side: &Mutex<Option<Waker>>, cx: &Context<'_>) -> io::Result<()> {
        {
            let mut waker = side.lock().unwrap_or_else(PoisonError::into_inner);
            if !waker
                .as_ref()
                .is_some_and(|waker| waker.will_wake(cx.waker()))
            {
                *waker = Some(cx.waker().clone());
            }
        }
        #[cfg(test)]
        if self.deaf {
            return Ok(());
        }
        if self.killed.load(Ordering::SeqCst) {
            Err(io::ErrorKind::ConnectionAborted.into())
        } else {
            Ok(())
        }
    }
}

/// A stream that fails once its [`KillSwitch`] is killed.
pub(crate) struct Killable<S> {
    inner: S,
    switch: Arc<KillSwitch>,
}

impl<S> Killable<S> {
    pub(crate) fn new(inner: S, switch: Arc<KillSwitch>) -> Self {
        Self { inner, switch }
    }
}

impl<S> Drop for Killable<S> {
    fn drop(&mut self) {
        self.switch.released.store(true, Ordering::SeqCst);
        self.switch.dropped.notify_waiters();
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Killable<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if let Err(error) = this.switch.check(&this.switch.reader, cx) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Killable<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if let Err(error) = this.switch.check(&this.switch.writer, cx) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if let Err(error) = this.switch.check(&this.switch.writer, cx) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if let Err(error) = this.switch.check(&this.switch.writer, cx) {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests;
