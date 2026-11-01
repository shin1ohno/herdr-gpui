# Sidebar research — screens 01, 03, and the sidebar of 05

Repo: `/Users/shin1ohno/ManagedProjects/herdr-gpui` at `feat/ipados-foundation` (HEAD 76a86890). Paths below are repo-relative unless absolute. All paths under `crates/herdr-gpui/src/` are abbreviated `src/`.

## Bottom line

- The sidebar already has a pluggable row architecture (`RowLayout` trait, `src/sidebar/cell.rs:152-161`, chosen by `layout_for` at `src/sidebar/cell.rs:225-234`), a collapsed rail (`src/sidebar/rail.rs`, 64 pt), long-press context menus on host / workspace / agent rows, agent harness icons, and status marks. The iPad sidebar can be built as a new `RowLayout` plus an iPad metrics preset rather than by editing `render.rs` (937 lines) or `row.rs` (806 lines) in place.
- Data that does **not** exist today: per-host latency in ms, any wall-clock "elapsed since state change" for agents or spaces, a compact `2m` age format, a "Needs you" reason for terminal agents (only an optional integration `state_labels["blocked"]` text), a tab count string, and any space list for Herdr 2 (core-only) hosts.
- Biggest risk: `crate::ipad_ui::active()` returns **true in every unit test** unless the test holds `ipad_ui::desktop()` (`src/ipad_ui.rs:5-24`). Only 3 tests do (`src/core_terminals/tests/reconnect.rs:128`, `src/settings_window/tests/ipad_key.rs:116`, `src/sidebar/rail/tests/touch_context.rs:55`) while the sidebar has 207 `#[gpui::test]` functions. Gating the iPad sidebar geometry on `active()` would move every existing desktop sidebar layout test onto the iPad path.
- The default sidebar width is **232**, not 272: `SIDEBAR_WIDTH = 232.` (`src/sidebar/metrics.rs:7`). With the activity bar on by default (`src/config/sidebar_style.rs:147,204`), the left column is 232 + 44 (`BAR_WIDTH`, `src/sidebar/activity.rs:35`) = 276 pt.

## How `ipad_ui` affects the sidebar today

| Behaviour | Gate | Evidence |
| --- | --- | --- |
| Rail agent rows gain a long-press pane menu | `ipad_ui::active()` | `src/sidebar/rail.rs:489-507` |
| Long press opens every `context_menu` (host, workspace, agent rows, expanded and rail) | `cfg(any(target_os = "ios", test))` | `src/menu/context_menu.rs:11-60` |
| The `local` sentinel endpoint is not listed | `cfg!(target_os = "ios")` | `src/endpoint/order.rs:54-56`, `src/menu/devices.rs:268-276` |
| Connection status of a Herdr 2 host comes from the core hub, not the Herdr 1 snapshot | `cfg!(target_os = "ios")` | `src/window/connection_status.rs:10-26` |
| Herdr 2 panes listed under the agents section | `core_target_supported()` | `src/core_terminals/sidebar.rs:17-50`, called from `src/sidebar/render.rs:831` |

Nothing else in `src/sidebar/` checks `ipad_ui` or `target_os` (grep of `ipad_ui::active` lists 14 call sites in the crate; only `rail.rs:492` is in the sidebar).

Consequence of `endpoints[0]` being the `local` sentinel on iPad: `multi = self.endpoints.len() > 1` (`src/sidebar/render.rs:99`) is true as soon as one SSH device exists, so `headed` is true (`render.rs:100`) and the spaces list draws a host header row per device (`render.rs:148-159`) with workspace rows nested by `look.nest_indent()` (`render.rs:179`).

---

## Screen 01 — main, stand + hardware keyboard

### Sidebar width (spec 300 pt; desktop must stay as is)

