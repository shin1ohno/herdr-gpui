# Research: screens 04 (Add Device sheet) and 05 (agent chat)

Repo: `/Users/shin1ohno/ManagedProjects/herdr-gpui`, branch `feat/ipados-foundation`, HEAD `76a86890`.
Paths below are repo-relative unless absolute. `[推測]` = inference not confirmed in code.

## 0. Cross-cutting facts that decide how "iPad only" can be done

- `crate::ipad_ui::active()` is `cfg!(target_os = "ios")` in normal builds but **`true` in every test** unless the test holds `crate::ipad_ui::desktop()` (`crates/herdr-gpui/src/ipad_ui.rs:14-24`, `:29-42`). Only 3 tests take the desktop guard today (`core_terminals/tests/reconnect.rs:128`, `settings_window/tests/ipad_key.rs:116`, `sidebar/rail/tests/touch_context.rs:55`). So any change gated by `active()` is seen by every existing headless test, including tests written with desktop geometry in mind.
- `cfg!(target_os = "ios")` / `#[cfg(target_os = "ios")]` paths are never run by headless tests (they compile for the macOS host). `just test-ios` runs only the sibling `herdr-uikit` tests plus Python tooling (`docs/ios.md:336`), and `just lint-ios` is clippy only (`docs/ios.md:333`). A cfg(ios)-only render path is verified only by clippy and simulator screenshots.
- Pattern that works for both: a render function compiled on all targets and called from tests directly, e.g. `render_ipad_host_key` (`menu/devices/add_device/host_key.rs:9`), tested in `add_device/host_key/tests/wrapping.rs:22`, but selected at runtime by `cfg!(target_os = "ios")` (`menu/devices/add_device.rs:370`).
- The iPad app is built with `--no-default-features` (`justfile:427`, `docs/ios.md:335`), so the `cloud` feature is off on iPad. `rail_sources()` then returns only `[Source::Host]` (`menu/devices/rail.rs:141-154`), and a lone source draws no rail (`rail.rs:316-321`). Headless tests build with default features (`crates/herdr-gpui/Cargo.toml:97`: all cloud providers), so the rail can appear in tests.
- Shared radii: only `corners::PANEL = 12`, `CONTROL = 8`, `SMALL = 4` (`crates/herdr-gpui/src/config.rs:162-166`). There is no 18 / 22 radius or 44/48/52/56/60 size constant anywhere yet. The handoff's PR 1 ("iPad size constants") must land first or these PRs add their own.
- Theme roles in use: `theme.surface` (panel bg), `theme.active` (borders, neutral buttons), `theme.primary()` (`config/theme.rs:452`, accent fill used by chat send), `crate::menu::accent(theme)` (blue tint, `menu/colors.rs:22`), `crate::menu::danger` (`menu/colors.rs:37`), chat `Hues::of(theme).yellow = theme.ink(palette[3])` (`chat/view.rs:107-123`). Menus have no yellow helper; `crate::menu::colors::tint(theme, 3)` (`menu/colors.rs:10`) is the closest [推測: whether `tint` is reachable as `pub(crate)` from devices; it is declared `pub(crate) fn tint`].

## Screen 04 — Add Device sheet

### 04.1 Where the dialog is drawn (frame / overlay / backdrop)

