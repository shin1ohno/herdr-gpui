# Completeness critique of design.md (iPad UI 01–08)

Read: HANDOFF.md (all sections), design.md (all 709 lines), research/fractal.md, linux-handoff-prompt.md, push-design-prompt.md.
I spot-checked about 60 of the design's code anchors against the repo at HEAD 694fc552 (docs-only past the design's basis 2a217897), herdr-uikit 00a8dba, and gpui-pre 0.3.6. Abbreviations follow design.md: `src/` = `crates/herdr-gpui/src/`, `G:` = gpui-pre 0.3.6.
Nothing was built. I ran only read-only probes: `xcrun simctl list`, `command -v`, `tmux ls`, `df`.

The most important finding is that the design's new `layout()` predicate fixes the test-gate problem for geometry, but the plan still gates new behaviour on `active()`. `active()` is already true in every headless test. The rest of the most severe findings are four places where the fractal runbook cannot run as written.

## Findings, most severe first

### M1 · major · 07/01 · PR5's `Command::Keybinds` retarget breaks existing tests and native checks
- The design (§2 PR5 "Legend → 07") makes `src/window/commands.rs:515-518` open Settings > Keyboard under `active()`. It also says "On iPad, ⌘/ also opens it."
- `active()` is true in every test unless `desktop()` is held (`src/ipad_ui.rs:14-24`).
- Existing tests type ⌘/ and expect the keybinds page. `src/sidebar/layout_tests/text_width/panels.rs:6-11` reads `menu.keybinds_search` after `cmd-/`, and the same holds at `:27-32`. These would fail, and the design's PR5 "Desktop guards" list does not name them.
- On iPad it also takes away the existing keybinds page from ⌘/, from the palette, and from Herdr's `prefix+?` "help" (`src/keymap/daemon.rs:73`). The 08 legend labels that chord "Help".
- **Fix:** Leave `Command::Keybinds` alone. The iPad legend (`window/status_bar/ipad.rs`, PR2b) calls `settings_window::open_at(Section::Keyboard)` directly, so the change lands with PR5's 08 commit after PR2. If ⌘/ on iPad really should go to Settings, ask the user.

### M2 · major · 07 · Adding `Section::Keyboard` to `Section::ALL` breaks the native Settings check on desktop
- `src/settings_window/native.rs:14-18` sizes a probe table by `Section::ALL.len()`.
- At `:153-165` it iterates every `Section::ALL` entry and fails with "missing Settings category paint" when a row did not paint.
- With the planned runtime filter (`ipad_ui::active()` in `navigation.rs:70-88`, which is false in production desktop), the Keyboard row never paints. `live_gui` then fails in `just test-gui` and in the `windows-native` CI job (AGENTS.md "Verification").
- The design names `section.rs`, `navigation.rs`, `controls.rs`, `catalog.rs` and `ui.rs`, but not `native.rs`.
- **Fix:** Add a `Section::shown()` iterator, filtered the same way, and use it in `navigation.rs`, `nav_item` (`section.rs:178-192`), and `native.rs:153,219,386`. Add `native.rs` to PR5's file list and to its desktop guards.

### M3 · major · fractal / all PRs · Ownership table omits the files that register test modules
- The design creates topic tests whose declaring files appear in no ownership row of §3:
  - `src/window/tests.rs`, which lists `mod` lines at `:11-20` and would receive W0 `keyboard_state`, the 2b en-XA topic, PR4 `key_band`/`compact`, and the PR5 08 test
  - `src/window/tab_strip/tests.rs` (2b `ipad`, PR3 `crumb`)
  - `src/core_terminals/tests.rs` (2b `focus_frame`, PR4 `pager`)
  - `src/menu.rs`, which has no `mod tests;` today, so PR3's `src/menu/tests/row_menu.rs` needs one
  - `src/sidebar/rail/tests.rs` (PR3)
  - possibly `src/i18n/leak_tests.rs`
- NODE.md (§4.4) says "Do not touch any other file", and its completion requirement is that `git diff --name-only ipad_ui...HEAD lists only owned files`. As written, no node can register its own tests and still complete.
- The same gap applies to visibility tweaks a node will need, for example a `server_keys.rs` accessor for PR5's prefix-source label (`src/window/server_keys.rs:97-115`).
- **Fix:** Add a column for "test-module registration (append-only)" files per PR. Put `// keep-sorted` markers on `window/tests.rs`'s mod list in W0, because three PRs append to it. Let NODE.md allow `mod` lines in the parent `tests.rs` of any directory the node writes tests into.