- `SIDEBAR_WIDTH = 232.` and `sidebar_width(preferred, window_width)` clamps to 160..480 and reserves 240 pt for the terminal (`src/sidebar/metrics.rs:7,35-41`).
- `SidebarMode::width` routes Expanded → `sidebar_width`, Rail → `RAIL_WIDTH` (`src/sidebar/rail.rs:61-72`).
- Width is used by `render_sidebar` (`src/sidebar/render.rs:43`), the activity column (`src/sidebar/activity.rs:251-262,398-399`), and the cached view (`src/sidebar/view.rs:83-91`).
- A user-dragged width persists as `sidebar_width_px` (`src/preferences.rs:123,264`; `src/window.rs:360,513`). A saved preference would override any iPad default.
- Layout tests assert the default against the constant (`src/sidebar/layout_tests.rs:352-353,406`).
- Seam: an iPad default (300) inside `sidebar_width` when `preferred` is `None`, behind an iPad gate that tests do not take by default (see Risks).

### Activity bar on iPad today

- On by default (`src/config/sidebar_style.rs:147,204`). The column becomes bar (44) + sidebar, under a 32 pt title row with the section name upper-cased and the sidebar toggle (`src/sidebar/activity.rs:35,41,232-238,634-663`). The bar carries Settings (`src/sidebar/activity.rs` render_activity_bar, `activity-settings` button at ~line 480-488) and its buttons are 32 pt (`BUTTON`, `activity.rs:36`), below the 44 pt minimum. The spec does not mention the bar at all.

### Host header row (spec: 64 pt, 10 px dot, bold name, line 2 `SSH · Herdr 2 · 18 ms`, chevron on the right)

Existing row: `HerdrWindow::host_row` in `src/sidebar/host_row.rs:50-373`.

