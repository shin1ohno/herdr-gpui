# iPad UI research: tab strip, panes, status bar (screens 01, 03 crumb, 08)

Scope: herdr-gpui `feat/ipados-foundation` @ 76a86890, herdr-uikit @ 00a8dba (local clone HEAD = pinned rev, `Cargo.toml:104`).
Paths below are relative to `/Users/shin1ohno/ManagedProjects/herdr-gpui/crates/herdr-gpui/src/` unless prefixed.
`U:` = `/Users/shin1ohno/ManagedProjects/herdr-uikit/`. `G:` = `~/.cargo/registry/src/index.crates.io-*/gpui-pre-0.3.6/`.

## 0. Cross-cutting facts that shape every change

- `ipad_ui::active()` returns **true in every `#[cfg(test)]` build** unless the test holds `ipad_ui::desktop()` (`ipad_ui.rs:14-24`, `27-42`). Only 3 test files use `desktop()` today (`settings_window/tests/ipad_key.rs`, `sidebar/rail/tests/touch_context.rs`, `core_terminals/tests/reconnect.rs`). Any geometry gated on `active()` (tab height, close size, status bar height) therefore changes what existing headless desktop tests see (e.g. `window/tab_strip/tests.rs:52-62` asserts close/mark bounds; `window/status_bar_tests.rs` measures `connection-status` and `terminal` bounds). Plan: gate with `active()` and add `let _desktop = crate::ipad_ui::desktop();` to desktop-geometry tests that break, or gate size constants with `cfg(target_os = "ios")` instead.
- No iPad size constants exist yet. Desktop geometry lives in `constants.rs:13` (`TAB_WIDTH = 64`) and `constants.rs:15` (`TAB_HEIGHT = 24`), `titlebar.rs:34` (`HEIGHT = 34`), `window/status_bar.rs:26-28` (`status_bar_height = max(ui.size*1.5+4, 22)`).
- Default tab layout is `Classic` (`config/tab_layout.rs:53-54`, `#[default]`); layouts are chosen via `tab_layout::layout_for` (`tab_layout.rs:341-351`).
- Two pane systems: Herdr 1 panes are cells the daemon draws into one surface (`herdr-protocol/src/wire.rs:437-450` `PaneSurfacePane { rect, inner_rect, focused, .. }`, painted by `window/render.rs:532`/`607` `paint_frame`); Herdr 2 (herdr-core) panes are drawn by the client (`core_terminals/split.rs:43-119`). The user's main iPad target is Herdr 2 (spec: "SSH · Herdr 2"), and on iOS the connection UI follows the core backend (`window/connection_status.rs:11-16`).

## Screen 01 — tab strip

### What exists

