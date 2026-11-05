# Settings on iPad

Settings shows controls the iPad client can use. Desktop settings and persisted
configuration remain intact; changing a device's platform does not erase values.
The shared presentation policy is in
[`settings_window/availability.rs`](../crates/herdr-gpui/src/settings_window/availability.rs#L30).
It applies to rows, navigation, search results, and direct page selection.

| Hidden controls | Reason and implementation evidence |
| --- | --- |
| Plugins page | Its switches edit the local daemon's shared `[ui.sidebar]` file, rather than the connected host: [`controls/plugins.rs`](../crates/herdr-gpui/src/settings_window/controls/plugins.rs), `value_switch`, and [`pages.rs`](../crates/herdr-gpui/src/settings_window/pages.rs#L48), `shared_gate`. |
| Remote Control page | This configures a local service; its supervisor is not installed by the iPad launcher: [`remote/supervisor.rs`](../crates/herdr-gpui/src/remote/supervisor.rs#L180), `Supervisor::install`, and [`app/ios.rs`](../crates/herdr-gpui/src/app/ios.rs#L75), `run`. Connecting to a saved SSH host is separate and remains available. |
| Cloud Devices page, if compiled in | Cloud jobs require desktop processes. The iPad app normally builds without these features: [`cloud.rs`](../crates/herdr-gpui/src/cloud.rs#L19) and [`app/ios.rs`](../crates/herdr-gpui/src/app/ios.rs#L75). |
| Listening ports, agent checkpoints, merged-worktree removal | Port monitors use process-backed shells; Git operations require a supported VCS host: [`window/monitors.rs`](../crates/herdr-gpui/src/window/monitors.rs#L79), `connected_hosts` and `update_checkpoints`, and [`herdr-vcs/git.rs`](../crates/herdr-vcs/src/git.rs#L1). |
| Browser skill installation and update channel | The browser skill targets agents on a computer; installation hooks are excluded on iOS: [`agent_skill.rs`](../crates/herdr-gpui/src/agent_skill.rs). The iPad launcher constructs an idle updater and installs no update action: [`app/ios.rs`](../crates/herdr-gpui/src/app/ios.rs#L75). |
| Usage, CPU/memory/GPU status and sidebar GPU | Usage is explicitly gated off on iOS; host monitors return no hosts because they use process-backed shells: [`window/monitors.rs`](../crates/herdr-gpui/src/window/monitors.rs#L79), `update_usage` and `connected_hosts`. |
| Copy-on-select, tab position, single-tab hiding, pane-history switch, notification delivery and shared sound switch, shared config file | These edit this computer's daemon config rather than the remote host: [`pages/general.rs`](../crates/herdr-gpui/src/settings_window/pages/general.rs), [`pages/tabs.rs`](../crates/herdr-gpui/src/settings_window/pages/tabs.rs), [`pages/notifications.rs`](../crates/herdr-gpui/src/settings_window/pages/notifications.rs), and [`pages/configuration.rs`](../crates/herdr-gpui/src/settings_window/pages/configuration.rs). Local clipboard/toast overrides, sound volume and reload remain available. |
| Remote pane-history controls | Their SSH scripts use `run_script`, which returns `ScriptUnsupported` on iOS: [`remote_history.rs`](../crates/herdr-gpui/src/settings_window/remote_history.rs) and [`herdr-client/script.rs`](../crates/herdr-client/src/script.rs#L50). The iPad does not launch those probes. |
| Herdr 2 terminal switch | The iPad cannot start a local core: [`core_terminals.rs`](../crates/herdr-gpui/src/core_terminals.rs). Existing remote terminals remain available. |
| Follow Herdr theme and Herdr theme library | These resolve or edit the local daemon's theme configuration, rather than the connected host: [`themes.rs`](../crates/herdr-gpui/src/settings_window/themes.rs#L198), `source_enabled` and `accept_theme_choice`, and [`pages/theme.rs`](../crates/herdr-gpui/src/settings_window/pages/theme.rs). The app's theme library remains available. |
| Start VS Code locally; Keep Awake's system-only scope | Local Code startup needs embedded pages and a CLI: [`controls/code/start.rs`](../crates/herdr-gpui/src/settings_window/controls/code/start.rs). iPad's Keep Awake explicitly rejects `AwakeScope::System`: [`caffeine/ios.rs`](../crates/herdr-gpui/src/caffeine/ios.rs#L93), `display_scope`. An existing Code URL and display Keep Awake remain available. |

Fonts, app themes, terminal padding, sidebar presentation, terminal tab layouts,
closing confirmations, ACP transcript appearance, local notification overrides,
chat launcher placement, extra chat environment, and SSH connections remain available.
Remote chat creation passes the configured environment to the endpoint
([`chat/start.rs`](../crates/herdr-gpui/src/chat/start.rs#L214), `start_chat`). Integrations use the connected endpoint's
advertised API, so they remain available too
([`integrations.rs`](../crates/herdr-gpui/src/integrations.rs#L212), `load_integrations`).
GitHub account controls and update menu entries already have iOS guards in
[`titlebar.rs`](../crates/herdr-gpui/src/titlebar.rs) and
[`menus.rs`](../crates/herdr-gpui/src/menus.rs).
