//! Small app-owned JSON state files. Reads are bounded, writes replace the
//! file atomically, and the latest value is saved on a worker so a change made
//! on the UI thread never waits for the disk.
use serde::Serialize;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

#[cfg(any(target_os = "ios", test))]
pub(crate) mod background;
mod sync;
#[cfg(any(target_os = "ios", test))]
pub(crate) use sync::{SyncReport, sync_all};

/// The file's bytes, or `None` when it does not exist yet.
pub(crate) fn read(path: &Path, limit: u64) -> crate::Result<Option<Vec<u8>>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(crate::Error::StateFileSize {
            path: path.to_owned(),
            limit,
        });
    }
    Ok(Some(bytes))
}

pub(crate) fn write<T: Serialize + ?Sized>(path: &Path, value: &T) -> crate::Result<()> {
    let parent = path.parent().ok_or(crate::Error::MissingStateRoot)?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    // Buffered: the serializer writes each escape and punctuation mark on its
    // own, which unbuffered is a system call apiece.
    let mut buffer = io::BufWriter::new(&mut file);
    serde_json::to_writer(&mut buffer, value)?;
    buffer.write_all(b"\n")?;
    buffer.flush()?;
    drop(buffer);
    // No fsync: on macOS it is F_FULLFSYNC, which can outlast GPUI's 100 ms
    // quit budget. The rename still replaces the file atomically, and a file
    // torn by power loss is rejected on load in favor of the defaults.
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

/// Saves the most recent value handed to it. Values replaced before the
/// worker reads them are never written. Each writer registers itself so
/// `sync_all` can wait for its latest value.
pub(crate) struct Writer<T> {
    barrier: Arc<sync::Barrier>,
    /// The latest value and its generation (`sync::Barrier::queue`).
    pending: Arc<Mutex<Option<(u64, T)>>>,
    wake: mpsc::SyncSender<()>,
    worker: JoinHandle<()>,
}

impl<T: Serialize + Send + 'static> Writer<T> {
    pub(crate) fn start(name: &str, path: PathBuf) -> io::Result<Self> {
        Self::spawn(name, path, &sync::REGISTRY, |path, value| {
            write(path, value)
        })
    }

    /// Starts the worker with `write` and registers it with `registry`; tests
    /// pass their own of both.
    fn spawn(
        name: &str,
        path: PathBuf,
        registry: &sync::Registry,
        write: impl Fn(&Path, &T) -> crate::Result<()> + Send + 'static,
    ) -> io::Result<Self> {
        let barrier = Arc::new(sync::Barrier::new(name));
        let pending = Arc::new(Mutex::new(None::<(u64, T)>));
        let mailbox = pending.clone();
        let progress = barrier.clone();
        let finished = sync::FinishOnDrop(barrier.clone());
        let (wake, receiver) = mpsc::sync_channel(1);
        let label = name.to_owned();
        let worker = thread::Builder::new().name(name.into()).spawn(move || {
            let _finished = finished;
            for () in receiver {
                let snapshot = mailbox.lock().ok().and_then(|mut pending| pending.take());
                let Some((generation, snapshot)) = snapshot else {
                    continue;
                };
                if let Err(error) = write(&path, &snapshot) {
                    tracing::warn!(%error, file = label, "Cannot save state");
                }
                // A failed write is recorded too: a sync waits for the
                // attempt, not for a disk that may never accept the value.
                progress.written(generation);
            }
        })?;
        registry.register(&barrier);
        Ok(Self {
            barrier,
            pending,
            wake,
            worker,
        })
    }

    pub(crate) fn save(&self, value: T) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some((self.barrier.queue(), value));
        }
        // A full wake queue already guarantees that the latest value is read.
        let _ = self.wake.try_send(());
    }

    /// Waits until every value saved before the call was written (or failed
    /// to write), the worker ended, or `deadline` passed. Blocks, so callers
    /// run it off the UI thread.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the app syncs every writer through sync_all")
    )]
    fn sync_until(&self, deadline: std::time::Instant) -> sync::Synced {
        self.barrier.sync_until(Some(deadline))
    }

    /// Writes whatever is still pending, then stops the worker. Blocks, so
    /// callers run it off the UI thread.
    pub(crate) fn finish(self) {
        drop(self.wake);
        if self.worker.join().is_err() {
            tracing::warn!("State writer panicked");
        }
    }
}

#[cfg(test)]
mod tests;
