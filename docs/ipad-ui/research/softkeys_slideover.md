# Soft keys, touch, and compact widths — screens 02 and 06

Scope: HANDOFF.md §3 screen 02 (handheld + software keyboard: 76pt key band, 1-pane mode, paging) and screen 06 (Slide Over 320pt: 52pt top bar, 36pt pager, 56pt key row, sidebar as full overlay). Repos read: `herdr-gpui` @ 76a86890 (`feat/ipados-foundation`), `herdr-uikit` @ 00a8dba (= pinned rev), `gpui-pre-0.3.6` from the cargo registry. Paths below are repo-relative unless absolute. `G=` means `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.6`. `U=` means `/Users/shin1ohno/ManagedProjects/herdr-uikit`.

## BLUF

- No key band, extra-keys row, sticky modifier, pane pager, compact top bar, or overlay sidebar exists. Every piece of 02 and 06 is new code.
- The keyboard inset already reaches layout. herdr-uikit samples the keyboard layout guide and animates `insets.ime.bottom` (`U/src/window/layout.rs:157-190`, `208-290`). GPUI folds it into `fully_visible_bounds()` (`G/src/window.rs:2787-2799`), and the window root is padded by it (`crates/herdr-gpui/src/window/viewport.rs:6-24`, applied at `crates/herdr-gpui/src/window/render.rs:926`). A band appended as the last child of the root column sits directly on top of the keyboard and rides its animation, with no platform change.
- GPUI cannot tell "software keyboard visible" from "hardware keyboard assistant bar". It exposes only the merged inset. The assistant bar reserves about 55pt (`U/src/window/layout/keyboard_geometry/tests.rs:54-58`), and floating/undocked keyboards reserve 0 (`U/src/window/layout/keyboard_geometry.rs:9-13`). herdr-uikit tracks real visibility (`U/src/window/keyboard.rs:15-21`), but keeps it `pub(super)`.
- Two-finger swipe is NOT available. GPUI's portable recognizer tracks one touch and ignores concurrent ones (`G/src/gestures.rs:478-489`, test `G/src/gestures.rs:1796-1829`). Raw `TouchEvent`s are never dispatched to elements, only recognized gestures are (`G/src/window.rs:5632-5720`). Handoff 未決 2 (2本指の横スワイプ) needs new herdr-uikit work. A one-finger `TouchDragEvent` claimed on the 36pt pager strip, or taps on the dots, works today.
- Single-pane mode differs per backend. Herdr 2 (herdr-core) zoom is client-local (`crates/herdr-gpui/src/core_terminals/commands.rs:89-95`). Herdr 1 zoom is a daemon request, `pane.zoom` with `mode: "toggle"` only (`crates/herdr-commands/src/lib.rs:879-882`), and `zoomed` lives on the shared tab snapshot (`crates/herdr-protocol/src/wire.rs:402`). Auto-zooming Herdr 1 on keyboard show therefore mutates daemon state [推測: other attached clients see it too; no Herdr checkout to confirm].
- At 320pt the sidebar is squeezed, not overlaid. The expanded width is `min(clamp(pref,160,480), window-240)`, i.e. 80pt (`crates/herdr-gpui/src/sidebar/metrics.rs:35-40`). The rail is `min(64, window-240)` (`crates/herdr-gpui/src/sidebar/rail.rs:66-68`). No breakpoint, drawer, or overlay code exists.
- `window/render.rs` is 987 lines and `window/tab_strip.rs` is 933 lines. Both are at the 1,000-line ceiling, so the band, pager, top bar, and overlay must live in new modules.

## Screen 02 — handheld + software keyboard

### 02.1 Existing key band / extra keys

