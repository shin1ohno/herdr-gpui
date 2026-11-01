# iPad app end-to-end QA log (simulator)

Environment: iPad Pro 13-inch (M5) simulator C2348F15-A037-4111-8712-5926A51CFE7F,
iOS 26.5, app `dev.herdr.gpui.ios-smoke` built from the uncommitted working tree
on top of 0f04443c. Input is injected with an XCUITest driver
(`qa/driver`, `qa.sh run '<json actions>'`); screenshots in `qa/driver/shots`.
App coordinates are points; screenshot pixels / 2 = points.

## Verified

- V1 Touch path: `examples/hello` button tapped 3 times through XCUITest's
  synthesized UIKit touches; the counter reads 3 (`shots/hello-taps5.png`).
  UIKit touches → herdr-uikit `TouchEvent` → GPUI tap → click works.

- V2 Device picker → Add Device opens the dialog with the SSH target focused,
  the software keyboard up (Japanese layout), Bonjour "Searching…", and the
  `restrict ecdsa-sha2-nistp256 … herdr-ipad-mini` line; "Copy public key"
  puts exactly that line on the pasteboard (`xcrun simctl pbpaste`).
  `shots/picker2.png`, `shots/add-device.png`.

- V3 Software-keyboard typing into the SSH target field (keys tapped through
  XCUITest's keyboard elements) and the delete key both edit the GPUI field.
- Fixture: `qa/fixture/qa-fixture.sh` (written by Codex, live-tested here):
  private sshd on 127.0.0.1 with the app's key, every session jailed by
  `ForceCommand` into `/tmp/hq.*` (HOME, PATH, XDG_*, HC_SOCKET), Herdr 1
  0.6.6 and the pinned herdr-core linked into the jail. A throwaway-key SSH
  self-test resolved `herdr`/`hc` inside the jail; `stop` removed it.
- V4 First Add Device to `shin1ohno@127.0.0.1:<port>` shows the host-key
  prompt with `ED25519 256 SHA256:bx9q…X9w`, identical to the fixture's
  `ssh-keygen -lf`. `shots/after-add.png`.
- V5 Trust key → the app authenticates with the simulator's software P-256
  device key (sshd: `Accepted publickey … ECDSA`) and runs the Herdr 1 probe.
- V6 A second Add Device to the now-pinned host connects with no prompt.
- V7 With Herdr 1 removed from the jail, the probe falls back to the core
  probe and offers "Add Herdr 2 host"; saving adds the device to the sidebar.
  `shots/add2.png`, `shots/added-core.png`.
- V8 The titlebar "+" opens a Herdr 2 terminal on that host: the core starts
  over SSH and `sh-3.2$` renders; the device dot turns green.
  `shots/plus.png`.
- V9 Typing `echo hello` + Return on the software keyboard runs it remotely
  and `hello` renders. `shots/term-typed.png`.

- V10 `xcrun simctl terminate` of the app leaves the remote Herdr 2 terminal
  live (`hc ls` before and after: `t1 live`).
- V11 After relaunch the Terminal tab is restored but not connected (no core
  started by the restore). Creating a terminal with "+" connects the core, and
  the restored tab then reattaches with its history. `shots/relaunch.png`,
  `shots/tab1.png`.
- V12 Same shell: `export QA=persist` before the app was killed, `echo $QA`
  after → `persist`. `shots/qa-var.png`.
- V13 Herdr 1 over the in-process SSH bridge (fixture B, Herdr 0.9.5 from the
  official release, jailed; probe reports generation 1 and the three required
  capabilities): host-key prompt matches `SHA256:q2m8…jp1Y`, Trust key adds
  the device, it connects, and its `home` workspace appears.
  `shots/addb-key.png`, `shots/addb-trust.png`.
- V14 Opening `home` shows the remote Herdr 1 pane `sh-3.2$`; typing works.
  `shots/h1-home.png`.
- V15 A pane split made on the remote Herdr 1 (`herdr pane split` inside the
  jail) appears live on the iPad; the ACP pane (`herdr-acp -- python3
  mock_acp.py`, a minimal ACP agent in `qa/acp/mock_acp.py`) is listed under
  Agents. `shots/acp-1.png`.
- V16 With `[features] acp_chats = true` (written to the QA app's
  `config-gpui.local.toml`; hot-reloaded), the pane becomes the native chat
  view and the app runs `herdr-acp connect` on the host over the in-process
  SSH transport (sshd commands log). `shots/acp-4.png`.
- V17 Native chat round trip: "hello agent" → "qa-mock heard: hello agent",
  "Worked 20ms". `shots/acp-5.png`.
- No retry storm: fixture A's sshd saw 5 sessions in total (2 Herdr 1 probes,
  3 Herdr 2), none from a Herdr 1 reconnect loop on the Herdr 2-only host.