| Piece | Location | Current behaviour |
| --- | --- | --- |
| Overlay + dim | `menu/chrome.rs:850-872` (`render_menu_layer` → `menu-overlay`) | Full-window overlay; for dialogs (`dims`) it centres the panel, pads by safe area/keyboard via `crate::window::viewport::pad` (`window/viewport.rs:17-24`), and paints `super::appear::backdrop(theme.background, appear.backdrop(now))`. Tap outside dismisses (`chrome.rs:887-893`). |
| Dim colour | `menu/appear.rs:31-38` | `DIM = 176` (≈69%) alpha over **`theme.background`**, not black. Shared by every dimming dialog. Spec wants 66% black. |
| Panel width | `menu/chrome.rs:437-459` (`add_device` branch `:446-447`) | `min(viewport - 32, rail::WIDTH(640) * font.size/12)`. Spec 620pt. |
| Panel height | `menu/chrome.rs:510-513` | `rail::HEIGHT(620) * font.size/12`, capped at `viewport - 32`. Constants at `menu/devices/rail.rs:12-18` (`COMPACT_BELOW 560`, `WIDTH 640`, `HEIGHT 620`). |
| Panel chrome | `menu/chrome.rs:550-560` (flex col, overflow hidden, shadow) and `:564-567` | `rounded(corners::PANEL)` (12), `border_1` `theme.active`, `bg theme.surface`. Shared by **all** menus/dialogs. Spec radius 18. |
| Entry motion | `menu/chrome.rs:575-578`, `menu/appear.rs:26-29` | Dialog rises 10px and fades. |
| Page dispatch | `menu/chrome.rs:634-635` | `Page::AddDevice` → `render_add_device(compact_rail, cx)`. |
| Frame (header + rail + pane) | `menu/devices/rail.rs:277-340` `add_device_frame` | Header `device-setup-header` (`:286-313`): `p 16`, bottom rule, title `add-device-title` (`:300`, size `ui.size*1.35`, semibold), right-side **Close** button `add-device-close` (`:302-312`, `px_2 py_1`, ~ <30pt tall). Rail only when >1 source (`:317-321`). Frame is shared by SSH (`add_device.rs:257`), Catalog (`catalog.rs:108`), Coder (`coder.rs:603`, `:773`), Cloud (`rail.rs:524`), WSL (`wsl.rs:213`). |

iPad-only hooks already present in `render_add_device`: frame `max_h_full().overflow_hidden()` (`add_device.rs:264-269`), body `w_full().whitespace_normal()` (`:292-294`), footer `flex_wrap` (`:417`), pane `max_h_full()` (`:496-498`), host-key-code section (`:343-345`).

### 04.2 Body, header and footer of the SSH form (`menu/devices/add_device.rs`, 986 lines)

- `Setup` state `:29-59` (fields `[Entity<SearchInput>; 3]` `:32`, `device_key` cfg(ios) `:57-58`); `Step` enum `:63-92`.
- `open_add_device` `:113-187`: fields get `config.ui` font (`:117`); target placeholder switches by `cfg!(target_os = "ios")` to `add-device-ipad-target-placeholder` (`:119-128`); session placeholder literal `"default"` (`:131`, `// i18n: data`). Starts discovery `:184` and on iOS loads the device key `:185-186`.
- Keys: `add_device_key` `:191-248` (Enter submits `:243`, Escape dismisses `:244`; Enter never trusts a key).
- `render_add_device` `:250-505`:
  - Body `device-setup-body` `:284-306` (`p 16`, `gap 12`, scrolls `overflow_y_scroll`), intro `add-device-ipad-intro` vs `add-device-intro` by `cfg!` (`:300-306`).
  - Fields loop `:307-333`: each field is `label` above `SearchInput` in a column (`gap 6`); the nearby list is inserted right after field 0 while `Step::Form` (`:327-332`). **All three fields are stacked**; spec wants Label + Remote session in 2 columns.
  - Desktop host-key checkbox `render_host_key_choice` only `cfg(not(ios))` (`:335-338`); iOS adds `render_device_key` (`:339-342`); `active()` adds `render_device_host_key_code` (`:343-345`).
  - Error line `:346-353` (`crate::menu::danger`).
  - Status line per step `:354-394`; the `TrustKey` arm also appends the host-key card: `render_ipad_host_key` when `cfg!(ios)`, else `render_host_key` (`:369-376`).
  - Button factory `:395-409`: `p 8`, `rounded CONTROL`, `bg theme.active` for every button (no accent/primary variant). Hit area ≈ text line + 16 (< 44pt) [推測 from `p(px(8.))` and the default line height].
  - Footer `device-setup-footer` `:410-419` (`p 16`, top rule, `justify_end`); Cancel `device-setup-cancel` `:420-421`.
  - Footer actions by step `:422-491`: iOS Confirm(core) = Cancel / Check again / Add Herdr 2 host (`:423-441`); iOS Confirm = Cancel / Check again (`:442-454`); desktop Confirm = Cancel / offer action (`:455-458`); AwaitingLocalNetwork = settings + Cancel (`:459-461`); KeyCodeRefused = Cancel (`:462`); **TrustKey = Cancel + Trust key** `device-setup-trust` (`:463-471`); Form and in-flight = single `device-setup-submit` "Add device" / "Checking…", disabled while a host key code awaits confirmation (`:472-490`). **In `Step::Form` there is no Cancel button**; dismissal is the header Close or Escape.
