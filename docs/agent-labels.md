# Agent labels

Status: chosen policy for SH1-89. The user selected work/conversation titles
first, with manual GUI Rename taking highest priority.

Herdr 1 agent rows use the following primary label, stopping at the first
nonblank, useful value:

1. The GUI pane Rename, from the matching snapshot pane's `label`.
2. The reported agent `title`.
3. The registered agent `name`.
4. A custom `display_agent` value.
5. The pane ID, followed by the matching tab label when available.

A GUI Rename is explicit and always wins, even when it is a kind word.
Other candidates that exactly match a canonical agent kind, the current
agent's kind, `agent`, or `Claude Code` do not describe a task. Matching is
case-insensitive and compares whole values. Useful words inside custom names
are never removed. An identifier such as `cos-stable-control-name` receives
the same treatment as every other registered name; its spelling supplies no
provenance. A useful reported title can take its place on screen without
altering the identifier.

`display_agent` is presentation metadata supplied by an integration, not a
reliable task-title field or a canonical identity. Upstream resolves it and
`title` independently from valid metadata reports. Custom values remain a
fallback; engine-only display values give way to real pane/tab information.
The terminal's OSC `terminal_title` and `terminal_title_stripped` are not
reported task metadata and are not candidates for the primary label.

Captions remove leading paste-transport envelopes, controls and direction
overrides, and are bounded to 160 characters. Pane labels are matched using
pane, tab and workspace IDs within the current endpoint's snapshot. Each
new accepted snapshot resolves the label again. A Rename therefore survives
title changes and refreshes while its pane label remains present; clearing
it restores the next candidate. The UI does not keep a separate rename
override or change daemon control names.

The normal sidebar, iPad panel and rail, rail touch menu, Go To labels and
previews, and close confirmation use the same resolver. Expanded-row
accessibility uses that primary label and omits a duplicate pane caption.
The kind icon and state indicator retain their existing source fields.
The iPad right column remains workspace/branch/status, not pane cwd.
Grouping, priority order and plugin view order do not depend on the label.

Go To can find the primary label and the original registered name, kind,
custom display metadata, reported title and Rename label. Its existing
terminal-title search keywords do not make those titles primary task labels.
The Spaces search remains a device/workspace/branch/note search. Explicitly
configured sidebar token rows retain their existing token semantics and
plugin styling rules, including the `agent` presentation token; they are a
user-specified alternative to the normal rows. Existing mods/device overview
API projections retain their legacy identity captions in this focused change.

Herdr 2 terminal rows have a separate resolver: live view title, terminal
record title, record name, cwd basename, then terminal address. This policy
does not change those rows or either daemon's data.

The protocol has no field distinguishing a human-registered name from an
orchestrator's registered control name. Missing task metadata cannot be
reconstructed by the client. Pane/tab information is a location fallback,
not an invented task description.
