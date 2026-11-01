# Keyboard style (screens 07 and 08): research findings

Scope: the shortcut style setting, prefix behaviour, the Keyboard settings section, and where shortcut text is drawn.
Repo: `/Users/shin1ohno/ManagedProjects/herdr-gpui` at `feat/ipados-foundation` (HEAD 76a86890). Paths below are relative to the repo root unless they are absolute.
Tags: `[推測]` marks inference not confirmed in code.

## Summary

- iOS gets the macOS shortcut table. `default_shortcuts()` checks only Linux and Windows, and every other target, iOS included, falls through to `macos::defaults` (`crates/herdr-commands/src/shortcuts.rs:12-19`). `CTRL_SHORTCUTS` is false on iOS (`shortcuts.rs:9`).
- There is one resolved `Keymap` per window, and both styles are already inside it. For each command, the catalog's direct ⌘ defaults come first and the daemon's prefix chords follow (`crates/herdr-gpui/src/keymap.rs:271-291`). `primary()` therefore always returns the ⌘ form (`keymap.rs:322-324`).
- There is no "shortcut style" concept anywhere yet: no config key, no enum, no UI. There is also no glyph formatter (⌘⌃⇧↩). Keycaps are words (`Cmd` `T`, `crates/herdr-gpui/src/menu/settings.rs:706-730`), and GPUI's `Display` cannot be used on iOS (see Risks).
- Prefix arming happens in one place, `HerdrWindow::prefix_keystroke` (`crates/herdr-gpui/src/window/prefix.rs:62-117`), so the iPad style needs only a single gate there.
- The host's prefix reaches the keymap only when the device opts into server keybindings (`KeybindingSource::Server`, which defaults to `Local`, `crates/herdr-gpui/src/config/devices.rs:16-23`) **and** the host is Herdr 1. Herdr 2 hosts publish no profile.

## Screen 07: Settings, Keyboard section

### Existing

**Shortcut resolution per platform**
- `CommandInfo::default_shortcuts` dispatches as `cfg!(target_os = "linux")` → linux, `cfg!(target_os = "windows")` → windows, and everything else → `macos::defaults` (`crates/herdr-commands/src/shortcuts.rs:12-19`). iOS is therefore mapped to macOS. There is no `ios.rs`; the directory holds only `linux.rs`, `macos.rs`, `windows.rs` and tests (`crates/herdr-commands/src/shortcuts/`).
- `base_shortcuts()` is the macOS table, used only by tests (`shortcuts.rs:22-25`).
- `CommandInfo.name` is the `[keybindings]` key, and `message` is the Fluent name (`crates/herdr-commands/src/lib.rs:137-145`). The GUI imports the crate as `controls` (`crates/herdr-gpui/src/main.rs:189`).

**Daemon keys and the prefix default**
- `DEFAULT_PREFIX = "ctrl+b"` (`crates/herdr-gpui/src/keymap/daemon.rs:22`).
- The `ACTIONS` table holds `(herdr action, GUI target, Herdr default)` (`daemon.rs:71-261`).
- `DaemonKeys { prefixes, bindings, navigate_up, navigate_down }` (`daemon.rs:277-286`). `prefixes` is never empty, and its first entry is the one shown in labels (`daemon.rs:278-280`, `404-417`).
- `from_table` reads the local daemon `[keys]` (`daemon.rs:320-384`). `from_profile` reads a host's `server_keybindings_toml` and returns an error when it is missing, larger than 64 KiB, unparseable, or has no `[keys]` (`daemon.rs:302-316`).
- `triggers()` expands `prefix+1..9` into nine chords (`daemon.rs:421-451`). `keystroke()` maps `minus` to `-` and turns an uppercase single letter into `shift`+lowercase (`daemon.rs:481-512`).