## Findings

- F1 (UI, iPad-only feature leak): the sidebar footer's server icon opens a
  "LOCAL SESSIONS" popover ("Stopped sessions start their daemon when
  selected", `default` stopped, "Add session… Create and connect"). An iPad has
  no local daemon. `shots/devices-icon.png`. The device picker repeats it:
  "Local — This device · disabled" and "Sessions… Switch, add, or delete this
  device's sessions" (`shots/picker2.png`).
- F2 (input, to reproduce): the device picker did not open on the first tap
  after launch (112,1334) nor on a tap right after an Escape key press
  (220,1334); it opened on a later tap (130,1334). `shots/picker.png`,
  `shots/chevron.png`.
- F3 (layout): with the software keyboard up, the Add Device dialog's primary
  "Add device" button sits at the keyboard's top edge and is almost hidden;
  the dialog does not shrink or scroll above the keyboard.
  `shots/add-device.png`.
- F4 (accessibility): the first-responder text input view is not an
  accessibility element, so XCUITest `typeText` fails with "Neither element
  nor any descendant has keyboard focus". VoiceOver and other assistive input
  presumably cannot find the focused text field either.
- F5 (editing): the keyboard assistant bar's Paste button does nothing in a
  GPUI text field (pasteboard held `qa@example`; the field stayed empty).
  `shots/pasted.png`.
- F6 (copy): the host-key prompt prints the full simulator/app container path
  of the known_hosts file in its explanation.
- F7 (logic): `in_process::probe` (crates/herdr-client/src/ssh/in_process.rs
  ~L236) checks for Herdr 2 only when Herdr 1 is `Missing`. A host whose
  Herdr 1 is `Outdated` (0.6.6 here: no `endpoint_protocol_generation`) but
  has a working `hc` gets only "Install or update Herdr … then check the
  device again here" and a Cancel button: no Herdr 2 path and no "check
  again" control. `shots/kb-hidden.png`.
- F8 (input mode/layout): after the Add Device dialog closes, the iOS
  floating input-assistant bar stays at the bottom and covers the app's
  footer (device picker, status line), though no text field has focus.
  `shots/device-row.png`.
- F9 (touch): a long press on the sidebar device row opens nothing; the
  device (host) menu that holds "New Herdr 2 terminal" is right-click only.
  The titlebar "+" happened to create the Herdr 2 terminal (V8).
  `shots/device-long.png`.
- F10 (touch/recovery): after relaunch the restored Herdr 2 tab says
  "herdr-core is unavailable: Herdr 2 terminals are unavailable on iPad until
  a remote core is connected." with no Reconnect control; tapping the
  sidebar's `t1` does nothing. Only "+" (a new terminal) reconnects.
- F11 (layout, major): the software keyboard does not shrink the terminal or
  the window content. Herdr 1 pane: `stty size` = 63×88 with the keyboard up,
  same rows as with it down (63×89 on the Herdr 2 pane); `seq 70` scrolls the
  prompt and rows 54–70 behind the keyboard. The sidebar footer is also
  covered. `shots/h1-stty.png`, `shots/h1-seq.png`, `shots/tab1.png`.
  F11 is intermittent: with the same binary, focusing the native chat
  composer after a config hot-reload did shrink the whole window above the
  keyboard (footer moved up, `shots/acp-4.png`).
- F14 (storage): the GUI config is `$HOME/.config/herdr/config-gpui*.toml`
  at the app container's root, while state moved to `Library/Application
  Support`; the container root is not a documented app location.
- F12 (input, blocker for touch-only iPads): after the keyboard's hide key,
  tapping the terminal, switching tabs and back, or tapping elsewhere never
  shows the software keyboard again; the cursor stays focused. `shots/stty.png`,
  `shots/tab-switch.png`.
- F13 (state): the device picker lists the Herdr 2-only host as
  "default · connecting" and "2 devices · 0 connected" while its Herdr 2
  terminal is live and the sidebar dot is green. `shots/picker3.png`.
- F6 (addendum): the explanation under the fingerprint is cut off mid-line
  and cannot be scrolled. `shots/addb-key.png`.
- R1 (risk, observed): macOS sshd (OpenSSH 10.5) logs `srclimit_penalise …
  penalty: connections without attempting authentication` after each key
  scan, which disconnects before authenticating by design. Not blocking in
  this run (1 s penalty); repeated scans could be refused for longer.
- Note: the key comment is `herdr-ipad-mini`; in the simulator the device name
  apparently comes from the host Mac ("mini"), not "iPad Pro 13-inch (M5)".

## Re-verification (Codex)

Completed on 2026-10-31 with the current uncommitted working tree, the dedicated
QA app and private SSH fixtures only. Root performed the native checks; an
independent evidence reviewer inspected the screenshots and action/fixture
logs. `CODEX-FINAL-COVERAGE.md` maps every V1–V17 and F1–F14 to evidence;
`CODEX-COMMANDS.log` records the full commands and exit statuses. Screenshot
stems below refer to `driver/shots/STEM.png` and its accompanying driver log.
Earlier failed attempts remain in the archive and are superseded only by the
explicit successful reruns named here.

### Findings

| Finding | Final status | Native re-verification and commands/evidence |
| --- | --- | --- |
| F1 local sessions on iPad | Fixed in inspected states | Picker contains saved remotes only (`codex-picker-final`); live Core selection offers no Sessions (`codex-picker-core-online`, `codex-core-expand`). Selected remote Herdr1 Sessions remain intentionally supported. Host tests cover empty catalog and selected-device gating. |
| F2 intermittent first picker tap | Not reproduced | First picker tap succeeds in `codex-picker-final`; no absolute intermittent-failure fix claim. |
| F3 Add Device under keyboard | Fixed | Wide and375pt native windows keep header/footer/actions above keyboard. `codex-outdated-core-offer`, `codex-narrow-light-offer`; touch drag scroll in `codex-narrow-scroll-light` reveals complete final recovery advice without moving actions. |
| F4 accessibility/typeText | Fixed for focused XCUITest input; VoiceOver needs device | Driver `type` succeeds for target/session fields, H1/Core terminals and chat. `codex-native-traits-type` runs fresh echo$QA/stty, showing persist and44x89. Later field/chat/H1 tests also pass. |
| F5 keyboard assistant Paste | Fixed | Paste qa@example after native Allow Paste and one-time onboarding (`codex-paste-tutorial`). Screenshot proves exact value; redundant second Continue caused that script's failure and is not claimed passed. Exact public-key Copy comparison passes separately (`codex-public-key-copy-final`, notes). |
| F6 host-key text/path | Fixed in wide native prompt | `codex-fixture-c-key-retry` shows complete fingerprint, ssh-keygen hint and app-storage explanation; no container path. Root matched fixture's fingerprint with ssh-keygen. Narrow wrapping has host regression coverage; native narrow scrolling was checked on recovery advice. |
| F7 outdated Herdr1 hides Core/retry | Fixed | Old0.6.6+Core offer shows all3 actions (`codex-outdated-core-offer`). Fixture-only symlink upgrade to0.9.5 then Check again connects original qa-f7 target/session (`codex-check-again-upgraded`, `codex-fixture-c-session.json`). Later Core-only and outdated offers are rechecked, explicit Core registration succeeds (`codex-core-only-offer`, `codex-outdated-core-reconfirmed`, `codex-core-host-saved`). Root confirms C/run/hc.sock remains absent after registration. |
| F8 assistant after dialog close | Fixed | Close asserts keyboard:false (`codex-add-closed`); Core saved/Done also closes keyboard and assistant (`codex-window-restored`). |
| F9 long-press menus | Fixed for four requested targets | Driver1s long presses open correct device/workspace/tab/pane menus: `codex-device-menu`, `codex-workspace-menu`, `codex-tab-menu`, `codex-pane-menu`. Rail/agent/clipping/occlusion/release-click suppression have host tests; real pointer remains device QA. |
| F10 restored Core reconnect | Fixed | Explicit Reconnect keeps original t1/t2 and history (`codex-core-reconnect`, before/after JSON). Fresh echo$QA→persist follows in `codex-native-traits-type`. Sidebar-row recovery has host tests. |
| F11 keyboard viewport | Fixed for both backends | H1 stty up42x42/down58x42 (`codex-h1-rows-up`, `codex-h1-hidden-key`); Core up44x89/down63x89 (`codex-native-traits-type`, `codex-core-rows-down`). Both seq70 prompts/footer remain above keyboard (`codex-h1-final-resize`, `codex-core-seq`). |
| F12 keyboard cannot return | Fixed in simulator scenarios | H1 hide/retap/type, then hide/home/activate stays hidden until retap/type (`codex-h1-keyboard-recovery`). Core Hide→t2→t1→retap/type succeeds (`codex-core-tab-recovery-final`); unavailable Core correctly rejects input until explicit reconnect. Chat hide/retap/type produces a new reply (`codex-chat-hide-retap`). |
| F13 Core picker says connecting | Fixed connected case | `codex-picker-core-online` shows2devices/2connected, Core Connected text and green dot matching live terminal. Host tests cover disabled/disconnected projection. |
| F14 config outside Library | Fixed with migration | Config now uses Library/Application Support; root confirms old/new override bytes identical after automatic migration (CODEX-NOTES). Existing destination wins; original files/conflicts retained; actual migration reads bounded to1MiB. Seven path/migration tests pass; see CODEX-F14.md. |
| R1 sshd unauthenticated-scan penalty | Open nonblocking risk | Original B logs retain deferred1s penalties after no-auth scans; later public-key authentication succeeds. No stress/rate-limit mitigation claim. |

Native actions were replayed with `bash $QA/driver/qa.sh run '<actions>' STEM`;
full action JSON is in the journal/driver logs. Changes to Herdr binary links
and split commands were confined to the private fixtures. No personal daemon,
SSH settings or other worktrees were altered.

### V1–V17 regression pass

| Check | Result and evidence |
| --- | --- |
| V1 | PASS, current UIKit hello counter0→3 after exactly3 taps (`codex-hello-zero`, `codex-hello-three`). |
| V2 | PASS, Add Device/key line/Copy (`codex-add-keyboard`, `codex-public-key-copy-final`, exact-key comparison in notes). |
| V3 | PASS, target/session/label typeText plus trailing-x Delete1 (`codex-narrow-light-offer`); correct target displayed. |
| V4–V5 | PASS, full matching unknown C fingerprint, explicit trust and Accepted publickey ECDSA in private C sshd.log (`codex-fixture-c-key-retry`). Simulator signer only. |
| V6 | PASS, same pinned C host added for qa-core without another host-key prompt (`codex-outdated-core-registration`, `codex-narrow-light-offer`). |
| V7 | PASS, missing-Herdr1 Core-only offer, then outdated-Herdr1 Core offer and explicit registration/no automatic Core start (`codex-core-only-offer`, `codex-outdated-core-reconfirmed`, `codex-core-host-saved`). |
| V8 | PASS, select new Core host, titlebar+, fresh shell pwd/echo c-core (`codex-new-core-host-empty`, `codex-new-core-terminal`). |
| V9 | PASS, real remote input/output on both backends (`codex-native-traits-type`, `codex-h1-keyboard-recovery`, `codex-new-core-terminal`). |
| V10–V12 | PASS, restored/reconnected same A t1/t2, fresh QA=persist; C t1 also remains live after detach (`codex-core-reconnect`, A before/after JSON, `codex-native-traits-type`, `codex-core-c-after-detach.json`). |
| V13–V14 | PASS, new compatible Herdr1 qa-f7 connection plus existing B pane typing (`codex-check-again-upgraded`, `codex-fixture-c-session.json`, `codex-h1-keyboard-recovery`). |
| V15 | PASS, private `herdr pane split w1:p1 --direction down --no-focus` returns new w1:p3, which appears live; ACP remains listed (`codex-before-live-split`, `codex-live-split`, `codex-live-split.json`). |
| V16–V17 | PASS, current native ACP attach and fresh codex final/after hide roundtrips (`codex-acp-roundtrip-final`, `codex-chat-hide-retap`; replies visible in `codex-live-split`). |

### Gates and limits

Final successful command results are recorded in CODEX-COMMANDS.log:

- `just format`: PASS.
- Focused host GPUI regression suite:49 passed,0failed (`codex-host-regressions-fixed.log`). Earlier stale-width test-fixture failure was corrected and rerun.
- Focused herdr-client tests:8 passed (`codex-final-client-tests.log`).
- `just test-ios`:226 UIKit tests plus9 Python script tests passed (`codex-test-ios-done.log`). Earlier notes saying225 are stale.
- Host `cargo clippy --offline --locked -p herdr-uikit -p herdr-client -p herdr-gpui --all-targets -- -D warnings`: PASS (`codex-host-clippy-final-tree.log`).
- `just lint-ios`: PASS, device/simulator targets (`codex-lint-ios-final-verified.log`).
- `git diff --check`, `bash scripts/check-file-size.sh`, `just check-sorted`, `just check-i18n`: PASS (`codex-*-handoff.log`, `codex-size-last.log`).
- `just ci`: not run; disk remained below the handoff's8GiB threshold.
- `just lint-linux`: failed/unavailable Docker check. `just lint-windows`: interrupted during cargo-lock/disk pressure; Windows target unavailable. Neither cross check passed.

The actual native simulator UI was checked in dark and light themes and in a
375pt Stage Manager window with the software keyboard. Text wraps, the modal
scrolls and header/footer/actions remain visible. The driver's orientation
change did not rotate the physical simulator view: rotation is unverified.
No semantic IME-composition claim is made from plain Romaji typing.

Needs device: Local Network denial/grant/recovery, Secure Enclave and locked
signing behavior, real trackpad/right-click/terminal mouse reporting,
Japanese/Chinese composition/reconversion, physical scenes/multitasking and
rotation, and VoiceOver. Desktop-native UI checks were not run.

Cleanup: see CODEX-HANDOFF.md.

## Re-verification (Claude, 2026-11-01, after the final review fixes U2/R1)

Build: `herdr-gpui.app` bundled 02:45 from the working tree with U2 (keyboard state
machine), R1 (bounded config reads, Check again tests, migration recovery) and the
review follow-ups. Fixture D: `/tmp/hq.STMvwG`, `shin1ohno@127.0.0.1:55744`, Herdr
0.9.5, host key `SHA256:Xvy6QcUKn001MR7SQXeETcuGcSpIKk/0mPBFlvb2eWs`.

- UIKit simulator suite (integrator-run outside the sandbox, U2 could not reach
  CoreSimulator): 230 passed, 0 failed, incl. the new hit-test
  (`claude-uikit-simtest.log`).
- Add Device → TOFU prompt shows the fixture's exact fingerprint → Trust key → QA D
  connected, workspace `home` listed (`claude-k3`, `claude-k4`, `claude-k5`).
- Herdr 1 terminal: keyboard shown on focus, window shrinks above it (`claude-k6`).
- PASS hide → tap terminal → keyboard returns → `stty size` typed and answered 44 88
  (`claude-kA`).
- PASS with the keyboard visible on the numbers plane, two more terminal taps keep it
  visible on the same plane; the app log shows no WillHide/WillShow or responder
  restart during the taps (`claude-kC`).
- PASS hide → tap empty sidebar → hidden; tap a workspace row → hidden (`claude-kB`
  steps 5–10).
- **F15 FAIL** hide → tap the device's host row → keyboard comes back (`claude-kB`
  step 13). Log: the hide key resigns the text view before WillHide
  (`first_responder:false`), so `Dismissal::User` is never recorded, and the row's
  press/click produces GPUI FocusLost→FocusGained, whose `set_text_input_focused`
  clears dismissal and re-acquires. Sent to Codex as `fix/TASK-U3.md`.
- F2 re-observed once: the first tap right after the XCUITest runner re-activated the
  app (picker at 112,1335) did nothing; the next run's tap opened it (`claude-k1`,
  `claude-k2`).
- Fixture E: `/tmp/hq.CO21Rq`, `shin1ohno@127.0.0.1:56018`, default (outdated
  0.6.6) Herdr 1 plus core, host key `SHA256:SAIZvxrWESQ267FldY+mS4xdgnORIyiURrf52iGTYLs`.
  Add Device → exact fingerprint → Trust key → "Herdr 1 too old, Herdr 2 installed"
  with Check again and Add Herdr 2 host → saved, keyboard closed with the dialog
  (`claude-e3`, `claude-e4`, `claude-e5`).
- PASS long press on the QA E host row opens its device menu; New Herdr 2 terminal
  starts t1 over SSH with the keyboard up (`claude-e6`, `claude-e7`).
- PASS Herdr 2 terminal: hide → tap → keyboard returns → `stty size` 44 89
  (`claude-eA`).

### After U3 (F15 fix), build of 03:25

- UIKit simulator suite (integrator-run): 244 passed, 0 failed (`claude-u3-verify.log`).
- Relaunch restores the Herdr 2 tab as a Reconnect placeholder with no automatic
  connection; Reconnect returns to the live t1 with its earlier output (`claude-u0`,
  `claude-u1`).
- Correction to F15's trigger: the FocusLost→FocusGained pair at 02:49:32 came from the
  workspace row tap (t=5.15 s), and the keyboard's show landed after that step's check,
  so the host-row step reported it.
- PASS Herdr 1: hide → workspace row ×2 → host row: hidden throughout; the log shows two
  FocusLost→FocusGained pairs with `dismissal: User` kept and no WillShow (`claude-u4`).
- PASS Herdr 2: hide → empty sidebar → host row ×2 → hidden (FocusLost with User kept),
  terminal tap restores it and `echo f15-core` runs (`claude-u2`).
- PASS background: user-hidden → Home → activate stays hidden; terminal tap restores
  and types; visible → Home → activate returns visible (`claude-u3`). On return UIKit
  re-posts six show pairs and one hide while the scene is still inactive; the reducer
  ignores them (`active:false`), and the layout inset changes only twice (20 → 405.5),
  so nothing flickers.

### Full herdr-gpui suite after the review fixes (Claude)

First full run (`claude-final-gates.log`): 4701 passed, 2 failed.

- `ipad_picker::picker_connected_count_uses_the_same_projection_as_the_sidebar`: my
  cross-check follow-up had moved the desktop picker back to `endpoint.live.status`.
  That test (from the handoff, runs on the desktop host) documents the change as
  intended: the selected endpoint's stored projection can lag the window's, and the
  picker must match the sidebar. The reviewer's finding was wrong; the follow-up and
  its contradicting test were reverted, so the picker and footer use
  `displayed_connection_status` as the sidebar does.
- `discover::tests::en_xa::nearby_devices_show_only_messages_under_en_xa`: present
  since the handoff's iPad Add Device layout (not caught before because only focused
  tests ran). The iPad body's `min_w_0()` made taffy measure each truncated suggestion
  row at min-content, so the recording text system saw `m4max.tail1.ts.net` shaped as
  `m` + `…` although the painted row is 606 px wide. Bisected: `min_w_0().w_full()`
  fails, `whitespace_normal()` alone passes, `w_full().whitespace_normal()` passes
  all 18 Add Device tests including the 360 px viewport tests. The body now uses
  `w_full().whitespace_normal()`.
- The first run's `just` gates exited 127: `just` is not on this machine's PATH. Their
  recipe bodies were run directly in `claude-final-gates2.log`.

### After the layout and picker corrections, build of 04:10

- Full `herdr-gpui` host suite: 4702 passed, 0 failed, 4 ignored. Workspace
  `cargo fmt --check`, `git diff --check`, file sizes, keep-sorted and i18n checks
  (with their script tests), `scripts/ios/lint.sh` (both iOS targets), the iOS Python
  script tests, and host clippy for herdr-gpui pass (`claude-final-gates2.log`).
- Add Device on the new body layout (`w_full().whitespace_normal()`): intro, key line
  and fields wrap inside the 640 pt dialog; after tapping the target field the keyboard
  opens, typing works, and the dialog sits above it with the footer visible
  (`claude-l4`, `claude-l5`).
- Behaviour to note (U3 policy): when the XCUITest runner took the foreground, UIKit
  resigned our text view while the scene still reported active, which U3 records as a
  user dismissal (same class as another app's field in Split View taking the
  keyboard). The Add Device field autofocused afterwards without opening the keyboard;
  tapping the field opened it. Before U3, any focus change reopened it.

## Another agent's handoff (setup session, 2026-10-31 snapshot), checked on 2026-11-01

Source: `/private/tmp/claude-501/-Users-shin1ohno-ManagedProjects-setup/31fdb089-678c-4c04-9baf-a94011d72514/scratchpad/herdr-ipados-handoff.md`.
Committed first (7ee913c1..e907fe9c, signed, `scripts/ios/lint.sh` included), then each
claim was re-checked against the tree:

| Claim | Now | Action |
| --- | --- | --- |
| A1 no stderr / RUST_LOG has no effect | True by design (file-only sink, no env filter); sim.sh help and docs/ios.md said RUST_LOG reaches the app | Docs and help corrected, log path named (b9087d67) |
| A2 LOG_TARGETS lacks herdr_uikit/herdr_ssh | Stale: both listed; uikit debug lines appear in the app log | None |
| A3 Quit / cmd-q appear to work on iPad | True (platform quit is a no-op by design) | Hidden from the palette and unbound on iOS (40a2e62b) |
| A4 sim.sh passes on process liveness | True | `--expect-color` / `--min`, exit 5; +control 99.1% #101419 pass, #ff00ff fail (b9087d67) |
| app/ios.rs `%error` loses the chain | True (anyhow) | `{error:#}` (e30b390d) |
| B1 failed texture rebuild never retried at the same size | True | Codex U4: 1 s retry policy, path frames skipped while missing |
| B2 unchecked Metal constructors (queue, buffer, encoders) | True (3 call sites + queue) | Codex U4: checked msg_send wrappers, typed Allocation errors |
| Quit-time stores never flush on iOS (15 `on_app_quit`) | True: iPadOS kills suspended apps without `applicationWillTerminate:`; GPUI exposes no app-level lifecycle hook | Not done: needs a cross-cutting flush design (asked the user) |
| NoScene guard unreachable with UIApplicationSupportsMultipleScenes=true | Not re-checked in depth; low impact | Not done |

## Device install tooling and background persistence (2026-11-01)

- 24c4c478 `feat(ios): sign and install the app on an iPad` (workflow wf_4b8556fe-54f: implement,
  2 reviews, 12 findings fixed). 69 script tests pass. Running provision.sh from Claude's
  session failed with "User interaction is not allowed" (Xcode needs a keychain/account prompt
  in the GUI session), so the device run is handed to the user's own terminal.
- 8f4458d2 `fix(ios): save the app's state when iPadOS sends it to the background` (workflow
  wf_5d510bdd-a3d: implement, 2 reviews, 5 findings fixed incl. the major one, drafts not
  covered by the background task). Gates: UIKit sim suite 266/266 (BackgroundTask and delivery
  tests executed), herdr-gpui 4719/0, host clippy, lint-ios, fmt, file sizes, keep-sorted.
- Simulator e2e: Home at 12:25:33 logged `Saved state for the background {"drafts": true,
  "synced": 9, "lost": 0, "timed_out": 0}` (claude-bg1).

## herdr-uikit git dependency switch (Claude, 2026-11-01 15:53–16:05)
- Codex SWITCH report: 361 app-suite failures were sandbox permission denials; reran outside the sandbox.
- Sim launch of the switched app (sim.sh --expect-color 101419): 97.5% of pixels, screenshot qa/switch-verify/app/screenshot.png.
- fmt, diff --check, file size, sorted, i18n 3954/163: pass. herdr-gpui suite 4923 passed / 0 failed / 4 ignored.
- External herdr-uikit sim tests (rev 00a8dba via ../herdr-uikit manifest): 320/320. scripts/ios/lint.sh exit 0 (both targets, uikit from Cargo git checkout).
- Reverted Codex's fork-only CI guard additions (fork CI never ran; out of scope). Patch kept at qa/switch-ci-guard.patch.
- Committed ed63700c on chore/uikit-path-dep, ff into feat/ipados-foundation, pushed to fork. Docs commit 0da124d3 (Herdr Remote positioning, remote-control rationale, no app lock).

## KEY + QR integration (Claude, 2026-11-01 16:30–17:30)
- Cherry-picked KEY (f755b60b) and QR (11579862) onto feat/ipados-foundation; one conflict (settings_window/tests.rs mod list).
- Gates on 11579862: fmt, file size, sorted, i18n 4001/165, clippy (gpui+credentials all-targets; client test-support), credentials 26/26, client 409/0, herdr-gpui 4968/0.
- iOS lint FAILED: QR used Application::to_async (absent in gpui-pre 0.3.6) and left qr_square unused on iOS (Codex skipped lint.sh for disk). Fixed in d1cad43a (platform.on_open_urls inside run, like watch_lifecycle). lint.sh + bundle pass.
- Simulator: `simctl openurl herdr-ipad://host-key?...` -> system "Open in Herdr?" -> Add Device opens with the warning, host hint, fingerprint. Footer read "Checking…" while pending (misleading) and the warning said "target below" (it is above): fixed in 723f9cb8 with a regression test (fails on the old label, mutation-checked). Accept/Dismiss sit one scroll below the fold (follow-up: move them into the footer while pending).
- Simulator: Settings > General > THIS IPAD shows the line, Copy, Replace key…; first tap shows Cancel/Replace key + red warning; second tap replaced the line (…BBBBAaRyj… -> …BBBBK+KXf…); after terminate+relaunch the new line persists (software key path; Secure Enclave is device-only).
- Noted, unrelated: simulator shows "shared Herdr config path … writable by group or others (mode 0775)" for the container data dir.
- Disk: full (ENOSPC) at ~16:48 for a few minutes; cause = swap files growing under memory pressure on the same APFS container (swap 6-8 GiB). User approved deleting the host build; done via `cargo clean --profile dev` (3.1 GiB). Simulator shut down at 17:28 to relieve memory.
- Pushed 76a86890 to fork. Codex MENUS started on .claude/worktrees/ipad-menus from 76a86890.
