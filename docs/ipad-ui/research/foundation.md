# Foundation research — iPad UI (screens 01–08)

Repo: `/Users/shin1ohno/ManagedProjects/herdr-gpui` @ `feat/ipados-foundation` (76a86890). Paths below are repo-relative unless absolute. Read-only investigation; nothing built or run beyond `xcrun simctl list` (read-only).

## 0. Headline findings (read first)

1. **Nord already ships as a hand-authored built-in theme.** It does not need to be bundled from Ghostty. `"Nord"` is in `Theme::BUILTIN_NAMES` (`crates/herdr-gpui/src/config/theme.rs:386-393`) and its own `match` arm in `Theme::builtin` (`theme.rs:553-560`) runs before the bundled-file fallback (`theme.rs:585-590`). Adding `assets/themes/Nord` from Ghostty would duplicate the name, and the hand-authored arm would shadow the file. The handoff step 「Nord が無ければ同梱」 is a no-op.
2. **The built-in Nord does not produce the mock's roles.** Its chrome is *derived* (`derive_chrome`, `theme.rs:542-547`), and its accent is **nord15 purple, not nord8**: `primary()` falls back to `palette[5]` = `0xb48ead` (`theme.rs:452-456`, `theme.rs:557`). Herdr's own `nord` UI palette (reached only through the `Follow Herdr` theme) does give nord8 / nord0 / nord2 / nord3 / nord6 (`crates/herdr-gpui/src/herdr_settings/palette.rs:110-113`, `palette.rs:245-264`). See §1 for the full table. This needs a decision (§R1).
3. **`ipad_ui::active()` is TRUE in every `cfg(test)` build** unless a test holds `ipad_ui::desktop()` (`crates/herdr-gpui/src/ipad_ui.rs:14-24`, `ipad_ui.rs:28-32`). Any *size* change gated on `active()` (sidebar 300, tab strip 52, status bar 44) therefore changes every existing headless desktop layout test. Example: `crates/herdr-gpui/src/sidebar/layout_tests.rs:352-353` asserts `sidebar_width(None, w) == SIDEBAR_WIDTH`. This is the main Wave-0 decision (§R2).
4. **Desktop sidebar default is 232, not 272.** The handoff's 「デスクトップは 272 のまま」 is wrong: `SIDEBAR_WIDTH = 232.` (`crates/herdr-gpui/src/sidebar/metrics.rs:7`) and `RAIL_WIDTH = 64.` (`crates/herdr-gpui/src/sidebar/rail.rs:38`, mock: 72).
5. **The app has no hardware-keyboard presence signal.** 01/08 show ⌘ hints and a 「⌨ Hardware keyboard」 item only while a physical keyboard is attached. herdr-uikit says apps cannot read the hardware keyboard state (`/Users/shin1ohno/ManagedProjects/herdr-uikit/src/window/text_input/keys.rs:31`), and the app only sees `window.fully_visible_bounds()` (`crates/herdr-gpui/src/window/viewport.rs:6-15`). This is a cross-repo contract (§R4).
6. **There is no simulator instance at 1194×834.** The device type **"iPad Pro (11-inch) (4th generation)"** is exactly 834×1194 pt (2388×1668 px @2x) and the iOS 26.5 runtime supports it, but no instance of it exists. The closest instance, "iPad Pro 11-inch (M5)", is 1210×834 pt landscape (§6).

## 1. Theme struct and colour roles

`pub struct Theme` (`crates/herdr-gpui/src/config/theme.rs:307-329`): 24-bit RGB `u32` fields:
`background`, `foreground`, `cursor`, `bold: Option`, `surface`, `active`, `muted`, `sidebar: Option` (Herdr `sidebar_bg`, sidebar only), `accent: Option`, `palette: [u32;256]`, `contrast`, `system_accent: Option`.

Derived accessors:

| Accessor | Definition | Evidence |
| --- | --- | --- |
| `primary()` | `system_accent` → `accent` → `palette[5]` | `theme.rs:452-456` |
| `sidebar_background()` | `sidebar` else `surface` | `theme.rs:467-469` |
| `subtext()` | `ink(mix(background, foreground, 78))` | `theme.rs:473-475` |
| `dimmed(c)` | `mix(surface, c, 55)` | `theme.rs:478-480` |
| `primary_wash()` | `mix(surface, primary, 22)` (current-tab fill) | `theme.rs:485-487` |
| `text_on(fill)` | bg or fg, whichever contrasts more | `theme.rs:492-503` |
| `ink(c)` | contrast-lift of a chrome mark vs bg/surface/active | `theme.rs:509-515` |