**Keymap layering**
- The module doc describes the layering: catalog defaults, then daemon `[keys]` (prefix chords included), then GUI `[keybindings]` on top (`keymap.rs:1-8`).
- `Shortcut { label, chord: Option<Keystroke>, catalog_default }` is private (`keymap.rs:105-115`), and `Keymap.shortcuts` is parallel to `COMMANDS` (`keymap.rs:128-140`).
- A chord's label is `format!("{} {}", prefix.unparse(), keystroke.unparse())`, for example `ctrl-b c` or `ctrl-b shift-n` (`keymap.rs:262-266`).
- A prefix keystroke that a GUI binding already holds is dropped from `prefixes` (`keymap.rs:227-232`).
- When the GUI `[keybindings]` names a command, that command keeps only the direct keys listed there and loses its daemon chord (`keymap.rs:239-241`, `275-276`).
- Public reads:
  - `shortcuts(command)` returns labels only and does not expose the direct/chord kind (`keymap.rs:311-319`).
  - `primary` (`keymap.rs:322-324`)
  - `prefix_label()` returns GPUI `unparse` text, for example `ctrl-b` (`keymap.rs:341-343`).
  - `is_prefix` (`keymap.rs:346-350`)
  - `chord` (`keymap.rs:352-365`)
- Default-prefix test: `chords_follow_the_prefix` (`crates/herdr-gpui/src/keymap/tests.rs:154-160`).

**Prefix arming and pass-through (`crates/herdr-gpui/src/window/prefix.rs`)**
- An interceptor runs before any binding, per window (`prefix.rs:35-60`).
- Unarmed, `Bound::Prefix` sets `prefix_armed = true` and calls `stop_propagation()` (`prefix.rs:81-87`).
- Armed, a non-bound key is swallowed (`prefix.rs:96-101`). The prefix typed a second time returns without `stop_propagation`, so it passes through to the terminal (`prefix.rs:102-105`). A chord runs its command and closes any open menu first (`prefix.rs:106-116`).
- `bound()` checks in this order: prefix, then chord when armed, then custom command (`prefix.rs:122-135`).
- `disarm_prefix` (`prefix.rs:197-200`) runs on a keymap change (`crates/herdr-gpui/src/window/server_keys.rs:123-124`) and on window leave.
- Existing tests are inline in `prefix.rs` rather than in topic files:
  - `the_prefix_runs_chords_and_swallows_other_keys`: arm, chord, twice passes through, Escape cancels, chord from a menu text field (`prefix.rs:224-314`)
  - `a_typing_prefix_arms_everywhere_and_twice_types_it` (`prefix.rs:319-380`)
  - `custom_commands_run_only_while_connected` (`prefix.rs:384-412`)
  - Shifted-key tests sit in `crates/herdr-gpui/src/window/prefix/shifted_keys/tests.rs`.

**Server keybindings and where the host's prefix comes from**
- Wire field: `ClientShellSnapshot.server_keybindings_toml: Option<String>` (`crates/herdr-protocol/src/wire.rs:319`).
- `HerdrWindow::keymap()` returns the server keymap if one was built, otherwise `config.keybindings` (`server_keys.rs:74-76`). `sync_server_keymap` builds the server keymap only when `config.keybinding_source(endpoint) == Server` and a snapshot exists (`server_keys.rs:97-115`). A build error falls back to local keys with a warning (`server_keys.rs:36-40`).
- Error text for the host menu: `server_keybindings_error` (`server_keys.rs:86-92`).
- `KeybindingSource { #[default] Local, Server }` (`config/devices.rs:16-23`). It is read per saved device by `keybinding_source` (`devices.rs:100-104`). Local and explicit-socket endpoints always resolve to Local (`devices.rs:93-97`).
- It is saved as `[devices.<profile>] keybindings = "server"` in the GUI local file by `Config::save_device_keybindings` (`devices.rs:119-136`).
- Host menu:
  - `Action::ServerKeybindings` (`crates/herdr-gpui/src/menu/devices/host_menu.rs:19-37`) appears in `SSH_ACTIONS` only, not in cloud actions (`host_menu.rs:40-60`).
  - The toggle flips the source and saves through `save_preference` (`host_menu.rs:262-280`).
  - The checkbox row is drawn at `host_menu.rs:577-597`, and the "Using local keybindings: {error}" note at `host_menu.rs:634-656`.
  - Strings: `host-menu-server-keybindings = Use server keybindings`, `host-menu-local-keybindings` (`crates/herdr-gpui/locales/en-US/host-menu.ftl:6,12`).