### M4 · major · 08 / fractal · The 08 test commit has no node that can run it
- §2 PR5 makes "the 08 test commit PR5's last step after rebasing on PR2". §4.7 schedules `keyboard` alongside `main_sidebar` (2a) and before `main_chrome` (2b).
- The 08 test needs 2b's tab chips and legend: the tab chip reading `⌃B 1` and the legend in `src/window/tests/herdr_prefix_style.rs`.
- `keyboard` finishes and is merged by `fractal node merge` before 2b exists. The design has no ninth node, no hold on `keyboard`, and no rule for who writes the test.
- **Fix:** Pick one of three:
  1. Move the 08 test and the iPad legend → Settings hookup (M1) into a small `prefix_style` node that starts after `main_chrome` merges.
  2. Schedule `keyboard` after `main_chrome`.
  3. Have Claude write the test during the PR5 export.

  Record the choice in §3 and §4.7.

### M5 · major · fractal export · PR2 and PR6 span two nodes, but the export step takes one merge commit
- §4.6 step 4 transplants `git diff --binary <merge-commit>^ <merge-commit>`.
- PR2 is `main_sidebar` plus `main_chrome`, and PR6 is `sheet` plus `chat`. Under §4.7 they merge into `ipad_ui` non-adjacently (for example 2a, then keyboard, then 2b).
- The design also exports in a different order (1→2→6→5→3→4) than nodes merge into `ipad_ui`. A plain `git apply` of a diff computed against an older `ipad_ui` fails on context lines in shared aggregators (M3).
- **Fix:** Export per PR as the ordered list of its node merge commits, applied with `git apply --3way --index`. Alternatively, compute `git diff <ipad_ui at last export> <ipad_ui now> -- <the PR's owned paths>`.

