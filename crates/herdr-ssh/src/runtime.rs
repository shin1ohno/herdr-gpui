//! The one tokio runtime russh runs on: current-thread, on a dedicated
//! thread started on first use, so nothing async reaches this crate's
//! callers and no other crate sees tokio.

use std::{
    sync::{Mutex, OnceLock, PoisonError},
    thread,
};

use tokio::runtime::{Builder, Handle};

/// Blocking threads serve name resolution and the signing closure; a few
/// suffice, and a small cap keeps lookups that cannot be interrupted from
/// piling up behind repeated reconnects.
const MAX_BLOCKING_THREADS: usize = 2;

static HANDLE: OnceLock<Handle> = OnceLock::new();
/// Held while starting, so two first callers start one runtime.
static STARTING: Mutex<()> = Mutex::new(());

/// The runtime's handle, starting it on first use.
pub(crate) fn handle() -> std::io::Result<&'static Handle> {
    if let Some(handle) = HANDLE.get() {
        return Ok(handle);
    }
    let _starting = STARTING.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(handle) = HANDLE.get() {
        return Ok(handle);
    }
    let runtime = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(MAX_BLOCKING_THREADS)
        .thread_name("herdr-ssh")
        .build()?;
    let handle = runtime.handle().clone();
    // The thread drives every task spawned on the handle for the life of
    // the process; it is never joined. Task panics are caught by tokio and
    // do not end it.
    thread::Builder::new()
        .name("herdr-ssh".into())
        .spawn(move || runtime.block_on(std::future::pending::<()>()))?;
    Ok(HANDLE.get_or_init(|| handle))
}