- Local daemon `[keys]` (used when the source is Local) are read from `daemon_config_path` (`crates/herdr-gpui/src/config/daemon.rs:17-43`, `56-71`) and built into `config.keybindings` (`crates/herdr-gpui/src/config.rs:902-905`).

**Settings window sections, catalog, search, pages**
- `Section` enum (`crates/herdr-gpui/src/settings_window/section.rs:7-25`). User-visible order `Section::ALL` (`section.rs:72-88`) is not sorted: General, Theme, Sidebar, Indicators, Fonts, StatusBar, Tabs, Code, Notifications, Integrations, Plugins, [CloudDevices], RemoteControl, Configuration.
- `message()` maps sections to `settings-section-*` (`section.rs:95-113`), with the strings in `crates/herdr-gpui/locales/en-US/settings.ftl:72-85` (Tabs is `settings-section-tabs-and-panes = Tabs & panes`, `settings.ftl:78`).
- Other per-section tables: `lead()` (`section.rs:115-132`), `group()` with Tabs in `Group::Behaviour` (`section.rs:136-149`), `icon()` (`section.rs:151-169`), and `nav_item()`, which counts over `ALL` with no availability filter (`section.rs:178-192`).
- The section list renders every `Section::ALL` entry with no runtime filter (`crates/herdr-gpui/src/settings_window/navigation.rs:70-88`). The only gating precedent is the compile-time `#[cfg(feature = "cloud")]`.
- Page dispatch:
  - `render_controls` match (`crates/herdr-gpui/src/settings_window/controls.rs:294-307`), which is exhaustive.
  - The window's render (`crates/herdr-gpui/src/settings_window.rs:776-786`) uses a `_` fallback to `render_controls`.
  - Page modules are listed in `crates/herdr-gpui/src/settings_window/pages.rs:3-16`.
- Search catalog:
  - `Entry { section, id, title, description, availability }` and `Availability::Ipad` ("Only on iPadOS") (`crates/herdr-gpui/src/settings_window/catalog.rs:9-30`).
  - Precedent for iPad-only rows: `general-ipad-key*` `.only(Availability::Ipad)` (`catalog.rs:80-92`). Tabs rows are at `catalog.rs:403-446`.
  - `available()` maps `Ipad` to `crate::ipad_ui::active()` (`crates/herdr-gpui/src/settings_window/search.rs:50-58`).
- Opening at a given section: `remote_control::open` calls `settings_window::open`, then defers `select_section(Section::RemoteControl)` (`crates/herdr-gpui/src/settings_window/pages/remote_control.rs:42-56`). The plain `open` starts on the default section, `Theme` (`settings_window.rs:79-90`, `350`; `section.rs:10-11`). `select_section` is at `settings_window.rs:706-763`.
- Reading the main window from Settings: the field `source: WeakEntity<HerdrWindow>` (`settings_window.rs:181`), with the pattern `source.read(cx)` (`crates/herdr-gpui/src/settings_window/cloud_devices.rs:238`; `crates/herdr-gpui/src/settings_window/remote_history.rs:172-176`).
- Page-building precedents:
  - Segmented control bound to a GUI preference: Tabs → Layout (`crates/herdr-gpui/src/settings_window/pages/tabs.rs:34-46`).
  - Radio: `indicators::radio` (`crates/herdr-gpui/src/settings_window/pages/indicators.rs:156-168`).
  - iPad-only block: `#[cfg(any(target_os = "ios", test))]` combined with `.when(crate::ipad_ui::active(), …)` (`crates/herdr-gpui/src/settings_window/pages/general.rs:61-66`).
- Icon `assets/icons/keyboard.svg` exists. The status bar already uses it (`crates/herdr-gpui/src/window/status_bar.rs:139`).

