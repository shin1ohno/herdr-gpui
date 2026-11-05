# Native iPad chrome comparison

These original screenshots show an isolated test daemon and synthetic review
labels on an iPad Pro 11 Simulator, iOS 26.5. They contain no personal terminal
session. The native window is 1194 x 834 points; this is not narrow-window proof.

| Files | Native source |
| --- | --- |
| `nord-before.png`, `latte-before.png` | Accepted naming implementation before the chrome changes; executable SHA-256 `b15ee14709f7667ad0f3678638dac63b2451b22d93f231bb80c0f13350b29bd5` |
| `nord-after.png`, `latte-after.png` | Frozen combined-1; original executable SHA-256 `d09907e13ed4a23b4a496b596110ad8ac6d2c87ec835b4582fe82717a4dfeac0` |

The Nord after image uses the expanded `en-XA` pseudo-locale. Tab selection,
44-point height, removed account/status-keyboard slots and long labels are
visible. The yellow band identifies the development worktree branch.

The independent native QA also operated tab creation/closure, folded host
reveals, Space/Agent navigation and permanent Devices/Settings rail controls.
See [SH1-91](https://linear.app/shin1ohno/issue/SH1-91),
[SH1-92](https://linear.app/shin1ohno/issue/SH1-92) and
[SH1-81](https://linear.app/shin1ohno/issue/SH1-81).

These images establish chrome observations only. Later clipboard changes were
validated on combined-2 using the published UIKit dependency; its separate
cold-paste and exact terminal-byte evidence is recorded in
[SH1-90](https://linear.app/shin1ohno/issue/SH1-90). Physical keyboard events,
actual narrow OS windows and synthetic Escape guide dismissal remain unproven.