- Strip builder: `HerdrWindow::render_tab_strip` (`window/tab_strip.rs:136-476`). Order left→right: leading header controls when the strip stands in for the titlebar (`:412-415` → `titlebar/tabs.rs:152-203` `strip_leading`), launcher leading spot (`:416`), scroller with tabs (`:357-402`, `:420`), "+" new tab (`:421-454`), drag room (`:455` → `titlebar/tabs.rs:256-273`), launcher trailing (`:456`), **split** button (`:457-463`, id `split-editor`), **•••** button (`:464-472`, id `tab-actions` → `open_group_menu`), titlebar trailing (`:473-475`).
- Strip height: `tab_strip_height()` (`window/tab_strip.rs:127-132`) = `max(tabs.size*1.6+4, TAB_HEIGHT)`, then `titlebar::strip_height` raises it to 34 when merged into the header (`titlebar/tabs.rs:278-280`). The tab list takes it at `window/tab_strip.rs:236`; the sidebar header row matches it (`titlebar/tabs.rs:103`).
- Tab cell (Classic): `tab_layout/classic.rs:78-119`; width formula `classic.rs:50-65` must agree with the drawn tab (contract stated at `tab_layout.rs:226-229`). `min_w(TAB_WIDTH)` at `classic.rs:98`.
- Close button: `tab_layout::close_button` (`tab_layout.rs:454-488`), size passed by the layout; Classic passes `CLOSE = 18.` (`classic.rs:23`, used `classic.rs:118`); icon 12px (`tab_layout.rs:479-480`). Hit area is the whole `size` square (div with `on_click`, `tab_layout.rs:484-487`). Spec wants 40×40.
- Per-tab activity dot: Herdr 1 tabs get `Mark::Herdr { status, agent }` from `herdr_mark` (`window/tab_strip.rs:916-930`); `lead_mark` draws `status_indicator` before the title (`tab_layout.rs:355-368`, Classic `classic.rs:113`). `AgentStatus::Unknown` draws nothing (`tab_layout.rs:83-92`). Dot geometry: `STATUS_WIDTH = 8.` (`sidebar/metrics.rs:14`); **Working is a spinning orbit SVG, not a filled dot** (`sidebar/agents.rs:403-427`, `446-457`); colors come from `Indicators::color` (`sidebar/agents.rs:318-327`).
- **Herdr 2 tabs have no status dot.** Core terminal tabs are browser-store tabs (`Location::CoreTerminal`) built in `browser/view/render.rs:29-90` with `Mark::Icon("icons/terminal.svg")` (`:55`, `:60-63`) → `Lead::Icon`, no status. Per-pane status exists: `core_pane_label` returns `(title, status)` (`core_terminals/split.rs:275`, used `:258`), indicators from `core_indicators` (`core_terminals/split.rs:306`).
- Split and ••• buttons: `strip_button` (`window/tab_strip.rs:608-641`) is `w(30)` × `min_h(TAB_HEIGHT)` with a 14px icon. Note: the split button calls `split_group` (`:462`), i.e. the **editor-group split** (`Command::SplitEditor`, `cmd-\`, `herdr-commands/src/shortcuts/macos.rs:62`), not Herdr's pane split (`SplitRight`, `cmd-d`, `macos.rs:9`). The spec legend "⌘D Split" names the pane split.
- "+" button: Classic default `new_tab` width 34 (`tab_layout.rs:254-272`).
- Overflow: tabs scroll horizontally (`window/tab_strip.rs:239-240`) with a 4px thumb along the foot (`:33`, `:381-402`); "•••" is the group menu, not a tab overflow list.

### Where ⌘1–9 hints could render and how labels resolve

- Tab-number semantics: `Command::TabNumber(n)` indexes `workspace_tabs` (Herdr + page tabs of the focused workspace, hidden ones filtered) via `stepped` (`browser/groups_view.rs:32-47`, `317-323`, `432-457`; dispatch `window/commands.rs:392-396`). A hint's number must come from that list's index, not from the strip's per-group enumeration (a split group lists only its own tabs, `window/tab_strip.rs:199`, `246`).
- Catalog defaults: `CommandInfo::default_shortcuts` (`herdr-commands/src/shortcuts.rs:11-20`) — iOS falls to the `else` branch = macOS table; `TabNumber(1..9) => "cmd-1".."cmd-9"` (`shortcuts/macos.rs:30-38`), `Workspace` `cmd-shift-n` (`:8`), `WorkspacePicker` `cmd-p` (`:53`), `ToggleSidebar` `cmd-b` (`:39`), `Palette` `cmd-shift-p` (`:54`), `Keybinds` `cmd-/` (`:50`).
- Resolved per window: `HerdrWindow::keymap()` (`window/server_keys.rs:74-76`) → server keymap if "Use server keybindings" else local. `Keymap::shortcuts(command)` yields labels, primary first (`keymap.rs:311-319`); `primary` (`keymap.rs:322-324`). Layering in `Keymap::layer` (`keymap.rs:219-294`): catalog direct defaults first, then daemon prefix chords labeled `"<prefix> <key>"` e.g. `ctrl-b c` (`keymap.rs:255-266`). Each `Shortcut` knows whether it is a chord (`keymap.rs:104-115`, field `chord`), but the field is private and there is **no accessor that picks "direct only" vs "chord only"** — 08 needs one (e.g. `Keymap::shortcut_for(command, Style)`).
- Display formatting: labels are raw GPUI strings (`cmd-shift-p`). The only formatter is `menu/settings.rs:706-726` `keycaps` → words ("Cmd", "Shift", "P"). GPUI's `Display for Keystroke` emits ⌘/⇧/⌃ **only under `cfg(target_os = "macos")`** (`G:src/platform/keystroke.rs:683-716`); on iOS `cmd` prints nothing and ctrl/alt/shift print `ctrl-`/`alt-`/`shift-` (same lines). So a herdr-gpui-owned glyph formatter (⌘ ⇧ ⌃ ⌥ ↩ [ ]) is required for every kbd chip (tabs, SPACES `⌘P`, New space `⌘⇧N`, status legend, 07 table).
- Placement: inside the Classic `tab()` row next to the title (`classic.rs:108-117`), passed in through `TabFace`/`TabContext` (`tab_layout.rs:165-195`); the width formula (`classic.rs:50-65`) must add the chip width so measuring (`window/tab_strip.rs:78-102`, `200-212`) stays consistent.
- Gate: "physical keyboard connected" — see the hardware keyboard section; no signal exists today.

### Gaps (01 strip)

1. Tab strip height 52 on iPad (today 24/34): `tab_strip_height` + `titlebar::strip_height` + `sidebar_header` row.
2. Close hit area 40×40 (today 18): Classic `CLOSE` and `width()`.
3. Activity dot 8px for working tabs: Herdr 1 has a status mark (Working = orbit); Herdr 2 core tabs have none — needs a status source from core panes.
4. Split and ••• 52pt each (today 30 wide); confirm split = pane split vs group split.
5. kbd chip ⌘1…⌘9 per tab: needs glyph formatter + workspace index + hardware-keyboard gate.

## Screen 01 — panes

- Herdr 2 split: `render_core_terminal_tab` (`core_terminals/split.rs:43-119`), gutter from `PaneChrome::gutter()` (Classic `1.`, `tab_layout/panes/classic.rs:13-15`), filled with `gutter_fill = theme.active` (`tab_layout/panes.rs:80-82`, used `core_terminals/split.rs:106`, `170-174`); drag grip 6px over it (`core_terminals/split.rs:21`, `176-196`). **Divider is already 1px** in Classic.
- Herdr 2 focus: active pane = header washed with `primary_wash` (`tab_layout/panes/classic.rs:21-41`, 22px header `:7`), inactive panes get a veil `0x50` (`classic.rs:17-19`; applied via `PaneLook.veil` `core_terminals/split.rs:249-254`). **No 2px accent inner frame exists.** Single-pane tabs draw no chrome (`core_terminals/split.rs:55-61`).
- Herdr 1: the daemon's frame carries pane borders as cells [推測: `rect` vs `inner_rect` differ by the border, `wire.rs:440-441`]; the client paints the frame and hit-tests `rect`/`inner_rect` (`terminal.rs:138-148`, `388-405`) and knows `pane.focused` (`wire.rs:444`), but draws no focus overlay (no client border code found; grep of `.focused` usages shows none in `terminal.rs`/`terminal_painter*`). A 2px frame for Herdr 1 would be a new overlay painted from `rect × cell metrics`, same geometry as hit testing.
- Editor-group dividers (between client groups, not panes): 1px line, 5px grip (`window/tab_strip/dividers.rs:13-17`).

Gap: 2px accent inner frame on the focused pane — Herdr 2 via `PaneChrome::pane` or an overlay in `core_leaf` (`core_terminals/split.rs:238-273`); Herdr 1 via a new overlay. `theme.primary()` is the accent (`config/theme.rs:452`).

## Screen 01 — status bar

### What exists

- `render_status_bar` (`window/status_bar.rs:30-194`), placed in the content column under the groups (`window/render.rs:969` via `zen_status_bar`, `window/zen.rs:172-181`). Height `status_bar_height` (`status_bar.rs:26-28`) — spec wants 44.
- Item order today: focused agent model (`:61`, `:199-235`), plan usage (`:62`; disabled on iOS, `window/monitors.rs:15`), **prefix-armed chip** showing `keymap().prefix_label()` (`:63-78`; label from `keymap.rs:341-343`), resize mode (`:80-96`), **connection dot only when NOT connected** (`:97-99`, `269-293`), connection message only when not connected / error (`:31-35`, `:100-116`), mod items (`:117`), listening ports (`:118`), system load / GPU (`:119-120`; host-less on iOS, `window/monitors.rs:85-87`, `174-176`), network quality (`:121`), keep-awake cup (`:122`, `325-423`, iOS note `:329`), **theme** (`:123-135` → `open_theme_picker`), **shortcuts** (`:136-148`, id `status-keybinds`, label `status-bar-shortcuts` = "Shortcuts" `locales/en-US/status-bar.ftl:18`, opens `open_keybinds` = in-window Keybinds menu page `menu/settings.rs:98-118`), **report issue** (`:149-159` → `window/report_issue.rs:22-32`, opens a URL), version / update (`:160-193`).
- Item configuration: `[status_bar]` (`config/status_bar.rs:72-102`): `show`, `usage`, `system_load`, `gpu_detail`, `keep_awake`, `keep_awake_scope`, `theme`/`shortcuts`/`report_issue` as `Button::{Label, Icon, Hidden}` (`:37-61`); Settings window writes via `Edit` (`:104-130`) from `settings_window/pages/status_bar.rs:50`. No item list/order config; the bar is fixed code.
- iPad-specific today: only the connection text on the LOCAL endpoint says "Choose a computer…" (`window/connection_status.rs:28-33`, `status-bar.ftl:38`), and the cup's system scope is unavailable (`status_bar.rs:329`). Nothing else differs.
- Connection data available: host label `Endpoint::label` (`endpoint.rs:139`), backend generation `Endpoint::backend_generation()` (`endpoint.rs:182-184`), status `displayed_connection_status` (`window/connection_status.rs:18-26`). **No host round-trip latency** exists: `network_quality` times TCP to 1.1.1.1:443, not the selected host (`network_quality.rs:1-6`, `25-28`), rendered as `"{ms}ms"` (`network_quality/render.rs:18-36`). Memory note "no health pings" in the iPad tunnel matches [推測 from memory file, not re-probed].

### Gaps (01 status bar)

1. Left "● host · 18 ms · Herdr 2" always shown on iPad (today only a dot when disconnected): new item; latency needs a new per-host RTT probe (SSH channel round trip) — no source.
2. "⌨ Hardware keyboard" item: needs the hardware keyboard signal (none).
3. Legend button reusing `status-bar-shortcuts`: today an icon+"Shortcuts" label opening the Keybinds menu page; spec wants 4 chords (`⌘T Tab · ⌘D Split · ⌘B Sidebar · ⌘⇧P Commands`) resolved from the keymap and opening 07 (settings Keyboard section). Settings window `open(source, cx)` (`settings_window.rs:79`) has no target-section argument and `Section` has no Keyboard (`settings_window/section.rs:8-25`).
4. Right-end 44pt "show software keyboard" button: request path exists (below); no button.
5. Hiding desktop-only items on iPad (theme? report issue? version?) is not decided in the spec [推測: spec lists only the four items].

## Hardware keyboard signal and software keyboard path

- herdr-gpui: no hardware-keyboard notion (negative search for `hardware.keyboard|hardware_keyboard|GCKeyboard` in `src/` = 0 hits; positive control `request_virtual_keyboard` = 4 hits).
- herdr-uikit: no GameController dependency or `GCKeyboard` (0 hits in `U:src` and `U:Cargo.toml`; positive control `UIPress` = 15 hits). Keyboard state is window-private: `Keyboard { dismissal, minimized, visibility, .. }` (`U:src/window/keyboard.rs:74-86`) fed by show/hide notifications (`U:src/window/keyboard.rs:174-208`, `U:src/window/lifecycle.rs:389-406`). The public crate API is only `BackgroundTask`, `Error/Result`, layout direction, `IosPlatform`, `application()` (`U:src/lib.rs:34-43`).
- Indirect signals available today: (a) hardware key presses arrive as `UIPress` with a `UIKey` in `presses_began` (`U:src/window/text_input/keys.rs:160-205`) — a "seen a hardware key" flag could be raised there [推測: software keyboard taps arrive via `insertText:` instead, `U:src/window/text_input/editing.rs:17`]; (b) keyboard occlusion: a hardware keyboard leaves only the ~55pt shortcuts bar (`U:src/window/tests.rs:126-130`, `U:src/window/layout.rs:205-207`), reported into `insets.ime.bottom` (`U:src/window/layout.rs:176-180`), but GPUI exposes only `fully_visible_bounds()` to the app (`G:src/window.rs:2783-2799`; herdr-gpui pads by it in `window/viewport.rs:6-24`) and only while a text responder is active — not a reliable "connected" signal.
- Proper signal: GameController `GCKeyboard.coalescedKeyboard` + `GCKeyboardDidConnect/DidDisconnect` notifications in herdr-uikit, exported as a public query + change callback, then a herdr-gpui rev bump (`Cargo.toml:104`). Cross-repo change (herdr-uikit is a separate repo, `docs/ios.md:10-19`).
- Software keyboard request: GPUI `Window::request_virtual_keyboard` → `PlatformWindow::show_soft_keyboard` (`G:src/window.rs:2805-2807`) → `IosWindow::show_soft_keyboard` (`U:src/window/platform_window.rs:249-255`) → `show_keyboard` clears dismissal and defers `update_first_responder` (`U:src/window/lifecycle.rs:376-386`), which shows the keyboard only if GPUI's focused element takes text (`U:src/window/lifecycle.rs:413-443`, `text_input_focused` set from `TextInputStateChange::FocusGained` `U:src/window/platform_window.rs:261-265`). herdr-gpui callers: `HerdrWindow::request_terminal_keyboard` (`input.rs:57-69`, gate `terminal_accepts_text_input`), terminal mouse down (`window/render.rs:388-406`, `window/mouse.rs:269`), core terminal (`core_terminals/view/pointer.rs:76-79`), text fields (`search_input.rs:697`, `dialog_input.rs:325`). A status-bar keyboard button should `window.focus(&self.focus)` then call `request_terminal_keyboard` in the same click handler, as `window/render.rs:403-406` does [推測: focus→FocusGained may land a frame later; `update_first_responder` is deferred, `U:src/window/lifecycle.rs:448-460`, so ordering needs a device check].

## Screen 03 — crumb at the left of the tab strip

- Natural slot: the leftmost strip's leading part, already used for the sidebar toggle and Back/Forward when the strip stands in for the header (`window/tab_strip.rs:412-415` → `titlebar/tabs.rs:152-203`); it is the only per-window "leading" hook and has shrink rules (`titlebar/tabs.rs:20-24`, `167-169`). It renders only when `ends.leading` and the strip is merged (`titlebar/tabs.rs:54-58`: Herdr `tab_bar_position == Top`, strip not hidden, not RTL). When not merged, the leading slot is the launcher's `Spot::Leading` (`window/tab_strip/launcher.rs:27-33`) — a crumb would need its own leading child before `.children(self.strip_launcher(.., Leading, ..))` (`window/tab_strip.rs:416`).
- Data: current space = `snapshot.focused_workspace_id` (used `window/tab_strip.rs:175`); label via `sidebar::workspaces::workspace_label` (`sidebar/workspaces.rs:157`); status for the dot via `Indicators` (`sidebar/agents.rs:318`).
- Switch action: `Command::WorkspacePicker` opens the palette with `Filter::Navigation` (`window/commands.rs:459-469`); `⌘P` default (`shortcuts/macos.rs:53`). The crumb's click can call `this.command(Command::WorkspacePicker, window, cx)` as the sidebar toggle does with `ToggleSidebar` (`titlebar.rs:310-313`).
- When to show: spec shows it with the 72pt rail (03). Rail state: `SidebarMode::{Expanded, Rail, Hidden}` (`sidebar/rail.rs:55-59`), `sidebar_mode()` (`sidebar/rail.rs:77-82`). Status bar's `⌘B Expand sidebar` would key off `sidebar_mode() == Rail` and `Command::ToggleSidebar`.
- Gap: no crumb element; 44pt+ hit target; must not regress the macOS traffic-light clearance (`LEADING`, `titlebar.rs:550-555`) — on iOS `LEADING = MARGIN = 8` (`titlebar.rs:550-557`).

## Screen 08 — Herdr prefix style

- Prefix source: `DaemonKeys::from_profile` reads the host's published `[keys]` (`keymap/daemon.rs:302-317`), default `ctrl+b` (`keymap/daemon.rs:21-22`); first prefix is the displayed one (`keymap/daemon.rs:278-280`, `keymap.rs:341-343`).
- Chord defaults: `new_tab prefix+c` (`keymap/daemon.rs:139`), `next_tab prefix+n` (`:145`), `switch_tab prefix+1..9` (`:157`), `copy_mode prefix+[` (`:175`), `split_vertical` → SplitRight `prefix+v` (`:226-230`), `split_horizontal` → SplitDown `prefix+minus` (`:231-235`), `close_pane prefix+x` (`:236-240`), `zoom prefix+z` (`:242`), `help` → Keybinds `prefix+?` (`:73`), `settings prefix+s` (`:74`), `toggle_sidebar prefix+b` (`:256-260`); list at `keymap/daemon.rs:71-261`.
- Chord labels reach the keymap only as `"<prefix> <key>"` strings (`keymap.rs:255-266`, label built at `:263`), and only for commands the GUI config does not override (`keymap.rs:239-241`). `Keymap::layer` starts at `keymap.rs:219`.
- Gaps: (1) per-style shortcut selection accessor on `Keymap` (direct vs chord); (2) glyph formatter for `ctrl-b c` → `⌃B c`; (3) the style setting itself (07, another stream); (4) the prefix shown must follow the selected host's profile (`window/server_keys.rs:74-82`) — note that when server keybindings are off, the local daemon's keys apply, which on iPad is the LOCAL endpoint [推測: iPad has no local daemon, so the "From studio-mac's Herdr config" prefix needs the server profile even when "Use server keybindings" is off].

## Risks and Unknowns

- R1 Test side-effects: `ipad_ui::active()` is true in tests (`ipad_ui.rs:14-24`); geometry gates alter existing desktop tests unless they take `desktop()`. Decide `active()` vs `cfg(target_os="ios")` per constant.
- R2 iOS keystroke display: GPUI `Display for Keystroke` drops ⌘ on iOS (`G:src/platform/keystroke.rs:698-707`); any reuse of GPUI's Display yields wrong chips.
- R3 Hardware keyboard detection is a herdr-uikit change (new `objc2-game-controller` dependency [推測: crate name], public API, rev bump in `Cargo.toml:104`) — cross-repo; heuristics from `ime` occlusion are unreliable.
- R4 Host latency ("18 ms") has no data source; `network_quality` measures internet latency (`network_quality.rs:25-28`).
- R5 Split button semantics: strip "split" = editor-group split (`window/tab_strip.rs:462`), spec legend "⌘D Split" = pane split. Needs a decision.
- R6 Herdr 2 tab activity: core tabs carry no status (`browser/view/render.rs:55`, `60-63`); deriving one from core pane statuses is new logic.
- R7 Herdr 1 focus frame must use the same cell geometry as hit testing (`terminal.rs:138-148`) per AGENTS.md geometry rule; Herdr 1 already draws daemon borders, so a 2px overlay may double up [推測].
- R8 File size: `window/tab_strip.rs` is 933 lines and `window/render.rs` 987 lines (limit 1,000); additions must go to new modules (e.g. `window/tab_strip/ipad.rs`, `window/status_bar/ipad.rs`). `window/status_bar.rs` is 535, `tab_layout.rs` 491, `titlebar/tabs.rs` 283.
- R9 Legend → 07: settings window cannot open a given section today (`settings_window.rs:79`); 07 Keyboard section is another stream's work.
- U1 Whether theme / report issue / version / cup / network items stay on the iPad bar is not specified.
- U2 Whether the 52pt strip height applies when the strip is merged into the header (34pt today) or replaces the header on iPad — `tabs_in_titlebar` depends on Herdr's shared `tab_bar_position` (`titlebar/tabs.rs:54-58`).
- U3 Native verification (glyph chips, 40×40 hit test with touch, keyboard button focus order) needs simulator/device; not run here.