**Where GUI settings are stored**
- GUI config file: `<config_root>/herdr/config-gpui.toml` (`Config::path`, `config.rs:621-623`). User overrides go in `config-gpui.local.toml` (`config.rs:637-639`).
- The settings-file struct keeps its fields sorted between keep-sorted markers (`config.rs:515-563`). `[keybindings]` is `BTreeMap<String, Binding>` (`config.rs:536`).
- `[keybindings]` cannot carry a style key. Every key must name a command, and an unknown key fails with `Error::UnknownKeybinding` (`keymap.rs:174-178`; `crates/herdr-gpui/src/keymap/error.rs:52`).
- GUI-only preference writes go through `Preference` and `save_preference_path` (`crates/herdr-gpui/src/config/preferences.rs:6-45`, `106-111`). The `TabLayout` example writes the top-level key `tab_layout` (`preferences.rs:37`, `184`).
- Enum-with-config-name precedent: the `tab_layouts!` macro (variant, config name, Fluent label, `ALL`) in `crates/herdr-gpui/src/config/tab_layout.rs:13-60`.
- `herdr_settings::Settings` is **Herdr's shared `config.toml`** (the TUI's file). It is loaded from `daemon_config_path` (`crates/herdr-gpui/src/herdr_settings.rs:1-8`, `259-286`, `409-410`). It is the wrong home for a GUI-only iPad style.

**iPad gate**
- `ipad_ui::active()` is `cfg!(target_os = "ios")` in builds. In tests it is **true unless `ipad_ui::desktop()` is held** (`crates/herdr-gpui/src/ipad_ui.rs:14-24`, `28-42`).

### Gaps (what does not exist yet)

1. No shortcut-style setting: no enum, no `Config` field, no settings-file key, and no `Preference` variant.
2. No Keymap API that returns "the first direct shortcut" separately from "the first prefix chord" for a command. The `chord` field is private to `keymap.rs` (`keymap.rs:105-115`), so the selector has to live in `keymap.rs`.
3. No glyph formatter for `cmd-shift-enter` → `⌘⇧↩` or `ctrl-b shift-n` → `⌃B ⇧N`. There is also no range aggregation of `TabNumber(1..9)` into `⌘1–9` / `⌃B 1–9`.
4. No `Section::Keyboard`, no `settings-section-keyboard` string (settings.ftl contains no "keyboard": 0 matches, positive control 16 `settings-section` matches), and no `keyboard-*` catalog ids.
5. No runtime filter that hides a section on the desktop. `navigation.rs:70-88` and `nav_item` (`section.rs:178-192`) iterate every section.
6. No arming gate in `prefix.rs:81-87` for the iPad style.
7. No "Prefix key" row. The value would be `keymap().prefix_label()` (`keymap.rs:341-343`) formatted as glyphs.
8. No settings-window copy of the "Use server keybindings" switch. The host-menu logic (`host_menu.rs:262-280`) is tied to `self.menu.host` and has to be reshaped so both places can call it.
9. No "open Settings at Keyboard" entry point. The `remote_control::open` pattern applies.

### Design notes for the implementer (from the code; choices marked)