**No `warning`, `border`, `text`, `accent`-as-required fields exist.** In practice:
- Borders use `theme.active`: 222 `border_color(rgb(..theme.active` sites, against 6 for `muted`, 5 for `foreground` and 3 for `primary()` (grep count over `crates/herdr-gpui/src`).
- Warnings use `palette[3]` (yellow). Examples: `crates/herdr-gpui/src/icons.rs:156-163`, `crates/herdr-gpui/src/automations/model.rs:39`, `crates/herdr-gpui/src/menu/devices/saved_warning.rs:82`.
- Error / ok colours: `palette[1]` red / `palette[2]` green (`crates/herdr-gpui/src/browser/kit.rs:48-50`, `crates/herdr-gpui/src/chat/view.rs:117-120`).
- `theme.muted` is used as text in 414 sites (grep count of `text_color(rgb(theme.muted))` / `self.theme.muted`).

Chrome for the hand-authored built-ins is derived: `surface = mix(bg,fg,5)`, `active = mix(bg,fg,12)`, `muted = mix(bg,fg,55)` (`theme.rs:542-547`).

### 1a. Nord role map (handoff §2 → existing fields)

Values for the **built-in "Nord"** were computed with `mix()` from `theme.rs:366-373` (bg `0x2e3440`, fg `0xd8dee9`, `theme.rs:554-555`). Values for **Follow Herdr + Herdr `nord`** come from `palette.rs:110-113` mapped by `Palette::theme()` (`palette.rs:245-264`).

| Nord / handoff role | Built-in `Nord` field → value | Follow-Herdr `nord` field → value | Verdict |
| --- | --- | --- | --- |
| nord0 `#2E3440` terminal face | `background` = 2e3440 ✓ | `background` = surface_dim 2e3440 ✓ | exists |
| nord1 `#3B4252` sidebar / toolbar / sheet | `surface` = **363c48** (derived); `sidebar` None → `sidebar_background()` = 363c48; nord1 only as `palette[0]` (ANSI black, `theme.rs:557`) | `surface` = panel_bg **2e3440** (nord0); Herdr `surface0` = 3b4252 (index 4) is **not mapped** into `Theme` (`palette.rs:249-252`) | **no field holds nord1 as chrome** |
| nord2 `#434C5E` floating parts | `active` = **424854** (≈nord2, derived) | `active` = active_row_bg 434c5e ✓ | `active` |
| nord3 `#4C566A` borders | borders use `active` (424854); `muted` = 8b919c | `muted` = overlay0 4c566a ✓ — but `muted` is also text in 414 sites, so nord3 *becomes text* (1.69:1 on nord0, computed) | border role = `active`; nord3-as-border **none** on built-in |
| nord6 `#ECEFF4` body text | `foreground` = **d8dee9** (nord4); nord6 only `palette[15]` | `foreground` = text eceff4 ✓ | built-in body text is nord4 |
| nord4 `#D8DEE9` secondary text | `subtext()` = ink(**b2b8c3**); `muted` 8b919c | `subtext()` = mix(nord0, nord6, 78); Herdr subtext0 d8dee9 (index 10) not mapped | `subtext()` (≠nord4) |
| nord8 `#88C0D0` selection / primary / focus | `primary()` = palette[5] = **b48ead (nord15)**; nord8 only `palette[6]` | `primary()` = palette[5] ← accent 88c0d0 ✓ (`palette.rs:257-262`) | **built-in Nord's accent is purple** |
| nord13 `#EBCB8B` working / Needs you / badge / warning | `palette[3]` = ebcb8b ✓ | `palette[3]` ← yellow ebcb8b ✓ | `palette[3]` |
| nord14 `#A3BE8C` connected / done ring | `palette[2]` = a3be8c ✓ | `palette[2]` ✓ | `palette[2]` |
| nord7 `#8FBCBB` waiting | `palette[14]` = 8fbcbb ✓ | `palette[6]`/`[14]` ← teal 8fbcbb ✓ | `palette[14]` |
| nord11 `#BF616A` error | `palette[1]` = bf616a ✓ | `palette[1]` ✓ | `palette[1]` |

### 1b. Status dots are NOT theme palette colours

