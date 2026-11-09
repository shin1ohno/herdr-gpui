# herdr-core client

A client of [herdr-core](https://github.com/herdrdev/herdr-core) (`hc`,
Herdr 2), the terminal runtime that keeps programs running and streams their
raw output to viewers. It speaks protocol generation 1: JSON-RPC on
channel 0 and framed byte streams on the others (`schema/README.md` in
herdr-core). No GPUI or async runtime.

- `Endpoint::resolve` finds a session's socket or pipe the way `hc` does.
- `Core::connect` joins a running core and never starts one; `start` runs
  `herdr-core server start` when the app decides to.
- `Core::over` speaks the same protocol over an application-owned `Read + Write`
  transport, such as SSH running the core's existing `hc connect --session NAME`.
  The worker retains the transport and any tunnel guard it owns until disconnect.
- `Core::disconnect` signals shutdown without draining queued commands or joining
  the worker; the core and its terminals keep running.
- `Core::attach` and `Core::subscribe` return streams whose reads give
  credit back as bytes are taken, so a slow view never blocks other calls.

- `CoreBackend` implements `sheprd_backend::Backend`: it emulates attached
  terminals with `sheprd-terminal`, so the app gets the same cells and sends
  the same semantic input as with Herdr 1.

Every call blocks: use it from a worker thread, never from the UI thread.

The opt-in `remote_ssh` integration test runs an explicitly selected core through
its real `hc connect` command and a private OpenSSH server. Select absolute
paths explicitly; it never discovers a user's daemon:

```sh
HERDR_CORE_TEST_BINARY=/absolute/path/to/herdr-core \
HERDR_TEST_SSHD=/absolute/path/to/sshd \
HERDR_TEST_SSH_KEYGEN=/absolute/path/to/ssh-keygen \
HERDR_TEST_TMPDIR=/tmp \
cargo test --locked -p herdr-core-client --features integration-test \
  --test remote_ssh -- --ignored
```

The harness clears the core's environment on both sides of SSH, stops only
its private core and terminals, and kills and reaps only the sshd it started.