- **Storage:** add a new top-level GUI key, for example `shortcut_style = "ipad" | "herdr-prefix"`, in the sorted settings struct (`config.rs:515-563`). Add a `Preference::ShortcutStyle` arm (`preferences.rs:6`, `120-210`) and an enum built with a `tab_layouts!`-style macro in a new `config/shortcut_style.rs`. `config.rs` is 986 lines, so adding a field, a struct field and an assignment (about 4-6 lines) is close to the 1,000-line limit; split if it crosses.
- **Default:** [推測] resolve the default as `Ipad` only under `cfg!(target_os = "ios")` and `HerdrPrefix` otherwise. Using `ipad_ui::active()` instead would flip the default inside headless tests (`ipad_ui.rs:14-24`) and break the existing `prefix.rs` tests (`prefix.rs:224-380`), which run on the iPad side.
- **Prefix gate:** in `prefix_keystroke`'s unarmed branch (`prefix.rs:81-87`), when `ipad_ui::active() && style == Ipad`, treat `Bound::Prefix` as unbound and do not call `stop_propagation`. The key then reaches the terminal. The armed-indicator (`status_bar.rs:63-77`) then never shows. Prefixed custom commands (`prefix.rs:127-134`) become unreachable in the iPad style; say so in `docs/ios.md`.
- **Keymap selector:** add something like `Keymap::styled(command, Style) -> Option<&str>` that picks the first `chord.is_none()` shortcut (iPad) or the first `chord.is_some()` shortcut (Herdr prefix) (`keymap.rs:311-324`). `keymap.rs` is 521 lines, so there is room.
- **Formatter:** a new pure module, for example `keymap/glyphs.rs`, tested in `keymap/tests/<topic>.rs` (`ctrl`→⌃, `alt`→⌥, `shift`→⇧, `cmd`→⌘, `enter`→↩, `-` kept, letters uppercased). Use it only behind `ipad_ui::active()`, because the desktop keeps word keycaps (`menu/settings.rs:706-730`).
- **Section:** insert `Keyboard` after `Tabs` in `ALL` (`section.rs:79`), in group `Behaviour`, with icon `icons/keyboard.svg`. Filter it from the nav list and `nav_item` with `ipad_ui::active()`, or gate it with `#[cfg(any(target_os = "ios", test))]` plus the runtime filter. Mark catalog rows `.only(Availability::Ipad)`.
- **Tests to update:** `tests/search.rs:41-62` (exhaustive section → id-prefix match; add `Section::Keyboard => "keyboard-"`), `tests/search.rs:66-90` (renders every section on the iPad side), `tests/section_order.rs:6-18` (group order still holds if Keyboard is in Behaviour), and `tests/navigation_scroll.rs:16-37` (the last section stays Configuration).

## Screen 08: Main window with the Herdr prefix style selected

### Existing

**SHORTCUTS table (handoff `HANDOFF.md:89`): every value and where it resolves from**

iPad column = `crates/herdr-commands/src/shortcuts/macos.rs`. Herdr prefix column = `crates/herdr-gpui/src/keymap/daemon.rs` `ACTIONS` with prefix `ctrl+b` (`daemon.rs:22`).

| Action (handoff) | Command | iPad (spec) → resolves from | Herdr prefix (spec) → resolves from |
|---|---|---|---|
| New tab | `Tab` | `⌘T` → `cmd-t` `macos.rs:10` | `⌃B c` → `new_tab` `prefix+c` `daemon.rs:139` |
| Previous / next tab | `PreviousTab`/`NextTab` | `⌘⇧[` `⌘⇧]` → `cmd-shift-[` `macos.rs:14`, `cmd-shift-]` `macos.rs:13` | `⌃B p` `⌃B n` → `daemon.rs:140-144`, `daemon.rs:145` |
| Tab 1–9 | `TabNumber(1..9)` | `⌘1–9` → `macos.rs:33-41` | `⌃B 1–9` → `switch_tab` `prefix+1..9` `daemon.rs:157` (expanded by `daemon.rs:434-441`) |
| Split right / down | `SplitRight`/`SplitDown` | `⌘D` `⌘⇧D` → `macos.rs:11`, `macos.rs:12` | `⌃B v` `⌃B -` → `split_vertical` `prefix+v` `daemon.rs:226-230`, `split_horizontal` `prefix+minus` `daemon.rs:231-235` |
| Close pane | `ClosePane` | `⌘W` → `macos.rs:30` | `⌃B x` → `daemon.rs:236-240` |
| Zoom pane | `Zoom` | `⌘⇧↩` → `cmd-shift-enter` `macos.rs:23` | `⌃B z` → `daemon.rs:242` |
| Copy mode | `CopyMode` | `⌘⇧C` → `macos.rs:28` | `⌃B [` → `daemon.rs:175` |
| Spaces | `WorkspacePicker` | `⌘P` → `macos.rs:54` | `⌃B w` → `workspace_picker` `daemon.rs:92-96` (a second chord `⌃B g` from `goto`, `daemon.rs:97-101`) |
| New space | `Workspace` | `⌘⇧N` → `cmd-shift-n` `macos.rs:8` | `⌃B ⇧N` → `new_workspace` `prefix+shift+n` `daemon.rs:75-79` |
| Settings | `Settings` | `⌘,` → `macos.rs:51` | `⌃B s` → `daemon.rs:74` |
| Keyboard shortcuts | `Keybinds` | `⌘/` → `macos.rs:52` | `⌃B ?` → `help` `prefix+?` `daemon.rs:73` |
| Sidebar | `ToggleSidebar` | `⌘B` → `macos.rs:42` | spec says `—`, **but it resolves to `⌃B b`** from `toggle_sidebar` `prefix+b` (`daemon.rs:256-260`) |