- `status_style` uses **fixed Catppuccin Mocha literals** whatever the theme: Working f9e2af filled, Blocked f38ba8 filled, Done 94e2d5 filled, Idle a6e3a1 **hollow**, Unknown 6c7086 at 3px (`crates/herdr-gpui/src/sidebar/agents.rs:588-601`; locked by a test over Default/Nord/Dracula/Mocha in `crates/herdr-gpui/src/sidebar/tests/statuses.rs:4-28`).
- When shared Herdr settings are loaded, `Indicators::new` takes Herdr's palette status colours instead (`agents.rs:272-298`). For nord that gives Working ebcb8b, Blocked bf616a, Done 8fbcbb, Idle a3be8c, Unknown 4c566a (`palette.rs:228-240`).
- The window passes `this.settings.shared` (`crates/herdr-gpui/src/window.rs:437-445`). On iPad shared settings are local-file based. [推測] They are usually absent there, so the iPad shows the Mocha literals.
- **Semantic mismatch with the handoff.** The handoff says 「待機 = 水色、完了 = 緑の輪」. Code and `assets/README.md:17-19` say *Done (unseen completion) = teal filled, Idle = green hollow ring*. The handoff also says to follow the existing display, so the code wins; spec text should be corrected.
- Dot size: `STATUS_WIDTH = 8.` (`metrics.rs:14`). The mock wants 10px (01) and 12px (03 rail).

## 2. Nord in assets / Ghostty and the attribution pattern

- `assets/themes/` holds 28 Ghostty files and **no Nord** (listing). They are compiled in by `bundled_themes!` with `include_str!` (`theme.rs:375-427`) and parsed once (`theme.rs:430-435`).
- The `Nord` built-in is hand-authored (`theme.rs:553-560`). Herdr's `nord` is in `THEME_NAMES` (`crates/herdr-gpui/src/herdr_settings.rs:39`) and in the palette row (`palette.rs:110-113`).
- Ghostty's own file exists locally: `/Applications/Ghostty.app/Contents/Resources/ghostty/themes/Nord` (bg 2e3440, fg d8dee9, palette 8 = **596377** against the built-in's 4c566a, selection eceff4/4c566a). It has no accent key.
- Attribution pattern, if a Ghostty file is ever added:
  - `crates/herdr-gpui/THEMES-NOTICE.md:1-8`: unmodified iTerm2-Color-Schemes copies, revision `c379d7a3…`, MIT text.
  - `NOTICE:49-53`: the summary paragraph.
  - `README.md` has no per-theme attribution, only screenshots (`README.md:89-114`).
  - Also add the file name to `bundled_themes![…]` (`theme.rs:398-427`; keep the list sorted).
- **Conclusion:** do not bundle. The real gap is the role mapping in §1a (no nord1 chrome, purple accent on the built-in).

## 3. `crate::ipad_ui` — what it provides and every use

`crates/herdr-gpui/src/ipad_ui.rs` (42 lines): `active()` is `cfg!(target_os="ios")` in production and `!DESKTOP` in tests (`ipad_ui.rs:14-24`). `desktop()` returns a guard that flips the current thread to the desktop side (`ipad_ui.rs:28-42`). The module is declared at `crates/herdr-gpui/src/main.rs:83`.

Every use (`git grep ipad_ui`, 17 hits, 9 production files):

- `crates/herdr-gpui/src/core_terminals/split.rs:332`, `:372`: reconnect placeholder.
- `crates/herdr-gpui/src/menu/devices/add_device.rs:267`, `:292`, `:343`, `:417`, `:496`: dialog frame, scroll body, footer wrap, pane.
- `crates/herdr-gpui/src/menu/devices/add_device/host_key_code.rs:73`, `:94`.
- `crates/herdr-gpui/src/settings_window/pages/general.rs:63`.
- `crates/herdr-gpui/src/settings_window/pages/ipad_key.rs:31`, `:37`.
- `crates/herdr-gpui/src/settings_window/search.rs:57`: `catalog::Availability::Ipad` pages (`crates/herdr-gpui/src/settings_window/catalog.rs:22`, `:85`, `:92`). This is the existing hook for an iPad-only **Keyboard** section (07).
- `crates/herdr-gpui/src/sidebar/rail.rs:492`: long-press menu on rail rows.
- Tests using `desktop()`: `crates/herdr-gpui/src/core_terminals/tests/reconnect.rs:128`, `crates/herdr-gpui/src/settings_window/tests/ipad_key.rs:116`, `crates/herdr-gpui/src/sidebar/rail/tests/touch_context.rs:55`.