- The file is 986/1000 lines (`wc -l`), so any iPad sheet work must move code out first (e.g. a new `add_device/sheet.rs` or `add_device/render.rs` holding body/footer builders) [推測 module name]; `AGENTS.md` forbids exceeding 1,000 lines.

### 04.3 Nearby devices (Bonjour `_ssh._tcp`, Tailscale, SSH config)

- Render: `menu/devices/discover/render.rs:17-164` `render_device_suggestions`.
  - Status text: waiting (`devices-discover-ipad-waiting` on iOS `:26-31`), searching (`:32-36`), else `devices-discover-found` with `count = completion.unsaved` (`:37-38`).
  - Header row `:40-53`: heading `devices-discover-heading` ("Nearby devices"), status flex_1 muted, then "Search again" `device-discovery-again` when idle (`:54-68`). Spec places the found text to the right of an uppercase "NEARBY DEVICES" heading; current order is heading, status, Search again.
  - Busy bar `progress::bar` with `crate::menu::accent` (`:76-83`).
  - Rows: height `row = ceil(ui.size*1.6 + 12)` (`:86`), list `max_h(row * MAX_VISIBLE_ROWS)` with `MAX_VISIBLE_ROWS = 4.5` (`:14`, `:87-95`). Each row `device-suggestion-{i}`: name (max 220, truncate), target (subtext), sources (muted) (`:103-143`). Highlight = `bg theme.active` (`:115-117`); **no check mark**. Spec: 56pt rows, accent check on the selected row. `assets/icons/check.svg` exists.
  - Click → `use_device_suggestion` `:168-192` fills target + label.