Legend values:
- Screen 01 (iPad style): `⌘T` Tab (`macos.rs:10`), `⌘D` Split (`macos.rs:11`), `⌘B` Sidebar (`macos.rs:42`), `⌘⇧P` Commands → `Palette` `cmd-shift-p` (`macos.rs:55`). `Palette` has no `ACTIONS` row, so it has no prefix chord.
- Screen 08 (prefix style): `⌃B c` Tab, `⌃B v` Split, `⌃B x` Close pane, `⌃B ?` Help (`daemon.rs:139`, `226-230`, `236-240`, `73`).
- The legend's **command set** differs per style, not only its glyphs.

**Where shortcut text is drawn today**
- Command palette: each command row's detail is `self.keymap().primary(info.command)` (`crates/herdr-gpui/src/palette.rs:735`), always the ⌘ form. The detail string feeds palette search and highlight indices (`palette.rs:186-200`) and is drawn as raw text (`crates/herdr-gpui/src/palette/render.rs:177-181`).
- Status bar:
  - The `status-keybinds` button shows a keyboard glyph and the label `Shortcuts` and opens the keybinds page, not Settings (`window/status_bar.rs:136-147`; `crates/herdr-gpui/locales/en-US/status-bar.ftl:18`).
  - Its visibility comes from `StatusBar.shortcuts` (`crates/herdr-gpui/src/config/status_bar.rs:84`, `98`, `113`, `126`).
  - While armed, the prefix chip shows `keymap().prefix_label()` (`window/status_bar.rs:63-77`).
- Keybinds page: shows every shortcut per command as word keycaps (`menu/settings.rs:314-317`, `525-534`, `706-730`). Not in the handoff's list of views that follow the style.
- Tab labels: **no shortcut hint exists**. `crates/herdr-gpui/src/window/tab_strip.rs` (933 lines) never calls `keymap()`. `TabNumber` appears only in `window/commands.rs:392`.
- Sidebar SPACES heading kbd and New space kbd: **do not exist**. `sidebar/render.rs:786` and `sidebar/rail.rs:573` run `Command::Workspace` without drawing a hint.
- Control socket `keybinds` returns `keymap.shortcuts()` (`crates/herdr-gpui/src/control/server/settings/keys.rs:37-46`). The control socket is excluded on iOS (`crates/herdr-gpui/src/control.rs:11-15`), so an iPad-only setting needs no CLI command [推測: the AGENTS.md CLI rule does not apply to an iOS-only setting].

### Gaps