None. `git grep -i 'accessory|key_row|extra.?keys|key_bar|softkey'` over `crates/**/*.rs` hits only the Settings "iPad key" tests (`crates/herdr-gpui/src/settings_window/tests/ipad_key.rs:115,137`), which concern the SSH key, not keyboard keys. herdr-uikit has no `inputAccessoryView` (grep in `U/src` matched only comments about UIKit's assistant bar, `U/src/window/text_input/state.rs:25`). The band must be drawn in GPUI, not as a UIKit accessory view [推測: a GPUI band also keeps the theme and the 1,000-line rules in one repo].

### 02.2 How keys reach a terminal today

Wire type: `ClientPaneInputEvent::{Key{code,modifiers,kind,..}, TextCommit, Mouse, Paste}` (`crates/herdr-protocol/src/wire.rs:92-113`). Modifier bits are shift=1, control=2, alt=4 (`wire.rs:90`). `ClientKeyCode` includes `Esc, Tab, BackTab, Left/Right/Up/Down, Char(char)` (`wire.rs:23-42`).

GPUI keystroke → wire:
- `terminal::key_input` / `keystroke_input` / `key_code` (`crates/herdr-gpui/src/terminal.rs:513-529`, `546-567`, `623-664`). `"escape"`→`Esc`, `"tab"`→`Tab` (`BackTab` with shift), arrows→`Up/Down/Left/Right`, ctrl+single char→`Char(c)` with bit 2 (`terminal.rs:650`), alt+char only when `alt_keys` (`terminal.rs:654-661`). Printable text is deliberately not sent here, because it belongs to the input handler (`terminal.rs:509-512`).
- Herdr 1 pane: `HerdrWindow::key_down` (`crates/herdr-gpui/src/window/input.rs:290-431`). Escape first cancels drags (`input.rs:383-389`). Copy mode owns keys (`input.rs:326-333`). Otherwise `[pane_keys]` or `key_input`, then `self.send(input)` (`input.rs:418-430`; `send` at `crates/herdr-gpui/src/window/pending_input.rs:128`).
- Herdr 2 core terminal: `CoreTerminalView::key_down` (`crates/herdr-gpui/src/core_terminals/view.rs:202-265`) → `self.send` → `Command::Input` (`view.rs:180-182`).
- Herdr's prefix chord is a GPUI keystroke interceptor (`crates/herdr-gpui/src/window/prefix.rs:35-60`). Only keystrokes that go through GPUI dispatch can arm it.

Software keyboard → GPUI:
- Letters arrive as `insertText:` → `handler.replace_text_in_range(None, text)` (`U/src/window/text_input/editing.rs:19-26`). Return (a lone control char) and backspace are synthesized as key taps through `tap_key` (`editing.rs:34-54`, `59-73`, `139-148`).
- Herdr 1 sink: `HerdrWindow::replace_text_in_range` → `TextCommit(text)` (`crates/herdr-gpui/src/input.rs:185-214`). Herdr 2 sink: `CoreTerminalView::replace_text_in_range` → `TextCommit` (`crates/herdr-gpui/src/core_terminals/view.rs:453-468`).
- Terminal text traits default to no assistance (`U/src/window/text_input/traits.rs:1-6`, `Default`), so no autocapitalization or autocorrect alters letters.

Implications for the band:
- **esc / tab / arrows**: synthesize through `window.dispatch_keystroke(Keystroke::parse("escape"|"tab"|"left"…), cx)` (`G/src/window.rs:5373`; already used by the app, e.g. `crates/herdr-gpui/src/settings_window/native/fonts.rs:167`). This routes to whichever terminal is focused (Herdr 1 or core) and through copy mode, drag-cancel, and the prefix interceptor, exactly like a hardware key. Sending `ClientPaneInputEvent` directly would bypass copy mode and the prefix and would need two code paths (pane vs core) [推測: preferred design].
- **ctrl as sticky/toggle**: no latch exists. The next soft-keyboard letter arrives as text, not as a keystroke, so the latch must intercept the text commit in BOTH input handlers (`input.rs:185-214`, `core_terminals/view.rs:453-468`). It turns a single-char commit into a ctrl keystroke and clears the latch. Re-dispatching a keystroke from inside `replace_text_in_range` while the view is being updated risks a double lease [推測: defer with `cx.defer_in`/`window.defer`]. The latch state must be reachable from both `HerdrWindow` and `CoreTerminalView` [推測: a small window-scoped state or GPUI global]. ctrl+key via `dispatch_keystroke` would also let ctrl, then `b`, arm Herdr's prefix (`prefix.rs:35`), which matches the TUI.
- **alt**: `key_code` sends alt+char only when `option_as_alt` resolves true (`terminal.rs:651-661`; `window/input.rs:339-347`). A band alt latch should therefore synthesize the alt keystroke with `alt_keys` forced, or send `Key{Char(c), modifiers:4}` [推測].
- **`|` `~` `/` `-`**: plain text. Commit them as `TextCommit` through the focused input handler, or via `dispatch_keystroke` with the char [推測].
- **Paste**: `window.dispatch_action(Box::new(crate::actions::Paste), cx)` already routes to the focused element: a Herdr pane (`crates/herdr-gpui/src/window/render.rs:336` → `window/input.rs:283-288`), a core terminal (`core_terminals/view.rs:393-395`), or chat (`crates/herdr-gpui/src/chat/view.rs:361`). The action is defined at `crates/herdr-gpui/src/actions.rs:33`. Reading another app's text triggers the iOS paste-permission prompt (`U/src/platform/pasteboard.rs:96-97` notes `hasStrings` first because reading "may ask the user for permission").
- **Hide keyboard**: `window.dismiss_virtual_keyboard()` (`G/src/window.rs:2810-2812`). herdr-uikit then records an application dismissal (`U/src/window/keyboard.rs:145-150`). Re-showing needs a gesture that calls `request_virtual_keyboard` (`crates/herdr-gpui/src/input.rs:57-61`; pane tap `crates/herdr-gpui/src/window/render.rs:386-405`; mouse-report tap `crates/herdr-gpui/src/window/mouse.rs:268-270`; core tap `crates/herdr-gpui/src/core_terminals/view/pointer.rs:76-79`).
- **Arrow auto-repeat on hold**: not provided by a tap. A `LongPressEvent` claim (pattern `crates/herdr-gpui/src/menu/context_menu.rs:27-57`) plus a timer would be needed [推測].
- **Focus**: band buttons must not take GPUI focus (no `track_focus`), so the terminal keeps text focus. Taps are synthesized mouse down/up (`G/src/gestures.rs:483-486`), so `on_click` works [推測: that a tap on the drawing view never resigns the UIKit text responder; needs a device check].

### 02.3 Keyboard insets and placing a 76pt band

- Platform: layout-guide inset (`U/src/window/layout.rs:157-190`, disables `usesBottomSafeArea` at `163-167`). Frame-change notifications animate on display-link ticks (`layout.rs:208-290`). Both call `insets_changed` → GPUI `window.refresh()` (`G/src/window.rs:1848-1855`).
- GPUI: `WindowInsets { safe_area, ime }`, `effective()` = per-edge max (`G/src/platform.rs:842-864`). Public reads are only `fully_visible_bounds()` and `visual_viewport_bounds()` (`G/src/window.rs:2779-2799`). There is no public `insets()`.
- App: `viewport::pad` pads the root by `fully_visible_bounds` (`crates/herdr-gpui/src/window/viewport.rs:6-24`; `crates/herdr-gpui/src/window/render.rs:926`). Menus also clamp to it (`crates/herdr-gpui/src/menu/chrome.rs:200,256`; `crates/herdr-gpui/src/menu/chrome/anchor.rs:14`).
- Placement: append the band after `window-body` (`render.rs:943-971`) and before the worktree banner (`render.rs:977`), gated on `ipad_ui::active()` and "software keyboard shown". Because the root is already padded, the band's bottom edge is the keyboard's top. The terminal surface shrinks by 76pt and the daemon resizes. Text focus survives that resize on iOS (`crates/herdr-gpui/src/input.rs:88-106`, `RETAIN_TEXT_FOCUS_DURING_RESIZE`).
- "Software keyboard shown" signal: (a) a heuristic on `viewport_size().height - fully_visible_bounds().bottom()` above a threshold that excludes the 55pt assistant bar and the ~20pt home-indicator safe area [推測: e.g. >120pt], or (b) a new herdr-uikit public API exporting `Keyboard::visibility` (`U/src/window/keyboard.rs:74-86`). Option (b) is cross-repo and needs a rev bump. A floating keyboard yields a 0 inset (`U/src/window/layout/keyboard_geometry.rs:9-13`), so with (a) the band does not appear for floating keyboards.
- Headless tests can drive this: `TestWindow::simulate_insets_change` (`G/src/platform/test/window.rs:182-192`). herdr-gpui does not use it yet (git grep: 0 hits; positive control: the method exists at that line).

### 02.4 Single-pane mode while the keyboard shows

- Herdr 1: the surface is one frame per tab with pane rects (`crates/herdr-protocol/src/wire.rs:437-450`, `537-546`). A client cannot enlarge one pane locally; it can only crop. Full-width single pane needs daemon zoom: `Command::Zoom` → `pane.zoom` `{"mode":"toggle"}` (`crates/herdr-commands/src/lib.rs:879-882`; `crates/herdr-client/src/method.rs:59`; also `crates/herdr-gpui/src/pane_menu.rs:136`). Only "toggle" appears anywhere in the repo. The UI does not read the tab's `zoomed` flag (`wire.rs:402`): git grep `\.zoomed\b` outside core_terminals and tests returns 0 hits.
- Herdr 2: client-local zoom. `shown.set_zoom(Some(terminal))` (`crates/herdr-gpui/src/core_terminals/commands.rs:89-95`) and the zoom animation in split drawing (`crates/herdr-gpui/src/core_terminals/split.rs:29,63-88`). Moving focus unzooms (`commands.rs:70-73`). The pane order for a pager is `Panes::terminals()`, "left to right and top to bottom" (`crates/herdr-gpui/src/core_terminals/panes.rs:360-369`).
- Zen mode is client-local but hides chrome and other groups, not panes (`crates/herdr-gpui/src/window/zen.rs:1-8`). It is not a single-pane mode.

### 02.5 Paging between panes ("Pane 1 of 2 ● ○")

- Herdr 1: `Command::NextPane/PreviousPane` cycle the panes of the focused tab via `pane.focus` (`crates/herdr-commands/src/lib.rs:863-877`). The count is `snapshot.panes` filtered by workspace and tab (same filter, `lib.rs:865-868`).
- Herdr 2: `core_pane_command` handles only directional focus, resize, zoom, split, and close; `NextPane/PreviousPane` fall to `_ => return false` (`crates/herdr-gpui/src/core_terminals/commands.rs:44-106`). A pager needs a next/previous over `panes.terminals()` that keeps zoom on the new pane, because the existing focus move unzooms (`commands.rs:70-73`).
- Gestures: two-finger swipe is impossible in-repo (see BLUF; `G/src/gestures.rs:478-489`, `G/src/window.rs:5632-5720`). `GestureKinds` has only tap, long_press, pan, and pinch (`G/src/gestures.rs:353-363`). A single-finger horizontal pan on the terminal becomes a horizontal `ScrollWheelEvent` (`G/src/gestures.rs:1-11`), which the terminal forwards as wheel input (`crates/herdr-gpui/src/window/input.rs:214-279`), so it would conflict. Feasible today: (1) taps on the dots or the label, and (2) a one-finger `TouchDragEvent` claimed with `prevent_default` at `Started` on the pager strip (`G/src/gestures.rs:386-402`; claim logic `G/src/window.rs:5685-5693`). The claim pattern follows the LongPress capture in `crates/herdr-gpui/src/menu/context_menu.rs:27-57`. `TouchDragEvent` is unused in herdr-gpui so far (git grep: 0 hits outside tests). A real two-finger swipe needs herdr-uikit to add a native 2-touch `UIPanGestureRecognizer`, like its existing scroll and hover recognizers (`U/src/window/pointer.rs:44-66`), plus a delivery channel (a new public hook, since `PlatformInput` has no swipe variant: `G/src/interactive.rs:773-799`). That is a cross-repo contract.

### 02.6 Other 02 items

- "タブの ⌘ ヒントは出さない": the hints are 01's (not yet implemented in my scope). The same "soft keyboard shown" signal can gate them.
- Strings: no existing band strings (grep over `locales/en-US/*.ftl` found none). Add a new feature file, e.g. `locales/en-US/ipad-keys.ftl`. The build script embeds every `<feature>.ftl` (`crates/herdr-gpui/src/i18n.rs:1-4`; `crates/herdr-gpui/build.rs:73`).
- Motion: the band slides with the keyboard inset animation for free (`U/src/window/layout.rs:262-271`). Band-internal toggles (the ctrl highlight) follow `crate::motion` per AGENTS.md.

## Screen 06 — Slide Over, 320pt

### 06.1 How narrow widths behave today

- Sidebar column: `sidebar_column_width` (`crates/herdr-gpui/src/sidebar/activity.rs:251-261`) → `SidebarMode::width` (`crates/herdr-gpui/src/sidebar/rail.rs:61-71`). Expanded = `sidebar_width(pref, w)` = `pref.unwrap_or(232).clamp(160,480).min(w-240)` (`crates/herdr-gpui/src/sidebar/metrics.rs:7,35-40`), so 80pt at 320pt. Rail = `min(64, w-240)`, also 64pt at 320 (`rail.rs:38,67`). The activity bar adds 44pt when it leads (`activity.rs:35,258-260`). The 240pt terminal reserve is the only width rule. No breakpoint, drawer, or overlay exists (git grep `overlay|drawer` in sidebar/window/titlebar finds only unrelated overlays, `crates/herdr-gpui/src/window/reconnecting.rs:98`).
- Sidebar visibility/animation: `toggle_sidebar` (`crates/herdr-gpui/src/sidebar/rail.rs:97-102`), `sliding_sidebar` / `slide_start` (`crates/herdr-gpui/src/window/zen.rs:84-104,159-170`). The column renders in-flow beside the terminal (`crates/herdr-gpui/src/window/render.rs:952-958`).
- Dialogs already compact below 560pt for Add Device (`crates/herdr-gpui/src/menu/devices/rail.rs:12`; `crates/herdr-gpui/src/menu/chrome.rs:259-262`).
- Several browser "groups" can sit side by side (`crates/herdr-gpui/src/window/render.rs:721-793`). Zen mode keeps only the active group (`crates/herdr-gpui/src/window/zen.rs:1-8,288-305`). At 320pt more than one group is unusable [推測].
- `docs/ios.md:517-520` lists "narrow multitasking layouts" as still needing iPad checks.

### 06.2 What a 52pt top bar (menu 44 / tab switcher / ••• 44) would reuse

- Title bar: `render_titlebar` / `titlebar::frame` (`crates/herdr-gpui/src/window/render.rs:937-939,985`). Tabs-in-titlebar merge flag (`render.rs:717-720`). Tab strip data (`crates/herdr-gpui/src/titlebar/tabs.rs:66,87,128` read `sidebar_column_width`). The in-window application bar is not built on iOS (`crates/herdr-gpui/src/menus.rs:27`).
- Tab switching: `Command::NextTab/PreviousTab/TabNumber` (`crates/herdr-commands/src/lib.rs:29-30` enum; `TabNumber` mapping `lib.rs:891-899`). There is no tab-list page in `menu::Page` (`crates/herdr-gpui/src/menu/page.rs:5-140`; `Page::Tab` is one tab's context menu). Candidates for the chevron: the palette's `Filter::Navigation` (`crates/herdr-gpui/src/palette.rs:76-84`, opened by `WorkspacePicker`, `crates/herdr-gpui/src/window/commands.rs:459-469`), or a new menu page.
- `•••`: the group "…" menu `open_group_menu` (`crates/herdr-gpui/src/group_menu.rs:191`). It covers new tab kinds, close, and split.
- Menu button → full overlay sidebar: the sidebar view already renders standalone via `crate::sidebar::cached_view(&self.sidebar_view, width)` (`crates/herdr-gpui/src/window/render.rs:127-132`). Overlaying it absolutely with a scrim, and dismissing it on selection, is new code.
- Status dot colors: from the existing activity indicators (`crates/herdr-gpui/src/sidebar` `Indicators`, imported at `crates/herdr-gpui/src/core_terminals/split.rs:13`).

### 06.3 06-specific gaps

- A width breakpoint (e.g. `fully_visible_bounds().width < ~400pt` [推測]) that switches to: no in-flow sidebar (`SidebarMode::Hidden`-like for layout only, without touching `sidebar_visible` or the daemon's `ui.sidebar_collapsed_mode` at `rail.rs:84-95`), the 52pt top bar, the 36pt pager, and a 13px terminal font. Zen already shows how to change layout without rewriting flags (`zen.rs:6-8`). Font size changes resize the terminal (`zen.rs:186-190` notes the jolt).
- The 56pt horizontally scrollable key row (esc/ctrl/tab/↑/↓/⋯) shares the 02 key model. Only its geometry and the overflow `⋯` differ.
- Pager and single-pane: same as 02.4/02.5. The handoff does not say whether 06 is 1-pane always or only with the keyboard [推測: always, given 36pt "Pane 1 of 2"].

## Suggested module layout (keeps files < 1,000 lines)

- `crates/herdr-gpui/src/ipad_keys.rs` + `ipad_keys/{model,band,latch}.rs`: the key catalog (one table per AGENTS "declare each item once"), the ctrl/alt latch, and the band render. Tests in `ipad_keys/tests/*.rs`.
- `crates/herdr-gpui/src/window/compact.rs` (06 breakpoint, top bar, overlay sidebar) and `window/pane_pager.rs` (count, label, dots, TouchDrag claim). `render.rs` gets one `.children(...)` line each, since it is at 987 lines.
- Latch hooks: about 5-line calls in `crates/herdr-gpui/src/input.rs:185-214` and `crates/herdr-gpui/src/core_terminals/view.rs:453-468`.
- Sizes 44/48/52/56/76 come from the handoff §4 step 1 constants module (not yet present).

## Risks

1. **Shared zoom (Herdr 1)**: auto-zooming on keyboard show sends `pane.zoom toggle` (`herdr-commands/src/lib.rs:879-882`). Toggle semantics can invert if the tab was already zoomed, and zoom is daemon state (`wire.rs:402`) [推測: visible to the desktop and TUI clients of the same session]. A client-local alternative cannot enlarge a pane (`wire.rs:437-450`).
2. **Keyboard-visible detection** is heuristic without a herdr-uikit API: the assistant bar is 55pt (`keyboard_geometry/tests.rs:54-58`) and a floating keyboard reserves 0 (`keyboard_geometry.rs:9-13`).
3. **Two-finger swipe** (未決 2 default) is not implementable in herdr-gpui (`gestures.rs:478-489`; `window.rs:5632-5720`). It needs a herdr-uikit change, a public hook, and a pin bump.
4. **Re-entrancy**: dispatching a keystroke from inside `replace_text_in_range` (latch) while the view is leased [推測: defer it].
5. **Paste prompt**: a custom Paste button reading another app's text shows iOS's paste-permission alert (`U/src/platform/pasteboard.rs:96-97`). `UIPasteControl` is not wired.
6. **File ceilings**: `window/render.rs` 987 and `window/tab_strip.rs` 933 lines; `sidebar/rail.rs` 924.
7. **Focus loss**: a band tap must not move GPUI focus or resign UIKit's text responder [推測; device check].
8. **Prefix interplay**: ctrl latch + `b` arms the prefix only when sent via `dispatch_keystroke` (`prefix.rs:35`). Sending a raw `Key` bypasses it, which diverges from the 07 setting semantics.
9. **Desktop leakage**: everything must sit behind `ipad_ui::active()` (`crates/herdr-gpui/src/ipad_ui.rs:15-24`). Tests default to the iPad side, and `ipad_ui::desktop()` (`ipad_ui.rs:29-32`) is needed to assert desktop is unchanged.
10. Handoff says the desktop sidebar is 272pt; the code has 232 (`sidebar/metrics.rs:7`). This matters for 01 and for the 06 overlay width.

## Unknowns

- Whether Herdr's `pane.zoom` accepts explicit on/off modes, and whether zoom and pane focus are per-client or session-wide (no Herdr checkout here; only `"toggle"` appears in this repo).
- Whether a zoomed Herdr 1 tab stays zoomed on the newly focused pane after `pane.focus` (NextPane).
- Soft keyboard heights per orientation/model and the exact threshold separating it from the assistant bar on real iPads.
- Whether 06 is single-pane always or only with the keyboard, and the 06 breakpoint width.
- Native behaviour of band taps on the UIKit first responder, and `dispatch_keystroke` re-entrancy from an input-handler callback.
- Whether herdr-uikit should own a 2-finger recognizer (cross-repo contract) or the pager accepts a one-finger strip swipe instead.

## Evidence count

Concrete file:line references in this file: about 95 (herdr-gpui about 60, herdr-uikit about 15, gpui-pre about 20). Screens covered: 02 (§02.1-02.6), 06 (§06.1-06.3).