- Order today: collapse arrow ▾/▸ on the leading side (`host_row.rs:245-266`), optional origin icon (`267-279`), removing pulse, label (`283-296`), optional working count `2/5` (`297-305`, from `working_summary` `404-413`), load gauges, and a trailing 8 px status dot (`320-331`). The spec reverses this (dot first, chevron trailing).
- Height = lines × line height + 2 × host padding + chrome (`host_row.rs:170-172`); in the default Normal density that is about 20 pt, not 64.
- Connection state: `displayed_connection_status(endpoint).is_connected()` (`host_row.rs:75`; `src/window/connection_status.rs:18-26`). `ConnectionStatus` has Connecting, StartingDaemon, AwaitingSnapshot, Connected, Disconnected, Detached (`src/state.rs:16-23`); `is_connected` covers AwaitingSnapshot and Connected (`state.rs:26-28`). Herdr 2 hosts map hub states to these (`src/core_terminals.rs:239-253`).
- Dot colour: `crate::menu::online(theme)` = `theme.ink(0x63c68b)`, a hard-coded hex (`src/menu/colors.rs:56-58`), else `theme.muted`. The spec requires theme roles (nord14 for connected).
- Transport: `Origin::of(&endpoint.connection.target)` → Local / Ssh / Tailscale / Wsl / Cloud (`src/endpoint/origin.rs:11-40`); `describe()` gives "SSH host", "Tailscale host over SSH" (`origin.rs:73-78`; `locales/en-US/sidebar.ftl:62-66`). No short "SSH" word exists.
- Herdr generation: `Endpoint::backend_generation()` returns `herdr_backend::Generation::{Herdr1, Herdr2}` (`src/endpoint.rs:177-184`). No user-facing "Herdr 1/2" string exists in `sidebar.ftl`.
- Latency in ms: **not measured per host.**
  - `network_quality` measures this machine to `1.1.1.1:443`, for the status bar (`src/network_quality.rs:1,25-28`), not to the SSH host.
  - Herdr 1 over SSH sends `endpoint.health.ping.v1` after 5 s of silence (`crates/herdr-client/src/session.rs:388-398`; `crates/herdr-client/src/limits.rs:12-16`). `Health` stores the ping's send instant (`session.rs:83-129`), but `received()` clears it without computing a round trip (`session.rs:95-98`), and any inbound frame counts as an answer, so it is not an RTT.
  - herdr-ssh keepalives every 15 s (`crates/herdr-ssh/src/config.rs:23-24`) expose no RTT. herdr-core sends server `Heartbeat` notices (`crates/herdr-core-client/src/event.rs:17,151-153`; ignored at `crates/herdr-core-client/src/backend.rs:111`).
  - Showing `18 ms` needs new instrumentation in `herdr-client` (`session.rs` is 910 lines) and in `herdr-core-client` for Herdr 2, plus a path to the UI through `LiveState`. [推測] A dedicated echo (send instant matched to the daemon's response) is needed for a true RTT, since `received()` fires on any frame.
- Host menu on long press: `open_host_menu` (`host_row.rs:229-239`; `src/menu/devices/host_menu.rs:135`); iPad-only `NewCoreTerminal` row (`host_menu.rs:19-50`).

### SPACES heading (spec: `SPACES` + kbd `⌘P`)

- `header("spaces", fl!("sidebar-header-spaces"), …)` (`src/sidebar/render.rs:739-746`; helper `src/sidebar/render/header.rs:12-33`). String is lower case `spaces` (`sidebar.ftl:7`); `HeaderCase` has only `Lower` and `Title` (`src/sidebar/layout.rs:261-264,326-337`), so `SPACES` needs a new case or `to_uppercase()` as the activity title does (`activity.rs:661`).
- With search on, the heading is replaced by the search field (`render.rs:734-738`).
- `⌘P` is `Command::WorkspacePicker => cmd-p` (`crates/herdr-commands/src/shortcuts/macos.rs:54`); iOS uses the macOS table because `shortcuts.rs` falls through to it (`crates/herdr-commands/src/shortcuts.rs:13-17`).

### Space rows (spec: 56 pt, two lines, 10 px dot, mono 14 name, `3 tabs` or `Needs you` badge, line 2 icon + `agent state · elapsed · branch`, worktree indent 22)

Where they are built: the loop at `src/sidebar/render.rs:238-617` builds `RowData::Workspace(WorkspaceRow{…})` (`render.rs:397-440`) and hands it to the layout through `Cell::new(...).row()`.

Data available per workspace (`crates/herdr-protocol/src/wire.rs:374-388`): `workspace_id`, `number`, `label`, `custom_label`, `branch`, `git_ahead_behind`, `tokens`, `worktree { key, label, is_linked_worktree }` (`wire.rs:390-395`), `focused`, `agent_status`.

| Spec element | Existing source | Gap |
| --- | --- | --- |
| Status dot | `workspace.agent_status`, or the group's most urgent member when folded (`render.rs:291-295`; `workspaces.rs:169`) | Dot is 8 pt (`STATUS_WIDTH`, `metrics.rs:14`), spec 10 |
| Name | `workspace_label(workspace, indented)` (`src/sidebar/workspaces.rs:157`) | Sidebar font default is monospace 12 (`src/config.rs:476`), spec 14 |
| `3 tabs` | Count `snapshot.tabs` where `tab.workspace_id == id`, as `agent_place` does (`src/sidebar/agents.rs:216-220`; `wire.rs:396-406`) | No string; new Fluent message with plural |
| `Needs you` badge | `AgentStatus::Blocked` (`wire.rs:290-296`), per-workspace via `workspace.agent_status` or by filtering `snapshot.agents`; `Tally::of` counts by status (`src/devices_overview/model.rs:11-24`) | No string, no badge style; existing right-edge badges are PR / change counts / note / muted (`src/sidebar/row/badge.rs`) |
| Agent name on line 2 | `snapshot.agents` filtered by `workspace_id`; `agent_name()` = display_agent, name, agent, title (`agents.rs:161-173`) | Choosing which agent when several: not defined [推測: focused, then by `status_priority` `agents.rs:70-78`] |
| Agent state word | `status_text()` (`agents.rs:553-561`, `sidebar.ftl:37-41`) and integration labels `state_label()` (`agents.rs:570-586`) | — |
| Elapsed `2m` | None on the wire: agents carry only `state_change_seq: u64` (`wire.rs:419-436`). `AgentRecency` is a logical counter, not a clock (`src/state/agent_recency.rs:12-55`). `ago::text` formats `5 min ago` (`src/ago.rs:10-30`; `locales/en-US/ago.ftl:5-8`) | Needs a client-side `Instant`/wall stamp when `observe` sees a change [推測: extend `AgentRecency`], and a compact Fluent form |
| Branch | `WorkspaceRow::branch()` (`src/sidebar/cell.rs:87-95`) | — |
| `worktree` word | `crate::worktree_identity::is_linked(workspace)` (`render.rs:243`) | No string |
| Harness icon 13 px | `AgentIcon::from_identity(agent.agent)` (see Icons) | Workspace rows have no agent icon today; icon slot is repo avatar / GitHub mark / branch glyph (`row.rs:32-88`, chosen at `render.rs:407-421`) |
| Worktree indent 22 | Normal density `child_indent = STATUS_WIDTH + gap(6) + 8 = 22` (`src/sidebar/layout.rs:112-129`); applied in `row.rs:479-484` | Matches already in Normal; Comfortable uses 28 (`metrics.rs:20-21`) |
| Truncate from the end | `name_line` budgets segments in glyphs and truncates each (`row.rs:322-361`; `metrics.rs:49-75`) | Reusable |
| Row height 56 | Height comes from font line height and density padding (`layout.rs:307-314`; `metrics.rs:31-33`) | No fixed-height row today |

Existing layouts: Herdr (`src/sidebar/layouts/herdr.rs:16-91`, name over branch), Superset (single line), Orca (card with meta line for host, branch, PR, `src/sidebar/layouts/orca.rs:1-5`), Minimal. The `parts::Line` builder (`src/sidebar/layouts/parts.rs:67-266`) lays out fixed / shrinking / filling segments and is the natural base for a new two-line iPad layout. `LayoutMode` has no iPad variant (`src/config/layout.rs:37-56`).

Worktree grouping: `visible_workspace_entries` returns `(index, indented, group)` (`src/sidebar/workspaces.rs:76`), groups per `grouped_keys` (`workspaces.rs:22-36`); fold chevrons via `Fold` (`src/sidebar/layouts.rs:17-45`).

Herdr 2 hosts: a core-only host has no Herdr 1 snapshot, so the loop skips it (`render.rs:186-188`); its only "space" is the client-local `herdr-ui:core:{session}` key (`src/core_terminals.rs:224-236,331-350`), and its panes show as rows under the agents section (`src/core_terminals/sidebar.rs:17-80`). Those panes carry `TerminalInfo { agent: AgentStatus, cwd, id, name, state, title }` (`crates/herdr-backend/src/terminal.rs:93-100`) — no branch, no agent identity, no tab count.

### AGENTS section (spec: heading `AGENTS`, 44 pt rows, dot + 18 px icon + name + meta)

- Built by `append_agent_rows` → `agent_cell` (`src/sidebar/render/agent_rows.rs:23-75,112-193`), order from `panel_agents` (`agents.rs:124-156`; priority sort `agents.rs:88-114`), shown when `agents_placement == Panel` (`render.rs:101-106,624-627,808-833`). Heading `agents` (`sidebar.ftl:9`) with a sort toggle (`agents.rs:17-59`).
- `AgentRow` fields: key, name, icon, status, place, pane, status_text, lines (`src/sidebar/cell.rs:107-125`).
- Icon: already drawn in agent rows via `RowKind::Agent(icon)` (`agent_rows.rs:153`), at `line_height.min(12)` px (`row.rs:761-764`); spec 18.
- Meta: `herdr-gpui · 2m` = workspace label + elapsed (elapsed missing, above); `fix/sidebar` = branch of the agent's workspace (lookup by `agent.workspace_id`); `Needs you` = Blocked; `done` = `status_text(Done)`.
- Long press: `open_agent_menu` → pane menu (`agent_rows.rs:179-188`; `src/pane_menu.rs:334-352`).

### Needs you / waiting-permission reasons

- Terminal agents: the daemon reports only `AgentStatus::Blocked` (`wire.rs:290-296`) plus optional integration words in `state_labels` keyed `"blocked"` (`agents.rs:567-586`, capped at 80 chars `agents.rs:565`). No structured reason field exists. `terminal_title` / `title` may contain text but are untrusted captions (`agents.rs:166-203`).
- ACP chats (`herdr-acp`): the pane reports `blocked` to Herdr (`crates/herdr-acp/src/herdr.rs:155`), and the transcript holds `permissions: Vec<Permission{ title, options, call_id }>` (`crates/herdr-chat-protocol/src/model.rs:53-55,322-330`; `Transcript::is_blocked` `model.rs:94-98`). The window attaches to its daemon's chat panes (`src/chat/chats.rs:1-2,118,163`; `HerdrWindow.chats` `src/window.rs:146`; `Session::transcript()` `src/chat/session.rs:79`), so `permissions[0].title` is the reason source for chat agents [推測: the title reads like "plan wants to run a command" only if the agent phrases it so].
- Colour: Blocked is drawn red today (`0xf38ba8` fallback, `agents.rs:593-602`) or from the host's Herdr palette (`agents.rs:271-299`; `src/herdr_settings.rs:626-628`). The spec wants Needs you in nord13 yellow and red only for "異常", which no `AgentStatus` variant represents.

### Status dots: states, colours, drawing

- States: `Idle, Working, Blocked, Done, Unknown` (`wire.rs:290-296`). No error or disconnected status; "disconnected" exists only as a host `ConnectionStatus`.
- Colours: `Indicators::new` takes the Herdr palette when settings load, else Catppuccin literals: Working `f9e2af`, Blocked `f38ba8`, Done `94e2d5`, Idle `a6e3a1` (hollow), Unknown `6c7086` small (`agents.rs:274-299,593-602`).
- Drawing: `status_indicator` → `status_mark` (dots: idle hollow, others filled, working = spinning orbit) (`agents.rs:385-440`); other icon sets via `status_icon` (`agents.rs:475-490`; `src/icons.rs:99-123`); moving marks go to the mark layer (`agents.rs:497-513`). Edge rail bar option (`agents.rs:342-365`).
- Spec mapping vs code: spec "完了 = 緑の輪" is the code's **Idle** look (green hollow); code Done is teal filled. Spec "待機 = 水色" has no matching code colour. This is a semantic mismatch to resolve before painting.

### Bottom: New space and Settings (spec: `New space` 44 pt + kbd `⌘⇧N`, Settings 44 pt)

- Today: a footer row inside the spaces section with `new` (`Command::Workspace`) and `menu` (opens the app menu) (`render.rs:765-806`; strings `sidebar.ftl:21-24`), height `line_height + 2 × footer_padding` (≈22 pt in Normal). Below the 44 pt minimum.
- `New space` shortcut: `Command::Workspace => cmd-shift-n` (`macos.rs:8`). Settings: `Command::Settings => cmd-,` (`macos.rs:51`), reachable from the activity bar or the device footer (`src/menu/devices.rs:323-337`, footer height 40 `src/sidebar.rs:67`).

### kbd hint helpers

- No ⌘-glyph formatter or `kbd` component exists (grep for `⌘`/`\u{2318}` in `src/` finds only a literal in `src/code_search/render.rs:271`).
- Labels come from `Keymap::primary(command)` / `shortcuts(command)` as GPUI `unparse()` text such as `cmd-b` or `ctrl-b c` (`src/keymap.rs:247,263,311-323`; prefix `keymap.rs:341-343`). Raw labels are shown as-is in `group_menu` (`src/group_menu.rs:341-350`).
- `keycaps()` splits a label into capitalised words (`Cmd`, `B`) (`src/menu/settings.rs:706-729`), and the shortcuts panel draws each as a bordered chip inline (`src/menu/settings.rs:519-537`). That chip is the nearest reusable look; glyph mapping (⌘ ⇧ ⌃ ⌥) would be new [推測: shared with the tab-row and status-bar work].
- Hardware keyboard presence is not exposed to the app (herdr-uikit only reports keyboard geometry, `/Users/shin1ohno/ManagedProjects/herdr-uikit/src/window/layout.rs:206`).

---

## Screen 03 — minimized sidebar (rail of dots)

### Toggle and modes

- `Command::ToggleSidebar` (`crates/herdr-commands/src/lib.rs:51,571-573`; `cmd-b` `macos.rs:42`) → `toggle_sidebar` (`src/window/commands.rs:583`; `src/sidebar/rail.rs:104-109`). Also from the title-bar button (`src/titlebar.rs:294-317`), the activity title (`activity.rs:663`), and the control socket (`src/control/server/app.rs:274`).
- Collapsed result follows Herdr's `ui.sidebar_collapsed_mode`: Compact → Rail, Hidden → nothing (`rail.rs:84-102`; `src/herdr_settings.rs:161-169`), default Compact. So ⌘B already switches Expanded ↔ Rail; the spec's 300 ↔ 72 is this with different widths.
- `ui.sidebar_start_collapsed` applies once (`rail.rs:111-127`). Slide animation between them: `src/sidebar/slide.rs:1-5`; `slide_start` `src/window/zen.rs:84`.
- Which view renders: `SidebarView::render` → activity column, else rail, else expanded (`src/sidebar/view.rs:64-75`; `activity.rs:359-403`).

### Rail geometry vs spec (72 pt, 12 px dots, 10 px labels, 20 px icons, 64×56 buttons)

- `RAIL_WIDTH = 64.`, host mark 22, host dot 6, agent icon 14, gaps 4 (`rail.rs:38-51`); the font steps down to fit (`rail.rs:613-630`); row height = `look.row_height(line_height.max(22))` (`rail.rs:656`). Tests assert `RAIL_WIDTH` (`src/sidebar/rail/tests.rs:78,381,465`).
- Host: letter box with a status dot ring (`rail.rs:188-249,717-752`), only when `multi` — true on iPad with ≥1 device (sentinel).
- Spaces: workspace **number** + optional muted slot + status dot (`rail.rs:264-377`). The spec wants a dot + 10 px mono label (ellipsised) and a 16 px "needs you" count badge top-right; none exist (count source: `Tally::of`, above).
- Divider: 1 pt line between sections (`rail.rs:550-552`); spec 40×1.
- Agents: icon (host letter when multi) + dot (`rail.rs:385-513`); spec wants icon 20 with a 10 px dot at its bottom-right corner ringed in rail colour (the host dot ring `host_status` is the precedent, `rail.rs:712-739`) and a 10 px label.
- Bottom: only a `+` new-workspace button (`rail.rs:567-576`). The rail has no expand control by design ("the sidebar toggle is always on screen", `rail.rs:532-534`); the spec's `»` button is new.
- Tooltips name each mark (`rail.rs:871-891`; `sidebar.ftl:51-57`).

### Long-press menus on rows (spec: header with dot + name + `1 needs you`; rows Open / New tab / Open chat / Detach at 48 pt; width 240, radius 14)

What a long press opens today:

| Row | Menu | Evidence |
| --- | --- | --- |
| Host (expanded and rail) | Host menu: NewCoreTerminal (iOS), Rename, ServerKeybindings, Theme, Remove | `host_row.rs:229-239`; `rail.rs:234-244`; `src/menu/devices/host_menu.rs:19-50` |
| Workspace (expanded and rail) | Workspace popover: Mute/Unmute, New worktree, Fan out, Rename, Teleport…, Close, Checkpoints, Note, Orchestrator, Open worktree, scripts, Collapse/Expand, Delete worktree | `render.rs:449-465`; `rail.rs:346-367`; `src/menu/workspace.rs:477-511,605-679`; `src/menu/page.rs:114-158` |
| Agent (expanded always; rail on iPad) | Pane menu: Rename, Split right/down, Swap, Zoom, Move…, Edit scrollback, Right click, Processes, Close | `agent_rows.rs:179-188`; `rail.rs:489-507`; `src/pane_menu.rs:100-113,334-352` |

`group_menu` (`src/group_menu.rs:1-120`) is the tab-group "…" menu, not a sidebar row menu; it holds `Chat(Agent)` ("new chat with an agent through herdr-acp", `group_menu.rs:38-39,83-85,115`).

Mapping of the spec's four actions to existing code:

- Open → `navigate_endpoint(…, NavigationTarget::Workspace(id))`, what a tap does (`rail.rs:368-375`).
- New tab → `Command::Tab` (`crates/herdr-commands/src/lib.rs` enum; prompts via `WorkspaceAction::NewTab` when `ui.prompt_new_tab_name`, `src/menu/workspace.rs:574`).
- Open chat → [推測] `group_menu` `Action::Chat(agent)` (new herdr-acp chat tab) needs an agent choice; for an existing chat pane, focusing it via `HerdrWindow.chats.pane(pane_id)` (`src/chat/chats.rs:693`).
- Detach → `Command::Detach` (`src/window/commands.rs:642`) → `detach_endpoint` acts on the **selected endpoint** (whole host connection), not a space (`src/endpoint.rs:583-596`). On a space row its meaning would be "detach this host".

None of these four exist as one menu; the spec's compact menu is a new `menu::Page` or a new small popover [推測]. `ContextMenuEvent.mouse == None` marks a touch origin (`context_menu.rs:5-9`), used to keep `opening_right_click` false.

### Crumb in the tab row and status-bar `⌘B Expand sidebar`

Outside the sidebar itself: tab strip `src/window/tab_strip.rs` (933 lines) and status bar `src/window/status_bar.rs` (535 lines). The crumb's current space = focused workspace (`snapshot.focused_workspace_id`, `wire.rs:324`); switching = `Command::WorkspacePicker`.

---

## Screen 05 — sidebar while a chat tab is open

- Same sidebar code as 01; nothing chat-specific in `src/sidebar/` except agents listed from the daemon snapshot.
- Icon gap for ACP chats: `herdr-acp` reports `agent = "acp"` with `display_agent = "<Display>"` (`crates/herdr-acp/src/herdr.rs:23-25,166-169,183-187`; its own comment: clients that ignore display_agent "fall back to `acp` and lose the agent's logo"). The sidebar keys icons on `agent.agent` (`agent_rows.rs:153`; `rail.rs:417`; tab strip `src/window/tab_strip.rs:916-929`), so chat agents get `AgentIcon::Generic`. The chat view instead lowercases the display name (`src/chat/view.rs:101-104`), and display names lowercase to identities (`crates/herdr-chat-protocol/src/agents.rs:47-49`, e.g. `Claude`).
- Permission reason for a chat agent: `Transcript.permissions[0].title` (above).

### Icons

- `AgentIcon` declared once by macro; `from_identity(Option<&str>)` matches Herdr's `agent_label` words exactly, else `Generic` (`src/icons.rs:8-48,51-77`); `element(size, color)` draws SVG in the caller's ink, PNG (`omo`) in source colours (`icons.rs:43-45,80-97`).
- Assets: `assets/icons/agent-{agy,amp,claude,cline,codex,copilot,cursor,devin,droid,gemini,generic,grok,hermes,kilo,kimi,kiro,letta,maki,mastracode,muse,omp,opencode,pi,qodercli,qwen}.svg` and `agent-omo.png` (26 files; `ls assets/icons | grep ^agent-`). Coverage test `src/icons.rs:450-482`.
- Callers: `src/sidebar/render/agent_rows.rs:153`, `src/sidebar/rail.rs:417`, `src/window/tab_strip.rs:925`, `src/chat/view.rs:103`, `src/group_menu.rs:84`, `src/notifications.rs:144`, `src/devices_overview/table.rs:282`, `src/overview/view.rs:385`, others.

---

## Files the changes would touch, by size (limit 1,000)

| File | Lines | Note |
| --- | --- | --- |
| `src/sidebar/render.rs` | 937 | Expanded sidebar; footer, headings, row loop. Near limit — extend via `render/` submodules |
| `src/window/tab_strip.rs` | 933 | Crumb (03) |
| `src/sidebar/rail.rs` | 924 | Rail; any new rail element needs a split |
| `src/menu/devices/host_menu.rs` | 916 | Host menu |
| `crates/herdr-client/src/session.rs` | 910 | Health ping / RTT |
| `src/menu/workspace.rs` | 898 | Workspace menu |
| `src/endpoint.rs` | 880 | Endpoint, detach, backend generation |
| `src/sidebar/rail/tests.rs` | 871 | Add tests in new topic files under `rail/tests/` |
| `src/state.rs` | 842 | `LiveState` (RTT/elapsed plumbing) |
| `src/sidebar/layout_tests.rs` | 814 | Shared fixtures; new tests go to `layout_tests/<topic>.rs` |
| `src/sidebar/row.rs` | 806 | Herdr rows |
| `src/pane_menu.rs` | 777 | Agent menu |
| `src/sidebar/activity.rs` | 723 | Activity column |
| `src/sidebar/layout.rs` | 607 | Metrics presets |
| `src/sidebar/agents.rs` | 605 | Status colours, names |
| `src/sidebar/host_row.rs` | 506 | Host header |
| `src/icons.rs` | 488 | — |
| `src/sidebar/layouts/parts.rs` | 438 | Line builder |
| `src/sidebar/metrics.rs` | 75 | Widths |
| `src/state/agent_recency.rs` | 56 | Change stamps |
| `src/ago.rs` | 45 | Age text |
| `src/sidebar/cell.rs` | 234 | `RowLayout`, `layout_for` |
| `locales/en-US/sidebar.ftl` | 89 | New strings (only `en-US` exists, `crates/herdr-gpui/locales/`) |

## Design seams [推測 — proposals, not confirmed by code]

1. A new `src/sidebar/layouts/ipad.rs` implementing `RowLayout` (two-line space row, 44 pt agent row), selected in `layout_for` (`cell.rs:225-234`) and an iPad `SidebarMetrics` preset in `layout.rs` when an iPad-layout gate is on. Keeps `render.rs` and `row.rs` growth near zero.
2. A host header variant for iPad in a new `host_row` submodule, fed by a small struct `{ connected, origin_short, generation, rtt_ms: Option<u32> }`.
3. Elapsed: extend `AgentRecency` with an observation instant per pane; render with a new compact Fluent message next to `ago-*`.
4. Rail: split `rail.rs` (e.g. `rail/marks.rs`) before adding iPad label rows, badges, and the `»` button; `RAIL_WIDTH` 72 behind the same gate.
5. Row menu for 03: a new compact popover page with Open / New tab / Open chat / Detach, reusing `context_menu`.

## Risks and Unknowns

- **Test-side gate.** `ipad_ui::active()` is true in tests (`src/ipad_ui.rs:14-24`); 207 sidebar tests run without `desktop()`. Any geometry gated on `active()` changes desktop layout tests and snapshot-like assertions (e.g. `layout_tests.rs:352-353`, `rail/tests.rs:78`). Needs either an opt-in test flag for the iPad layout or `cfg(target_os = "ios")` plus explicit iPad tests.
- **Spec width basis.** The handoff says desktop 272; code says 232 (+44 activity bar). Whether the iPad 300 includes the activity bar, and whether the bar stays on iPad, is not specified.
- **Latency.** No per-host RTT exists; Herdr 2 on iPad has no client pings. Plumbing touches `herdr-client` (910-line `session.rs`) and `herdr-core-client`.
- **Elapsed time.** No wall-clock data from the daemon; a client-side stamp resets on reconnect or app relaunch [推測].
- **Needs-you reason.** Only ACP chats have structured permission titles; terminal agents have at most an integration label. Rows without a reason need a fallback word.
- **Status semantics.** Spec's colour table (done = green ring, waiting = cyan, error = red, disconnected = grey ring) does not map 1:1 to `Idle/Working/Blocked/Done/Unknown`; Blocked is red today. Colours are hard-coded hex in two places (`agents.rs:595-599`, `menu/colors.rs:56-58`), against the spec's theme-roles rule.
- **Herdr 2 hosts list no spaces.** Core-only hosts have one client-local workspace key and no daemon workspaces, branches, or agent identities (`core_terminals.rs:224-236`; `herdr-backend/src/terminal.rs:93-100`).
- **Detach semantics.** `Detach` is per host connection (`endpoint.rs:583-596`); placing it on a space row's menu detaches the whole host.
- **Multi-host on iPad.** Spec draws one host header above `SPACES` with a chevron; code nests every device's spaces under its own header inside the list (`render.rs:134-159`). How several devices appear in the 01 layout is not specified.
- **Persisted width.** A saved `sidebar_width_px` overrides any new default (`preferences.rs:264`).
- **Hardware keyboard detection** for showing kbd hints only when a keyboard is attached is not exposed by herdr-uikit (unconfirmed beyond the grep in `herdr-uikit/src`).
- **Not verified:** native rendering on iPad/simulator, actual text in `state_labels` for real integrations, and whether `Permission.title` reads like the spec's example.