182 files in `crates/herdr-gpui/src` use `cfg(target_os = "ios")` (mostly capability gating; `git grep -l` count).

## 4. Where the iPad size constants (44/48/52/56/72/300) should live

Existing homes for geometry:
- `crates/herdr-gpui/src/constants.rs:1-15`: crate-wide reference-design numbers (`TAB_WIDTH 64`, `TAB_HEIGHT 24`), re-exported at `main.rs:221`.
- `crates/herdr-gpui/src/sidebar/metrics.rs:1-41`: sidebar geometry shared by layout, hit testing and label budgets (`SIDEBAR_WIDTH 232`, `ROW_PADDING 12`, `STATUS_WIDTH 8`, `sidebar_width()` clamp 160..480 and terminal reserve 240).
- `crates/herdr-gpui/src/sidebar/rail.rs:38-51`: `RAIL_WIDTH 64`, `HOST_MARK 22`, `HOST_DOT 6`, `AGENT_ICON 14`.
- `crates/herdr-gpui/src/window/status_bar.rs:26-28`: height is `(ui.size*1.5+4).max(22)`, font-derived.
- `crates/herdr-gpui/src/settings_window/ui.rs:21`: `ROW_HEIGHT 38`.
- `crates/herdr-gpui/src/overview/view.rs:17`: `ROW_HEIGHT 44`.

Recommendation [推測, design]: keep the numbers in one iPad table next to the gate. Turn `ipad_ui.rs` into `ipad_ui.rs` + `ipad_ui/metrics.rs`, holding `TOUCH 44`, `ROW 48`, `TAB_STRIP 52`, `SPACE_ROW 56`, `RAIL 72`, `SIDEBAR 300`, `STATUS_BAR 44`, `KEY_BAND 76`, `SHEET_RADIUS 18`, and so on. Expose them through the existing owners as small functions, for example `sidebar::metrics::sidebar_width()` choosing 300 vs 232 and `SidebarMode::width()` choosing 72 vs 64 (`rail.rs:63-70`). Each screen PR then touches only its own module plus that one table. Desktop constants stay where they are, so desktop output cannot change. The table's order carries no meaning, so put it under `// keep-sorted` (AGENTS.md "Parallel Pull Requests").

## 5. Test infrastructure for iPad-only rendering on the host