- Highlight semantics: `discover/complete.rs:113-125` (`highlight`, `chosen`), computed in `device_completion` `:143-199`: highlighted row = arrow choice for this text, else row whose fill equals the field, else first row the text prefixes. So "selected" = `completion.highlight` (the row whose target is in the field) [推測: the spec's "selected" maps to this highlight].

### 04.4 Host key approval card and Trust key

- iPad card body: `menu/devices/add_device/host_key.rs:9-75` `render_ipad_host_key(target, &[HostKeyFingerprint]) -> Div` (selector `device-setup-host-key`): text `add-device-key-unknown` (`:57-63`), fingerprint box `device-setup-fingerprints` with `bg theme.active`, `rounded CONTROL`, terminal font (`:11-45`), check hint `key_check` → `add-device-ipad-key-check` (`:65-74`, `:91-99`). **No border, no buttons inside**; spec wants a nord13 (yellow) bordered card with Cancel / Trust key (44pt, Trust key accent) inside it.
- Desktop card: `menu/devices/host_key.rs:164-212` `render_host_key` (adds `known_hosts` path, `add-device-key-check`).
- Action: `trust_host_key` `menu/devices/host_key.rs:131`; fetch `fetch_host_key` `:18`, `host_key_fetched` `:41`. Buttons wired in the footer at `add_device.rs:463-471`.
- Host key **code** section (iPad-only via `active()`): `add_device/host_key_code.rs:313-393` `render_device_host_key_code` — Accept/Dismiss/Clear/Use pasted buttons, each `p 8`, `bg theme.active` (`:324-333`). Field init `:72-86`; code summary `:437-457`.
- Tests: `add_device/host_key/tests/{copy,labels,wrapping}.rs` call `render_ipad_host_key` directly.

### 04.5 authorized_keys line and Copy

- `menu/devices/add_device/device_key.rs` (whole module `#[cfg(target_os = "ios")]`, declared at `add_device.rs:6-7`):
  - `DeviceKey` state `:13-21`; `load_device_key` `:25-47` (Keychain read off the UI thread via `settings_window::device_key::load`).
  - `render_device_key` `:49-120`: lead text `add-device-ipad-key` (`:61`), loading/failed states (`:63-76`), the line in a mono box `device-setup-device-key-line` (`:80-88`), Copy button `device-setup-device-key-copy` right-aligned, `p 6`, label `add-device-ipad-key-copy` "Copy public key" (`:89-115`), flash `add-device-ipad-key-copied` (`:110-113`).
  - Because the module is cfg(ios), **no headless test renders it**; `Setup::device_key` and the test fixture field are cfg(ios) too (`add_device/tests.rs:28-29`).
- Spec row text "Add this iPad's key to `~/.ssh/authorized_keys` on the computer." does not exist; the nearest key is `add-device-ipad-key` ("Enable Remote Login and install Herdr on the computer, then add this line to its ~/.ssh/authorized_keys:", `locales/en-US/add-device.ftl:83`). Spec button "Copy key" vs existing "Copy public key" (`add-device.ftl:86`).

### 04.6 Fluent keys (existing)

`crates/herdr-gpui/locales/en-US/add-device.ftl`: `add-device-title` `:3`, `add-device-close` `:4`, `add-device-target` `:26`, `add-device-label` `:27` ("Label (optional)"), `add-device-session` `:28` ("Remote session (optional)"), `add-device-cancel` `:35`, `add-device-trust-key` `:36`, `add-device-submit` `:37` ("Add device"), `add-device-submit-checking` `:38`, `add-device-key-unknown` `:54`, `add-device-ipad-key-check` `:56`, `add-device-ipad-key` `:83`, `add-device-ipad-key-copy` `:86`, `add-device-ipad-key-copied` `:87`, `add-device-ipad-target-placeholder` `:88`, `add-device-ipad-intro` `:89`, `add-device-check-again` `:100`, `add-device-core-add` `:99`.
`crates/herdr-gpui/locales/en-US/devices-discover.ftl`: `devices-discover-heading` `:3` ("Nearby devices"), `devices-discover-ipad-waiting` `:5`, `devices-discover-searching` `:8`, `devices-discover-found` `:9-13` ("Search finished. Found { $count } devices."), `devices-discover-again` `:14`.
Uppercase headings are written uppercase in the `.ftl` (precedent `add-device-cloud-heading = CLOUD`, `add-device.ftl:9`), not transformed in code; "NEARBY DEVICES" therefore needs a new key. Only `en-US` exists (`crates/herdr-gpui/locales/`); `just check-i18n` (`justfile:237-239`) is part of `just ci` (`justfile:296`).

### 04.7 Gap list vs spec 04

1. Width 640·scale → 620; radius 12 → 18 (shared panel chrome `chrome.rs:564`).
2. Backdrop `theme.background` @ 69% → 66% black (shared `appear.rs:32-38`); conflicts with "no hard-coded hex" rule (see Unknowns).
3. Header: Close-only (`rail.rs:286-313`) → 60pt with Cancel (left) / title / primary action (right). The primary action is step-dependent and lives in `add_device.rs:422-491`; the header is built in the shared `add_device_frame`.
4. Footer-only actions, `bg theme.active`, no accent button, < 44pt hit areas.
5. Fields: stacked, `config.ui` font, SearchInput frame height = line height + `py_1` (`search_input.rs:638`, `:664-673`) → 48pt, target in terminal (mono) font, Label + Session in two columns.
6. Nearby: heading not uppercase; status order; 56pt rows; accent check on highlighted row.
7. Host key card: no yellow border; Cancel / Trust key not inside the card; Trust key not accent; 44pt.
8. authorized_keys row: wording differs; Copy is `p 6` (< 44pt).

### 04.8 How to change only on iPad (design notes)

- Add an iPad sheet frame beside `add_device_frame` (new function, e.g. `ipad_sheet_frame`, in a new file under `menu/devices/`) that takes the step's primary action and Cancel as elements, and call it from `render_add_device` when `crate::ipad_ui::active()`; keep `add_device_frame` unchanged for desktop and for cloud/WSL/Coder/Catalog [推測 design]. Keep the selectors `device-setup-header`, `device-setup-submit`, `device-setup-cancel`, `device-setup-body` so `add_device/tests/visible_viewport.rs:28-35`, `:62-67` keep finding them.
- Panel width/radius: add an `add_device && crate::ipad_ui::active()` branch in `chrome.rs:437-459` and at `:564`. `chrome.rs` is 939 lines (room ≈ 60).
- Backdrop: parameterise `appear::backdrop` or add a sibling for the iPad sheet; do not change `DIM` globally (all dialogs share it).
- Host key card: move Cancel / Trust key into `render_ipad_host_key` (or a wrapper) while keeping `trust_host_key` as the only trusting path (comment at `add_device.rs:463`). Note `render_ipad_host_key` is chosen by `cfg!(ios)` at `add_device.rs:370`, so in headless tests the desktop `render_host_key` is drawn; an `active()` switch there would change what existing tests render.
- `add_device.rs` must be split before any addition (986 lines).

## Screen 05 — Agent chat

### 05.1 How a chat becomes a tab (title, icon)

- Starting a chat creates a Herdr tab via `tab.create` with `"label": agent.display_name()` (`chat/start.rs:124-153`, label `:136`; request `:248`). Display names: Claude, Codex, Copilot, Gemini, OpenCode, Pi (`crates/herdr-chat-protocol/src/agents.rs:47-55`).
- After the first prompt, `herdr-acp` renames the tab to `"{agent}: {title}"`, ≤ 40 chars (`crates/herdr-acp/src/herdr/naming.rs:17-21`, call at `crates/herdr-acp/src/herdr.rs:190-193`), unless the user renamed it (`naming.rs:26-30`). So the tab title is not just the agent name once a prompt was sent.
- The chat view is drawn over the pane that runs `herdr-acp` (`chat.rs:1-15`; `chat/view.rs:158-200` `render_chats`, `:253-426` `render_chat`).
- Tab mark: `herdr_mark` (`window/tab_strip.rs:914-930`) picks the tab's focused agent and maps **`agent.agent`** through `AgentIcon::from_identity`. `herdr-acp` reports `agent = "acp"` (`crates/herdr-acp/src/herdr.rs:23-25`, `:168-169`) and puts the real name in `display_agent` (`herdr.rs:169`, `:187`; comment `:184-186` "or clients fall back to `acp` and lose the agent's logo"). `"acp"` is not in the icon table (`crates/herdr-gpui/src/icons.rs:51-76`), so the mark resolves to `AgentIcon::Generic`. The same `agent.agent` mapping is used by the sidebar agent row (`sidebar/render/agent_rows.rs:153`), while names use `display_agent` first (`sidebar/agents.rs:166-173`).
- The chat header already resolves the harness icon from the display name: `chat::view::agent_icon` (`chat/view.rs:101-104`), drawn at 15px (`chat/view.rs:606-612`).
- Tab layouts: default `TabLayoutMode::Classic` (`config/tab_layout.rs:53-54`) draws only the status dot via `lead_mark` (`tab_layout.rs:355-378`, `tab_layout/classic.rs:113`), **no agent icon**. Cards/Compact/Curved/Outline/Pills draw `kind_icon` (`tab_layout.rs:382-393`) at 11-13px (`tab_layout/{cards.rs:25, compact.rs:22, curved.rs:33, outline.rs:26, pills.rs:26}`). Spec: 16px harness icon + agent name.
- `window/tab_strip.rs` is 933 lines.

### 05.2 Chat layout widths

- `chat_column` (`chat/view.rs:61-72`): centred column, `max_w(config.chat.max_width)` unless 0; selector `chat-reading-column`.
- Default `DEFAULT_MAX_WIDTH = 800`, range `320..=4000` (`config/chat.rs:36-38`, default `:66`, validation `:81-87`). Spec: 760 on iPad.
- Coupling: rich prose keeps a book measure only when `max_width == DEFAULT_MAX_WIDTH` (`chat/view/rich.rs:53-56`). List remeasure on width change: `Chats::rewidth` (`chat/chats.rs:591-600`), called from `chat/window.rs:110`.
- Tests that pin 800/1200/0: `chat/tests/width.rs:13-60` (every layout), `settings_window/controls/tests/chat_width.rs:106` (asserts 800), `config/chat/tests.rs:8-9`.

### 05.3 User vs agent message styling

- Layout chosen by `config.chat.layout` (`config/chat.rs:48`), default **`Timeline`** (`config/chat.rs:139-143`): avatar gutter, no bubbles (`chat/view/layout/timeline.rs:1-30`).
- `Bubbles` (`config/chat.rs:144-146`) matches the spec's shape: `bubble_row` (`chat/view/layout.rs:223-264`) right-aligns the user bubble (`items_end` `:239`), `max_w 80%` (`:242`), single `rounded(bubble.radius)` (`:246`), fill `mix(background, foreground, 9)` (`:235`); agent text has no fill (`:252`). Shape constants `radius 10, padding 7, gap 18` (`layout/bubbles.rs:10-14`). Spec: radius 18 with a 6 corner, nord3 fill.
- Header layout button cycles layouts (`chat/view.rs:561-571`).

### 05.4 Permission request rendering

- `render_chat_permissions` (`chat/view/panels.rs:16-83`), placed between the working row and the queue, each wrapped in `chat_column` (`chat/view.rs:404-408`).
- Card `chat-permission` (`panels.rs:62-80`): `mx 12`, `p 8`, `rounded CONTROL`, **yellow border** `hues.yellow` (`:73`), `bg theme.surface`, semibold title = `permission.title` (agent text, `safe_text(..., 400)`, `:75-79`).
- Buttons `chat-answer-{i}` (`panels.rs:28-61`): label is the **agent's option name** (`safe_text(&option.name, 60)`, `:55`), outlined, green for allow kinds, red for reject kinds (`:33-37`), `px 10 py 3` (≈ 20-24pt tall [推測 from padding + line height]). Sends `Command::Answer` (`:39-42`, `:57-60`).
- Model: `Permission { call_id, id, options, title }` (`crates/herdr-chat-protocol/src/model.rs:321-330`), `PermissionOption { id, kind, name }` (`:332-338`), `PermissionKind` (`:340-356`). No command field; a tool call (`Tool`, `model.rs:248-266`, `ToolKind::Execute` `:276`) can be found in the transcript by `call_id` (no lookup helper exists; `Transcript::activity(call_id)` at `model.rs:125` is the only call_id accessor).
- No Fluent keys for permission headings or Allow/Always allow/Deny exist (grep of `chat.ftl`/`chat-text.ftl` for allow|deny|reject|permission returned nothing; positive control `chat-composer-placeholder` found at `chat.ftl:35`).
- No headless test renders a permission card (`PermissionOption`/`PermissionKind::` appear in `crates/herdr-gpui` only at `panels.rs:35`).

### 05.5 Composer

- Parts built in `render_chat_composer` (`chat/view/composer.rs:20-311`): round buttons are **28px** (`round` `:68-80`); attach `chat-attach` = plus icon 13px (`:81-95`); send `chat-send` = `theme.primary()` circle when enabled, `theme.active` when off (`:214-233`); stop `chat-stop` red (`:197-213`); draft + `chat-composer` field (`:235-270`); card from the layout (`:288`); outer padding (`:289-311`).
- Layout composers: Timeline card `rounded PANEL` with attach and send on the foot row (`layout/timeline.rs:84-112`); Bubbles field `rounded(PANEL + 4)` = 16 with send inside, **attach outside** in the settings row (`layout/bubbles.rs:26-60`, attach `:55`). Spec: radius 22 frame, attach 44 and send 44 (accent circle).
- Placeholder: `chat-composer-placeholder` "Message the agent — / for commands, Shift-Enter for a new line" (`locales/en-US/chat.ftl:35`), set in `chat/window.rs:182-190` (closed: `chat-composer-closed`, `chat.ftl:36`).

### 05.6 Gap list vs spec 05

1. Tab icon: chat tabs resolve to Generic (`acp`); Classic layout draws no icon; icon sizes 11-13 vs 16.
2. Tab title becomes "Agent: prompt" after the first prompt (herdr-acp behaviour, outside this client).
3. Reading width 800 vs 760.
4. Default layout Timeline vs spec's bubbles; bubble radius 10 vs 18/6; fill `mix(bg,fg,9)` vs nord3.
5. Permission card: heading is the agent's title, no separate "Run a command" heading, no mono command line, buttons are agent-named outlined chips (< 44pt) instead of 48pt Allow (accent fill) / Always allow / Deny.
6. Composer: 28px buttons vs 44; radius 16/12 vs 22; Bubbles puts attach outside the frame.

### 05.7 How to change only on iPad (design notes)

- Icon: fall back to `display_agent` when `agent.agent == Some("acp")` in `herdr_mark` (`tab_strip.rs:914-930`), mirroring `chat::view::agent_icon` (`chat/view.rs:101-104`). This is arguably a cross-platform fix (desktop also shows Generic); doing it only on iPad needs an explicit decision (see Unknowns) [推測].
- Width: `DEFAULT_MAX_WIDTH` per target (`if cfg!(target_os = "ios") { 760 } else { 800 }`) keeps `rich.rs:55`'s measure coupling and leaves desktop tests untouched; clamping in `chat_column` under `active()` would break `chat/tests/width.rs` (1200 and 0 asserted, `active()` is true in tests) [推測 recommendation].
- Bubbles: add iPad shape/fill in `bubble_row` or a dedicated iPad `Bubble` constant behind `active()`; per-corner radius needs GPUI per-corner setters [推測: `rounded_tl`/`rounded_br` style APIs exist in gpui's `Styled`; not verified in this repo]. Making Bubbles the iPad default requires replacing the derived `#[default]` (`config/chat.rs:105`, `:142`) with a target-aware default.
- Permission card: new Fluent keys (e.g. a heading per `ToolKind`, and `Allow` / `Always allow` / `Deny` mapped from `PermissionKind`) with the agent's `option.name` kept for `Other` kinds; mono command from the matching `Tool.title` by `call_id` [推測]. `panels.rs` is 236 lines (room).
- Composer: iPad branch in `composer.rs` `round` size (`:73`) and in the chosen layout's `composer`; `composer.rs` is 312 lines.

## Shared with desktop (touch points)

| Code | Shared with | Note |
| --- | --- | --- |
| `menu/chrome.rs` panel chrome, overlay, backdrop | every menu and dialog | Gate by `page == AddDevice && active()` |
| `menu/appear.rs:31-38` backdrop | every dimming dialog | Do not change `DIM` |
| `menu/devices/rail.rs:277-340` frame | SSH, Catalog, Coder, Cloud, WSL | iPad builds only reach SSH |
| `discover/render.rs` | desktop Add Device | — |
| `search_input.rs` (882 lines) | every text field | Sizing via wrapper / `set_frameless` (`:207`) |
| `tab_strip.rs` `herdr_mark`, `tab_layout/*` | all tabs | Overlaps screen 01 tab-row work |
| `chat/view/layout.rs`, `layout/*.rs` | all chat layouts | `bubble_row` used by Bubbles |
| `chat/view/panels.rs`, `composer.rs` | all chats | — |
| `config/chat.rs` defaults | config parsing and settings UI | — |

## Risks

1. `ipad_ui::active()` is true in all tests (`ipad_ui.rs:15-19`): every `active()`-gated change alters what existing desktop-oriented tests see (e.g. `chat/tests/width.rs`, device tests). Either add `ipad_ui::desktop()` guards to those tests or gate by `cfg!(ios)` and test the render function directly. The handoff's self-check "デスクトップ向けのテストとスナップショットが変わっていないか" depends on this.
2. `add_device.rs` is 986/1000 lines; `tab_strip.rs` 933, `chrome.rs` 939, `search_input.rs` 882. Splits must happen in the same PR.
3. cfg(ios)-only code (`device_key.rs`, `cfg!` at `add_device.rs:121/301/370/423-454`) has no headless coverage; `just test-ios` does not run herdr-gpui tests.
4. The header "Add device" button is step-dependent (Form submit, Check again, Add Herdr 2 host, Trust key); moving it to the header duplicates the footer's `match` unless the step→action mapping is extracted first.
5. Tests with default features may draw the rail (cloud on) while the iPad app never does; iPad sheet tests should construct the expected source set explicitly.
6. Chat tab icon fix via `display_agent` would also change desktop tabs and sidebar if done ungated.
7. Permission labels from the agent are untrusted text (`safe_text`, `panels.rs:55`); replacing them with Fluent labels by kind must keep the agent's name for `PermissionKind::Other`.

## Unknowns

1. Backdrop "66% black" vs the handoff's own rule "色はテーマから取り、hex をハードコードしない" — which wins.
2. Whether the chat-tab icon fix (`acp` → `display_agent`) should be iPad-only or for all platforms (AGENTS.md: desktop unchanged).
3. Whether the iPad default chat layout should switch to Bubbles, or Timeline gets an iPad look.
4. Where the "Run a command" heading text comes from (new Fluent key per `ToolKind`, or the agent's `permission.title`) and whether the mono command is `Tool.title`. `[推測]` that Claude's ACP titles carry the command text.
5. Whether "selected row with accent check" means the field's current match (`completion.highlight`) or a separate tap selection.
6. Exact font scale on iPad (`config.ui.size`), which scales the 640/620 dialog constants (`chrome.rs:447`, `:511`).
7. The design HTML (`*.dc.html`) is not on this Mac (HANDOFF.md line 16); pixel values above come from the HANDOFF text only.