1. The tab kbd (`⌘1`…`⌘9` / `⌃B 1`…), SPACES kbd (`⌘P` / `⌃B w`), and New space kbd (`⌘⇧N` / `⌃B ⇧N`) are not drawn anywhere. They are screen 01 work, but they must call the same style-aware helper (selector plus formatter) so that screen 08 only changes the style.
2. The status bar legend (four chords and their labels) does not exist. It needs new Fluent keys (`status-bar.ftl` has none for Tab/Split/Sidebar/Commands/Close pane/Help), a per-style command list, and a click handler that opens Settings at Keyboard instead of `open_keybinds`.
3. The palette detail does not follow a style. Changing it to glyphs behind `ipad_ui::active()` must keep the raw text searchable (`palette.rs:186-200`). For example, put the raw `cmd-…` text in keywords.
4. The prefix glyph shown (`⌃A` in the user's environment) follows the host only when server keybindings are on (see Risks 1).

## Risks and unknowns

1. **The host prefix is not shown by default.** `KeybindingSource` defaults to `Local` (`devices.rs:16-23`), so the iPad keymap uses the local daemon file. Inside the app sandbox that file is [推測] absent, which gives `ctrl+b` (`daemon.rs:22`, `config/daemon.rs:56-65`). The user's `⌃A` (`HANDOFF.md:94`) appears only after "Use server keybindings" is turned on for that device.
2. **Herdr 2 hosts publish no profile.** `server_keybindings_toml` exists only on the Herdr 1 `ClientShellSnapshot` (`wire.rs:319`). Core connections do not "manufacture a Herdr 1 snapshot" (`docs/ios.md:497-500`), and herdr-core-client has no keybinding code (grep for `keybind|prefix` in `crates/herdr-core-client/src` and `crates/herdr-backend/src` found only `tests/live.rs:26` `.prefix("hc")`). So for a "Herdr 2" host like the mock's `studio-mac`, the "From studio-mac's Herdr config" row cannot be truthful. It needs different wording or a fallback.
3. **The spec's Sidebar row conflicts with resolution.** The handoff wants `—` in the Herdr column, but the rule "draw from the resolution result" yields `⌃B b` (`daemon.rs:256-260`). Decision needed.
4. **Modifier order.** The mock writes `⌘⇧[`, `⌘⇧N`, `⌘⇧↩` (⌘ before ⇧). Apple's convention is ⌃⌥⇧⌘ (`⇧⌘N`), and that is also what GPUI's macOS `Display` emits. Decision needed.
5. **GPUI `Display` is unusable on iOS.** `display_modifiers` writes glyphs only under `cfg(target_os = "macos")`, writes `ctrl-`/`shift-` text elsewhere, and writes **nothing** for `platform` on iOS. Control shows as `^`, not `⌃`, even on macOS, and `enter` has no glyph (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.6/src/platform/keystroke.rs:432-437`, `681-711`, `717-745`). An in-repo formatter is required.
6. **Test-side default.** `ipad_ui::active()` is true in headless tests (`ipad_ui.rs:14-24`). A style default keyed on it would change the behaviour of the existing `prefix.rs` tests (`prefix.rs:224-380`, `the_prefix_runs_chords_and_swallows_other_keys`, `a_typing_prefix_arms_everywhere_and_twice_types_it`).
7. **Every new section must be gated.** The section list has no runtime filter (`navigation.rs:70-88`; `section.rs:178-192`). An ungated `Section::Keyboard` would appear on the desktop and change desktop navigation and tests.
8. **GUI overrides remove chords.** A command named in GUI `[keybindings]` loses its daemon chord (`keymap.rs:239-241`, `275-276`), so its Herdr column shows `—`. The same `[keybindings]` can also steal the prefix key itself (`keymap.rs:227-232`), and then `prefix_label()` is `None`.
9. **Files near the 1,000-line limit.** `config.rs` 986, `window/tab_strip.rs` 933, `menu/devices/host_menu.rs` 916, `palette.rs` 896, `settings_window.rs` 880. Adding the switch to Settings by extending `host_menu.rs` is risky; extract the toggle logic instead.
10. **Unknown: `ServerKeybindings` cannot apply to cloud or local endpoints.** Cloud devices lack the action (`host_menu.rs:52-60`), and local endpoints are never saved devices (`devices.rs:93-97`). The Settings switch therefore needs a disabled state for those cases. The wording for that state is not specified.
11. **Unknown: hardware-keyboard delivery on iPad.** The local `herdr-uikit` clone maps `UIKeyModifierFlags::Control`→`control` and `Command`→`platform` (`/Users/shin1ohno/ManagedProjects/herdr-uikit/src/window/convert.rs:24,27`). That read is from the local clone, which was not checked against pinned rev 00a8dba. Whether iPadOS delivers ⌃B to the app, or a system shortcut takes it first, has not been verified on a device.
12. **Note:** writing these findings was briefly blocked by a plan-mode interval mid-run. After plan mode ended, this file was written at the path the task specified.