### M6 · major · fractal test.sh · The templated `cargo test` line takes several positional filters
- The §4.5 template reads `nice -n 15 cargo test --locked -p herdr-gpui <per-node filters, e.g. ipad sidebar:: keymap:: palette::>`.
- Cargo accepts one positional TESTNAME. Further filters must follow `--`. As written, `cargo test` rejects the extra arguments, `test.sh` exits non-zero, and no node meets its completion requirement. [推測: not executed, because cargo runs are forbidden here; based on cargo's CLI shape.]
- **Fix:** Use `cargo test --locked -p herdr-gpui -- ipad sidebar:: keymap:: palette::`, and state that form in NODE.md's focused-run rule too.

### M7 · major · fractal test.sh · The mkdir lock has no stale-lock recovery
- §4.5 loops `until mkdir "$lock"; do sleep 20; done` and relies on `trap 'rmdir "$lock"' EXIT`.
- A codex command timeout, a fractal step timeout (45 min) or a kill sends SIGKILL, which skips the trap. Every later `test.sh`, and the operator's gate (§4.6 step 5 "build lock taken"), then waits forever on the shared `target/shared` dir.
- **Fix:** Write `$$` and a timestamp into the lock and break it when the PID is dead or the lock is older than 60 min. Also bound the wait (for example 30 min, then exit 5 with a message for REPORT.md).

### M8 · major · 04 · The host-key card is gated on `layout()` while the renderer is chosen by `cfg!(ios)`
- `src/menu/devices/add_device.rs:370-374` picks `render_ipad_host_key` only under `cfg!(target_os = "ios")`. The design keeps that ("The cfg! choice at add_device.rs:370 stays"), but drops the footer's Cancel and Trust key under `layout()` and moves them into the card.
- Under the planned test guard (`ipad_layout()`, layout true and cfg false), a host test renders the desktop `render_host_key` with no card and no footer buttons. Trust is then reachable only through the header, and `host_key/tests/card.rs` cannot see the card in context.
- **Fix:** Gate the renderer choice, the card, and the footer drop on one predicate: either `cfg!(target_os = "ios") || ipad_ui::layout()`, or `cfg!` for all three, with a direct-call unit test for the card. State whether the header's primary action on the TrustKey step is "Trust key" or a disabled "Add device", so Trust key does not appear twice.

### m1 · minor · 01 · Existing-code claim: `agent_recency` records an `Instant`
- The design says "`src/state/agent_recency.rs:12-55` records an `Instant` per pane". That is false.
- It stamps a process-wide `AtomicU64` logical counter (`agent_recency.rs:12-14,44-47`) and derives `Eq`. `ClientShellAgent` carries no timestamp either (`crates/herdr-protocol/src/wire.rs:419-434`).
- A node would find no elapsed source.
- **Fix:** State that PR2a adds an `Instant` map beside the counter, stamped in `observe`. The sidebar is a cached view (`render.rs:127-132` `cached_view`), so also add a 60 s refresh while the iPad layout shows an elapsed value. Without one, "2m" never ticks. The design has no timer.

### m2 · minor · 02/06 test plan · `TestWindow::simulate_insets_change` is not reachable from herdr-gpui
- `simulate_insets_change` exists (`G:src/platform/test/window.rs:182`), but the only accessor is `pub(crate) fn test_window` (`G:src/app/test_context.rs:615`).
- The reachable equivalent is `cx.simulate_window_visual_viewport_change(handle, bounds)` (`G:src/app/test_context.rs:471-478`). Existing tests already use it (`src/window/tests/visible_viewport.rs:13`), and `fully_visible_bounds` intersects the visual viewport (`G:src/window.rs:2787-2799`).
- **Fix:** Rewrite the planned `keyboard_state.rs`, `key_band.rs` and `compact.rs` tests to shrink the visual viewport.

### m3 · minor · 01 · The per-tab ⌘N hint is placed on `TabContext`, which is per strip
- `TabContext` is "Read-only inputs every tab of one strip shares" (`src/tab_layout.rs:184-193`). The per-tab content is `TabFace` (`:163-173`).
- **Fix:** Put `hint: Option<SharedString>` on `TabFace` (or `Mark`), and keep only the style and the hardware flag on `TabContext`.

### m4 · minor · 01/02/06 · Hardware-keyboard evidence: the compact key row counts as hardware
- In compact (06) the 56pt key row shows without the soft keyboard and dispatches through `window.dispatch_keystroke`, which the interceptor sees.
- `!soft_keyboard_shown` is then true, so `hardware_seen` flips on and ⌘ hints appear on a touch-only iPad. §1.6's argument ("key-band keys are dispatched only while the soft keyboard shows") does not hold for 06.
- §1.6 also claims "one sorted field `ipad`". The observer needs a `Subscription`, and `IpadState: Default` cannot hold one. So either `HerdrWindow` gains a second field (construct.rs:127-317 and `layout_tests.rs:592-812`), or the hook goes into the existing interceptor (`src/window/prefix.rs:35-60`, a PR5 file).
- **Fix:** Mark synthetic keystrokes (a flag set around the band's and key row's dispatch) and skip them. Name the observer's home explicitly.

### m5 · minor · 01–08 (§2) · No inventory of hover-only controls
- HANDOFF §2 asks to replace hover-dependent operations with a long press or "•••". The design covers only rail rows (PR3).
- Tab close buttons are hover-revealed in several layouts (`src/tab_layout/pills.rs:98-107`, `outline.rs:124`, `cards.rs:108`, `curved.rs:122`). In total there are 22 `group_hover`/hover sites in sidebar and tab code.
- The new `tab_layout/ipad.rs` does not say that ✕ is always visible.
- **Fix:** Add a §2 audit table (site → iPad replacement) to PR2 and require ✕ visible on every tab, or at least on the selected tab, under `layout()`.

### m6 · minor · 01/03 · Status dots keep the hard-coded Catppuccin hex values
- `status_style` hard-codes `0xf9e2af/0xf38ba8/0x94e2d5/0xa6e3a1/0x6c7086` (`src/sidebar/agents.rs:593-601`) unless Herdr settings load.
- On iPad under Nord, the working dot (f9e2af) then sits next to the nord13 Needs-you badge (`palette[3]` = ebcb8b). That contradicts HANDOFF §2 ("hex をハードコードしない", nord13 for 作業中).
- The design also asserts that Idle is a green hollow ring and Done is teal (true in code), against the handoff's 待機 = 水色 and 完了 = 緑の輪. It does not ask.
- **Fix:** Add a Q: either theme-derived dot colours under `layout()` (`palette[3]/[6]/[2]/[1]`), or keep Mocha. Put the status-shape mismatch with the mock into that same Q.

### m7 · minor · 07 · Config toggle extraction risks blocking I/O on the UI thread
- `Config::toggle_keybinding_source(&endpoint)` "so the host menu and Settings call one function".
- The current toggle hands the file write to `save_preference` (`src/menu/devices/host_menu.rs:262-279`). `save_device_keybindings` takes a file lock and writes (`src/config/devices.rs:118-121`).
- If the extracted function writes, Settings will call it on the UI thread (AGENTS.md "GPUI Rules").
- **Fix:** Extract only the pure next-source computation, and route both callers through `save_preference`.

### m8 · minor · 07 · Settings opens as a separate UIScene; the alternative was not evaluated
- Settings is `cx.open_window` (`src/settings_window.rs:131-139`). On iPadOS, herdr-uikit turns a second window into a scene activation (`herdr-uikit src/window/scene.rs:212,251`). So the legend in 01 jumps to another window rather than a sheet over the main view.
- The design marks the 1000×760 sheet "not feasible" without costing an in-window sheet under `layout()`.
- **Fix:** Add a Q with costs: keep the scene, or present the Keyboard page as an in-window sheet on iPad.

### m9 · minor · 06 · `window/render.rs` will likely cross 1,000 lines
- The file is at 987 (`wc -l`). PR4 adds the band mount, the compact top-bar swap (`render.rs:936-939`), and the sidebar overlay with its scrim. After `cargo fmt`, each `.when(..)` costs about 3 lines, so the total reaches about 996 or more. The design says "~988".
- **Fix:** Have PR4 add a single `.children(self.ipad_layers(window, cx))` call in `render.rs` and put everything in `window/compact.rs` and `ipad_keys/band.rs`. Pre-authorize a `render.rs` split in PR4's file list.

### m10 · minor · fractal launch · `GITHUB_TOKEN` reaches the nodes, and so does `gh`
- `GITHUB_TOKEN` is exported in this shell (probe), and nodes run `danger-full-access` (fractal.md §4).
- A tmux server cold-started from Claude's Bash inherits the token. `build.rs` also calls `gh` on every worktree build (`crates/herdr-gpui/build.rs:80-88`).
- The research recommended `env -u GITHUB_TOKEN` (fractal.md §7). The design dropped it.
- **Fix:** Start the first `fractal node start` with `env -u GITHUB_TOKEN`. Optionally set `HERDR_BUILD_PR_NUMBER` to a valid dummy, or document the 2 s `gh` timeout.

### m11 · minor · fractal root · Clone-side mutations sit outside the session's working directory
- `git clone … herdr-gpui-ipad`, `switch -c`, `config`, `cp`, `fractal node merge`, and the step 9 `merge` all write `/Users/shin1ohno/ManagedProjects/herdr-gpui-ipad`.
- The design marks only `fractal init` and `node start` as sandbox-disabled. The `cp HANDOFF.md design.md …` line uses relative sources.
- design.md also cites `linux-handoff-prompt.md` and `research/*.md`, which are not copied for the nodes.
- **Fix:** Mark every clone-side step as sandbox-disabled or user-run, use absolute source paths, and copy the cited research files, or strip those references from the node copy.

### m12 · minor · 03/06 · PR3 and PR4 run in parallel but serialize on two files
- Both "append" to `window/ipad_state.rs` and `window/keyboard_state.rs` (§3 table), and those regions currently hold one line.
- PR4's `tab_strip/ipad.rs` line lands "after PR3" inside the same wave. In fractal that line arrives only through a later PREPARE merge of `ipad_ui`, which the design does not mention.
- **Fix:** Name PR3's field, or drop PR3 from those rows. State that `touch` starts after `rail` merges, or that its compact line waits for PREPARE.

### m13 · minor · §5 / 05 · HANDOFF §5's "PR 本文に明記する" is not carried into the PR body rules
- HANDOFF §5 says each open item proceeds with its default and is stated in the PR body: 05's sidebar is two-line, auto single-pane, and the space-row order.
- §4.6 step 8 lists only HANDOFF §6 items. Item 2 also deviates from the default: a one-finger pager instead of a two-finger swipe, and no auto-zoom on Herdr 1 (Q6).
- The design's own new defaults also change iPad behaviour silently: iPad style on iOS (`ShortcutStyle::default`), Bubbles chat, and the activity bar off.
- **Fix:** Add "HANDOFF §5 items and design defaults that apply, with [推測]" to the PR body checklist.

### m14 · minor · §4 · Merge order and W0 scope deviate from HANDOFF §4 without a question
- HANDOFF §4 orders PRs 1→6 and limits PR1 to constants, theme roles, and the Nord check.
- The design merges 1→2→6→5→3→4 (§3) and moves palette chips, `KeyboardState`, `status_dot`, and `of_agent` into W0 (Q2). It also keeps most size constants PR-private (§1.2), where the handoff wanted one place for 44/48/52/56/72/300.
- Q2 covers scope, but no Q covers the order.
- **Fix:** Add the merge order to Q2, or to a new Q.

### m15 · minor · §1 / review · The mock sources are not wired into the nodes or the review
- `fork/design/ipad-ui` does not exist yet (`git branch -r`; push requested in `push-design-prompt.md`).
- The NODE.md template (§4.4) and the review (§4.6) never point at `design/ipad/*.png` or `.dc.html` once it lands, so nodes build from text alone and screenshots are never compared to the mocks.
- **Fix:** Add "if `fork/design/ipad-ui` exists, read `git show fork/design/ipad-ui:design/ipad/<NN>-*.png`" to NODE.md, and add a mock-versus-after comparison to step 7.

### m16 · minor · screenshots · `qa.sh` path is missing, and the light/dark rule is not followed
- §2 setup step 3 calls `qa.sh` bare. It lives only in this session's scratchpad (`…/scratchpad/qa/driver/`, foundation.md §6), so the path is volatile.
- AGENTS.md "Git And Handoff" requires light and dark variants when colours change (PR2 recolours the online dot and the badges), and a narrow window when layout changes.
- **Fix:** Give the absolute `qa.sh` path. Plan a light-theme capture (for example `Catppuccin Latte`) beside Nord for PR2, PR3 and PR6.

### m17 · minor · 07 · PR5 edits existing Settings tests, which HANDOFF §6's self-check forbids
- Because `active()` is true in tests, adding Keyboard forces edits to `settings_window/tests/{search,section_order,navigation_scroll}.rs` (design §2 PR5).
- HANDOFF §6 asks to confirm that desktop tests did not change.
- **Fix:** Have new tests cover the iPad list, and wrap the existing assertions in `desktop()` instead of editing their expectations. If edits are unavoidable, list each one in the PR body as an iPad-side expectation change.

## Coverage map (HANDOFF → design)

| HANDOFF item | Covered? | Note |
| --- | --- | --- |
| §2 44pt, hover → long press/••• | partial | m5 |
| §2 colours from theme, Nord | yes (Q4) | dots stay Mocha hex: m6 |
| §2 status dot meanings, icons, shortcut tables | yes | iOS uses the macOS table: `crates/herdr-commands/src/shortcuts.rs:12-19` (verified). All 12 spec rows resolve as written except Sidebar `⌃B b` (`keymap/daemon.rs:256-260`, Q12) |
| 01 | yes (18 ms Q5, Herdr 1 frame infeasible) | M1, m1, m3, m4, m5 |
| 02 | yes (Q6 deviates) | m2, m4, m13 |
| 03 | yes | m12 |
| 04 | yes | M8 |
| 05 | yes (tab title becomes `Agent: prompt`, noted) | m13 |
| 06 | yes [推測 compact threshold] | m4, m9 |
| 07 | yes (sheet → window, Q7 prefix source) | M1, M2, m7, m8, m17 |
| 08 | yes in design | M4: no node can land its tests |
| §4 PR list and gates | yes (no `just`, gates via recipe bodies; Q17 disk) | M5, m14, m16 |
| §5 open items | defaults chosen | m13 |
| §6 report format | yes | m13, m17 |

Claims verified true (sample): `active()` test gate (`ipad_ui.rs:14-24`); 232pt desktop sidebar (`sidebar/metrics.rs:7,35-41`); built-in Nord with a purple `primary()` (`config/theme.rs:452-456,553-560`); Idle hollow and Done filled (`sidebar/agents.rs:405-410`); `server_keybindings_toml` on the Herdr 1 snapshot (`herdr-protocol/src/wire.rs:319`); single-touch recognizer (`G:src/gestures.rs:478-489`); herdr-acp `agent = "acp"` (`herdr-acp/src/herdr.rs:23-24`); two `HerdrWindow` literals (`window/construct.rs:126`, `sidebar/layout_tests.rs:592`); `config.rs` at 986 lines, `add_device.rs` 986, `host_menu.rs` 916, `rail.rs` 924; `test-ios` runs only herdr-uikit's tests, so running it from the main checkout is correct (`justfile:448-454`); `lint.sh` honours `CARGO_TARGET_DIR` (`scripts/ios/lint.sh:22-24`); `bundle.sh` passes no `--target-dir`; 11-inch M5 simulator `ED30D4E4…` present.