- **iPad side by default.** Headless tests run `ipad_ui::active() == true` (`ipad_ui.rs:5-7`, `:14-24`). Desktop-path tests opt out with `let _desktop = crate::ipad_ui::desktop();` (pattern at `crates/herdr-gpui/src/sidebar/rail/tests/touch_context.rs:53-55`).
- **Window fixture.** `cx.add_window_view(crate::sidebar::layout_tests::fixture_window)` builds a full `HerdrWindow` (`crates/herdr-gpui/src/sidebar/layout_tests.rs:571-…`). The fixture's `Indicators::new(None, …, &Theme::default())` (`layout_tests.rs:578-584`) means tests use Default theme literals, not Nord.
- **Window size.** `add_window_view` opens `Bounds::maximized` on the test display (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.6/src/app/test_context.rs:342-359`), which is **1920×1080** (`…/gpui-pre-0.3.6/src/platform/test/display.rs:16`). iPad sizes need `cx.simulate_resize(size(px(1194.), px(834.)))`; no existing test uses 1194/834. 320-wide tests exist, e.g. `crates/herdr-gpui/src/menu/devices/add_device/host_key/tests/wrapping.rs:46` and `crates/herdr-gpui/src/sidebar/layout_tests/workspace_menu.rs:139`.
- **Drawing.** `full_draw(window, cx)` (`layout_tests.rs:543-550`) is required because the cached sidebar does not record debug bounds otherwise. `run_frames` (`layout_tests.rs:552-569`) handles animations.
- **Measuring.** `.debug_selector(|| "id".into())` on the element plus `cx.debug_bounds("id")` in the test (`touch_context.rs:23`; selectors like `crates/herdr-gpui/src/sidebar/host_row.rs:148`; 1,140 `debug_selector` sites in the crate). Pixel tolerance helper: `within_device_pixel` (`layout_tests.rs:380`).
- **Touch.** `LongPressEvent{phase: TouchPhase::Started,…}.to_platform_input()` through `window.dispatch_event` (`touch_context.rs:24-38`).
- **en-XA leak tests.** `crate::i18n::leak_tests::under_en_xa(name, |cx, recorder| …)` with `assert_translated(surface, shown, data)` (`crates/herdr-gpui/src/i18n/leak_tests.rs:97-121`, `:130-…`). Each window keeps its own `tests/en_xa.rs` (38 such files). Relevant ones: `crates/herdr-gpui/src/window/tests/en_xa.rs`, `crates/herdr-gpui/src/chat/tests/en_xa.rs`, `crates/herdr-gpui/src/settings_window/tests/en_xa.rs`, `crates/herdr-gpui/src/tab_menu/tests/en_xa.rs`. Memory note: `min_w_0()` truncation can make en-XA see `m…`; fix the layout, not the rule.
- **Theme in tests.** Nord is available as `Theme::builtin("Nord")` (`theme.rs:549`). Existing tests use it (`crates/herdr-gpui/src/sidebar/tests/statuses.rs:13`).

## 6. Simulator screenshot tooling

- **`scripts/ios/sim.sh`** (205 lines): boot, install, launch, wait, `xcrun simctl io <udid> screenshot` (`scripts/ios/sim.sh:162`). The default simulator is `"iPad Pro 13-inch (M5)"`, overridable with `--simulator` / `HERDR_IOS_SIMULATOR` (`sim.sh:18-19`, `:45`). It has `--expect-color RRGGBB --min` (`sim.sh:26-28`, `:202-203`); the Default theme's background is `101419` (`docs/ios.md:373-375`), and **Nord would be `2e3440`**. `sim.sh` has **no orientation option** (grep for orientation/landscape in `sim.sh`: 0 hits).
- **Recipes.** `just ios-app [WAIT]` = `bundle.sh --package herdr-gpui --bin herdr-gpui --no-default-features` + `sim.sh` (`justfile:424-428`). `just ios-sim` covers the uikit examples (`justfile:416-420`); `just lint-ios` (`justfile:409-410`); `just test-ios` runs the uikit tests in the sim plus the Python tooling tests (`justfile:448-454`). The simulator bundle id is `so.pen.herdr-gpui` (`scripts/ios/bundle.sh:153-158`).
- **XCUITest driver** (session scratchpad, not in the repo): `/private/tmp/claude-501/-Users-shin1ohno-ManagedProjects-herdr-gpui/30feb3d3-a1ac-4053-8529-33a68704efed/scratchpad/qa/driver/`.
  - `qa.sh build | run '<json>' [shot] | shot <name>` (`qa.sh:2-4`). It defaults to `QA_SIM=C2348F15-…` (the 13-inch M5) and **`QA_BUNDLE=dev.herdr.gpui.ios-smoke`**, which does not match the app's `so.pen.herdr-gpui` (`qa.sh:7-8` against `bundle.sh:157`). Set both env vars.
  - Actions (`Tests/Driver.swift:6-11`, `:45-117`): tap / double / long / drag / type / kbd / kbdkey / del / key+mods / wait / activate / launch / terminate / alert / keyboard visibility assert / dump / `{"orientation":"landscape"}` (`Driver.swift:107-108`) / home / button / field.
  - Screenshots land in `driver/shots/` (392 files).
- **Devices** (`xcrun simctl list devices available`, iOS 26.5): iPad Pro 13-inch (M5) `C2348F15-A037-4111-8712-5926A51CFE7F`, iPad Pro 11-inch (M5) `ED30D4E4-01BA-452A-B1D4-560C32DB9115`, iPad mini (A17 Pro), iPad Air 13-inch (M4), iPad Air 11-inch (M4) `2332E6A3-…`, iPad (A16).
  - Profile sizes (`…/CoreSimulator/Profiles/DeviceTypes/*.simdevicetype/Contents/Resources/profile.plist`): 11-inch M5/M4 = 1668×2420 px @2x → **834×1210 pt**; 13-inch M5 → 1032×1376 pt; Air 11 / A16 → 820×1180 pt; **iPad Pro (11-inch) 1st–4th gen = 1668×2388 px @2x → 834×1194 pt, i.e. 1194×834 landscape.**
  - The iOS 26.5 runtime's `supportedDeviceTypes` includes "iPad Pro (11-inch) (4th generation)", but **no instance exists**. Creating one (`xcrun simctl create … "iPad Pro (11-inch) (4th generation)"`) is a state change and was not run. Without it, use the 11-inch (M5) and accept 1210×834.

## 7. Files over 900 lines that the 8 screens will touch

`find … -exec wc -l`, over `crates/herdr-gpui/src` and `crates/herdr-commands/src`. The limit is 1,000 (AGENTS.md).

| Lines | File | Screens | Headroom |
| --- | --- | --- | --- |
| 993 | `crates/herdr-gpui/src/terminal_painter.rs` | 01 (pane focus ring) [推測] | 7 |
| 988 | `crates/herdr-commands/src/lib.rs` | 07 / 08 (shortcut resolution) | 12 |
| 987 | `crates/herdr-gpui/src/window/render.rs` | 01 / 02 / 03 / 06 (layout composition) | 13 |
| 987 | `crates/herdr-gpui/src/chat/window.rs` | 05 | 13 |
| 986 | `crates/herdr-gpui/src/menu/devices/add_device.rs` | 04 | 14 |
| 986 | `crates/herdr-gpui/src/config.rs` | 07 (shortcut-style setting) | 14 |
| 948 | `crates/herdr-gpui/src/window/commands.rs` | 07 / 08 | 52 |
| 939 | `crates/herdr-gpui/src/menu/chrome.rs` | 03 (long-press menu chrome), 04 sheet | 61 |
| 937 | `crates/herdr-gpui/src/sidebar/render.rs` | 01 / 03 / 05 | 63 |
| 933 | `crates/herdr-gpui/src/window/tab_strip.rs` | 01 / 03 / 05 / 06 / 08 | 67 |
| 930 | `crates/herdr-gpui/src/window/mouse.rs` | 02 (pane swipe) [推測] | 70 |
| 924 | `crates/herdr-gpui/src/sidebar/rail.rs` | 03 | 76 |
| 916 | `crates/herdr-gpui/src/menu/devices/host_menu.rs` | 07 (ServerKeybindings mirror) | 84 |

Under 900, for reference: `settings_window.rs` 880, `sidebar/activity.rs` 723, `window.rs` 727, `theme.rs` 670, `chat/view.rs` 648, `herdr_settings.rs` 647, `settings_window/catalog.rs` 627, `sidebar/layout.rs` 607, `sidebar/agents.rs` 605, `keymap/daemon.rs` 562, `window/status_bar.rs` 535, `keymap.rs` 521, `icons.rs` 488, `window/prefix.rs` 413, `herdr-commands/src/shortcuts/macos.rs` 67 (wc).

Rule for implementers: in the files with ≤20 lines of headroom, add new iPad code as **new sibling modules**. Do not grow them.

## Per-screen foundation notes

### 01 Main — stand + hardware keyboard
- Sidebar 300: change the default inside `sidebar_width()` (`crates/herdr-gpui/src/sidebar/metrics.rs:35-41`). The user-dragged width from `preferences.rs:123` still wins (clamp 160..480). Desktop default is 232 (`metrics.rs:7`), not 272.
- Colours: see §1a. Body text is nord4 on the built-in Nord, and nord1 chrome does not exist.
- Status dots come from `status_style` (`crates/herdr-gpui/src/sidebar/agents.rs:593-601`) or Herdr settings (`agents.rs:272-298`), sized by `STATUS_WIDTH 8` (`metrics.rs:14`).
- Harness icon: `AgentIcon::from_identity` / `.path()` (`crates/herdr-gpui/src/icons.rs:20`, `:27-30`). There are 26 `assets/icons/agent-*` files; one is PNG (`agent-omo.png`), so not every icon is a single-colour SVG.
- The status bar legend reuses the `status-keybinds` button labelled `status-bar-shortcuts` (`crates/herdr-gpui/src/window/status_bar.rs:136-147`; `crates/herdr-gpui/locales/en-US/status-bar.ftl:18`). The bar's height is font-derived (`status_bar.rs:26-28`).
- Tab strip height: `TAB_HEIGHT 24` (`crates/herdr-gpui/src/constants.rs:15`), used at `crates/herdr-gpui/src/tab_layout.rs:257`, `:322`.
- ⌘ hints only with a physical keyboard: there is no signal for that (§R4).
- Shortcut glyphs: `crates/herdr-commands/src/shortcuts/macos.rs:3-66` matches the mock (Tab cmd-t, SplitRight cmd-d, Workspace cmd-shift-n, WorkspacePicker cmd-p, Palette cmd-shift-p, ToggleSidebar cmd-b, Keybinds cmd-/). The only keycap splitter found is `keycaps()` (`crates/herdr-gpui/src/menu/settings.rs:706-730`), which yields words such as `Cmd`. No ⌘⇧⌃ glyph mapper was found: grep for `u{2318}`, `"cmd" =>` in herdr-gpui/herdr-commands found nothing [未列挙 beyond those patterns].

### 02 Main — handheld + soft keyboard
- Keyboard-safe layout already exists: `window::viewport::margins/pad` use `fully_visible_bounds()` (`crates/herdr-gpui/src/window/viewport.rs:1-24`). The bottom inset follows the soft keyboard (`/Users/shin1ohno/ManagedProjects/herdr-uikit/src/window/layout.rs:204-215`), so the 76pt key band can sit on that margin.
- [推測] "Soft keyboard showing" can be read as a bottom inset larger than the shortcuts-bar height. herdr-uikit's test notes that a hardware keyboard leaves only the shortcuts bar (`/Users/shin1ohno/ManagedProjects/herdr-uikit/src/window/tests.rs:126`).
- Files with no headroom: `window/render.rs` 987, `window/mouse.rs` 930.

### 03 Sidebar minimised (dot rail)
- The rail exists: `SidebarMode::Rail` with `RAIL_WIDTH 64` (`crates/herdr-gpui/src/sidebar/rail.rs:38`, `:53-70`). Collapse mode follows Herdr's `sidebar_collapsed_mode`, default Compact → Rail (`rail.rs:85-101`). `toggle_sidebar` is at `rail.rs:103-108`.
- iPad long-press on rail rows already opens the pane menu (`rail.rs:492`; test `crates/herdr-gpui/src/sidebar/rail/tests/touch_context.rs:5-50`).
- `rail.rs` is 924 lines: put new rail code in a sibling module.
- Rail host dot `HOST_DOT 6` (`rail.rs:41`) against 12 in the mock; agent icon `AGENT_ICON 14` (`rail.rs:48`) against 20 in the mock.

### 04 Add Device sheet
- Already iPad-gated in 5 places in `crates/herdr-gpui/src/menu/devices/add_device.rs` (`:267`, `:292`, `:343`, `:417`, `:496`). The file is 986 lines, so the sheet chrome must go into a new submodule.
- The host-key card's warning frame maps to `palette[3]`, as the existing warning chips do (`crates/herdr-gpui/src/icons.rs:156-163`).
- Narrow-width test precedent: `crates/herdr-gpui/src/menu/devices/add_device/host_key/tests/wrapping.rs:46`.

### 05 Agent chat
- Chat colours come from palette slots (`crates/herdr-gpui/src/chat/view.rs:117-120`). Running state is `palette[3]`/`[2]`/`[1]` (`crates/herdr-gpui/src/chat/view/running.rs:87-89`).
- `chat/window.rs` is 987 lines.
- The user bubble on nord3 needs a fill role. [推測] `theme.active` (nord2 under Follow Herdr, 424854 on the built-in) is the nearest; nothing maps to nord3 on the built-in.
- en-XA test file: `crates/herdr-gpui/src/chat/tests/en_xa.rs`.

### 06 Slide Over (320)
- 320-wide headless precedent: `crates/herdr-gpui/src/window/config_diagnostic/tests/narrow_window.rs:15`.
- `sidebar_width()` keeps a 240pt terminal reserve (`metrics.rs:35-41`) and the rail is dropped when `window_width-240 <= 0` (`rail.rs:63-69`). At 320 the expanded sidebar clamps to 80pt (`min(320-240)`); the mock's "no sidebar, menu overlay" needs a new Hidden-by-width rule.
- Slide Over cannot be simulated in a headless window except by `simulate_resize`. Native: the iOS 26 windowing replaces the classic Slide Over in many configs [推測, not verified].

### 07 Settings — Keyboard
- The iPad-only section hook is `catalog::Availability::Ipad` (`crates/herdr-gpui/src/settings_window/catalog.rs:22`, `:85`, `:92`) checked through `ipad_ui::active()` (`crates/herdr-gpui/src/settings_window/search.rs:57`).
- The prefix logic is in `crates/herdr-gpui/src/window/prefix.rs` (413 lines) and `crates/herdr-gpui/src/keymap/daemon.rs` (562).
- `config.rs` is at 986 lines: a new setting field needs a new `config/<name>.rs`.
- `settings_window/ui.rs:21` row height is 38, against the mock's 56.

### 08 Main — Herdr-prefix style
- Same surfaces as 01. Labels come from the `keymap/daemon.rs` profile, not from `macos.rs`.
- `window/commands.rs` 948, `herdr-commands/src/lib.rs` 988.

## Risks and Unknowns

- **R1 — Theme decision (blocks colour fidelity).** The mock's nord8 accent and nord1 chrome cannot be reached with today's built-in `Nord`: purple `primary()` (`theme.rs:452-456`), derived surface 363c48 (`theme.rs:542-547`). `Accent` has only `Theme`/`System`, so no hex override exists (`crates/herdr-gpui/src/config/accent.rs:11-19`). Options:
  - (a) set `accent: Some(0x88c0d0)` (and maybe `sidebar: Some(0x3b4252)`) on the built-in Nord. This changes desktop Nord, which conflicts with 「デスクトップ不変」.
  - (b) accept the derived roles and judge screenshots by role, not hex.
  - (c) add an iPad-default theme.

  Default theme today is `"Default"` (`crates/herdr-gpui/src/config.rs:491`), so iPad screenshots are not Nord unless the simulator config sets `theme = "Nord"`.
- **R2 — The test gate default.** Because `active()` is true in tests (`ipad_ui.rs:14-24`), gating *sizes* on it changes every desktop layout test, e.g. `crates/herdr-gpui/src/sidebar/layout_tests.rs:352-353`, `:406` and the `LABEL_WIDTH` users at `crates/herdr-gpui/src/sidebar/layout_tests/text_width/sidebar_rows.rs`. Options:
  - (a) a separate layout predicate that is desktop-by-default in tests, with an opt-in guard (recommended, [推測]).
  - (b) wrap all existing layout tests in `desktop()`, a large conflict-prone diff.
  - (c) flip the default, which breaks the existing iPad tests (`touch_context.rs`, `ipad_key.rs`, `add_device`).

  Decide in Wave 0, before parallel PRs.
- **R3 — Desktop numbers in the handoff are off.** Sidebar is 232, not 272 (`metrics.rs:7`); rail is 64 against the mock's 72 (`rail.rs:38`); dots are 8 (`metrics.rs:14`).
- **R4 — No hardware-keyboard signal.** Fixing it is a herdr-uikit change (separate repo, pinned rev `00a8dba`, `Cargo.toml:104`) plus a GPUI-facing contract. That is a cross-repo design decision. The heuristic alternative relies on keyboard-frame height [推測].
- **R5 — Status-dot semantics differ from the spec.** Done is teal filled and Idle is a green hollow ring (`agents.rs:595-599`, `assets/README.md:17-19`); the handoff swaps them. The colours are Mocha literals, not Nord, unless Herdr settings load.
- **R6 — 1,000-line ceiling.** 13 touched files are above 900, six of them with ≤14 lines of headroom (§7).
- **R7 — Tooling fragility.** The XCUITest driver lives only in this session's `/private/tmp` scratchpad, so it is volatile and not in either repo. Its defaults point at the wrong bundle and a 13-inch simulator (`qa.sh:7-8`). Codex/fractal workers in other worktrees need the absolute path and `QA_BUNDLE=so.pen.herdr-gpui`.
- **R8 — Follow-Herdr Nord makes `muted` = nord3,** and `muted` is text in 414 sites. That breaks the handoff's 「nord3 は文字色に使わない」 under that theme. This is pre-existing; the iPad screens should not lean on `muted` for text there.
- **Unknown U1.** Pane focus-ring rendering was not located: grep for `focus_border|pane_border|focus.*border` in `window/`, `terminal_painter.rs` and `tab_layout/` returned 0 [未列挙]. The screen-01 research should find it.
- **Unknown U2.** Whether the window's GPUI viewport on a real iPad equals 1194×834 or excludes the 24pt status-bar safe area was not verified. `viewport::pad` handles insets (`viewport.rs:17-24`) [推測].
- **Unknown U3.** The requested canvas and HTML sources (claude.ai artifact; `design/ipad-ui` branch) were not read; they are not available on this Mac per `HANDOFF.md:15-16`.
