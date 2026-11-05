# UIKit input regression patch

The client pins the signed, published Git revision
[`2ff3bccd4609e665af89b1b444c95452de5e1b78`](https://github.com/shin1ohno/herdr-uikit/commit/2ff3bccd4609e665af89b1b444c95452de5e1b78).
[UIKit PR #1](https://github.com/shin1ohno/herdr-uikit/pull/1) reviews that source.
No local Cargo path override is required.

`herdr-uikit-ime.patch` reproduces that source from
`d92c542f7ccab81c2b2b726ff96b9e0313c69d38`, retaining its window-corner and
top-bar geometry. It restores the Japanese input-source, composition, Unicode
selection, and confirmation/cancellation fixes from
`c3eb6e736eb7e268d8f2b2806f1c7fb3ad3894e6`, which was not an ancestor of the
previous dependency. It clears stale user-dismissal state only when the owned,
requested native text responder receives DidShow again.

Apply the reproducible patch in an isolated checkout of that exact base:

```sh
git apply --unidiff-zero --check /path/to/herdr-gpui/docs/ios-patches/herdr-uikit-ime.patch
git apply --unidiff-zero /path/to/herdr-gpui/docs/ios-patches/herdr-uikit-ime.patch
```

The published UIKit source passed formatting, 340 simulator regressions, and
all-target/all-feature simulator clippy with `RUSTFLAGS="-D warnings"` on
Rust 1.96.1. Physical hardware keyboard behavior is a separate verification.

For a bounded Escape-routing investigation, launch the app with
`HERDR_IOS_ESCAPE_DIAGNOSTICS=1`. Diagnostics contain only fixed routing stages,
handler/responder/composition booleans, and guide state. They never record text
or arbitrary keystrokes. With a missing handler, `handler_available=false`
distinguishes unknown composition from an observed unmarked handler.

The earlier patch's observations in [verification.md](verification.md) are
historical, not acceptance for the current client. Current integration work is
tracked in [SH1-90](https://linear.app/shin1ohno/issue/SH1-90).
