//! Terminals that herdr-core (Herdr 2) runs, shown as tabs in the window's
//! groups beside its Herdr tabs and pages. herdr-core streams raw output, so
//! the app emulates each one and paints it with the same painter as a Herdr
//! pane. A tab splits into panes the app lays out itself, drawn in the style
//! of the window's tab layout; any group can show one too, so terminals also
//! sit side by side across groups.
//!
//! Everything that waits runs off the UI thread: the hub connects (and may
//! start the bundled core), each shown pane has a worker that owns its
//! `herdr_backend::Terminal`, and terminal details are listed on a thread.
//! The window reads their mailboxes on its tick and renders the prepared
//! screens.

mod commands;
mod error;
mod fallback;
mod hub;
mod infos;
pub(crate) mod panes;
mod remote;
mod screen;
mod selection;
mod sidebar;
mod split;
mod tabs;
mod view;
mod worker;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

pub(crate) use error::Error;
pub(crate) use hub::{Hub, Start, State};
pub(crate) use remote::Remote;
pub(crate) use view::{CoreTerminalView, Look};

use crate::{
    HerdrWindow,
    browser::{Location, Scope, Store, TabId},
    i18n::fl,
    window::Flash,
};
use gpui::{prelude::*, *};
use herdr_backend::{Backend as _, Spawn, TerminalId, TerminalInfo};
use panes::{Axis, Panes};
use std::collections::HashMap;

type Started = std::sync::mpsc::Receiver<Result<TerminalInfo, herdr_core_client::Error>>;

/// Desktop core discovery is local; iPad connects only through SSH. A browser
/// workspace never changes which machine the core transport actually reaches.
fn supports_core_target(target: &herdr_client::ConnectTarget) -> bool {
    if cfg!(target_os = "ios") {
        matches!(target, herdr_client::ConnectTarget::Ssh { .. })
    } else {
        !target.is_remote()
    }
}

/// What a shell being started is for.
#[derive(Clone, Debug)]
enum Purpose {
    /// A new tab in a workspace.
    Tab { scope: Scope, workspace: String },
    /// A new pane beside `at` in a tab.
    Split {
        tab: TabId,
        at: TerminalId,
        axis: Axis,
    },
}

/// One tab's panes as the window holds them. The tab's saved location
/// follows, written when a change settles rather than on every drag step.
#[derive(Debug)]
struct TabPanes {
    /// The pane keys go to while the tab has focus.
    active: TerminalId,
    /// A mirror tab only watches: it neither types nor splits.
    observe: bool,
    panes: Panes,
    unsaved: bool,
    /// How far the zoomed pane has grown over the tab; it shrinks back to
    /// its place when unzoomed.
    zoom: crate::motion::Toggle,
    /// The pane filling the tab, the rest hidden behind it, kept while it
    /// shrinks back.
    zoomed: Option<TerminalId>,
}

impl TabPanes {
    fn new(panes: Panes, observe: bool) -> Self {
        Self {
            active: panes.first().clone(),
            observe,
            panes,
            unsaved: false,
            zoom: crate::motion::Toggle::default(),
            zoomed: None,
        }
    }

    /// Zooms `terminal`, or unzooms, animating from where the zoom is now.
    fn set_zoom(&mut self, terminal: Option<TerminalId>) {
        let now = crate::motion::now();
        match terminal {
            Some(terminal) => {
                self.zoomed = Some(terminal);
                self.zoom.set(true, now, false);
            }
            None => self.zoom.set(false, now, false),
        }
    }

    /// The zoomed pane while it shows, and how much of the tab it fills.
    fn zoom_at(&self, now: std::time::Instant) -> Option<(&TerminalId, f32)> {
        let zoomed = self.zoomed.as_ref()?;
        (self.zoom.is_open() || self.zoom.moving(now)).then(|| (zoomed, self.zoom.shown(now)))
    }
}

/// The window's herdr-core state.
#[derive(Default)]
pub(crate) struct CoreTerminals {
    /// The shell shown when no Herdr 1 is installed.
    fallback: fallback::Fallback,
    /// A shown tab whose active pane should take the keyboard once its view
    /// exists.
    focus_wanted: Option<TabId>,
    pub(crate) hub: Hub,
    infos: infos::Infos,
    /// How the views draw now, to tell when the window's look changed.
    look: Option<Look>,
    /// The endpoint whose core results and views this window currently accepts.
    scope: Option<Scope>,
    /// A shell starting, and what for.
    starting: Option<(Purpose, Started)>,
    tabs: HashMap<TabId, TabPanes>,
    views: HashMap<(TabId, TerminalId), Entity<CoreTerminalView>>,
    /// A shell to start once the hub connects.
    wanted: Option<Purpose>,
    /// An app-owned workspace explicitly opened for a core-only SSH host.
    workspace: Option<String>,
}

impl CoreTerminals {
    fn reset_views(&mut self) {
        let hub = std::mem::take(&mut self.hub);
        let scope = self.scope.take();
        let workspace = self.workspace.take();
        *self = Self {
            hub,
            scope,
            workspace,
            ..Self::default()
        };
    }

    /// Every view on screen: the tabs' panes and the stand-in shell.
    fn shown(&self) -> impl Iterator<Item = &Entity<CoreTerminalView>> {
        let fallback = match &self.fallback {
            fallback::Fallback::Shown(view) => Some(view),
            _ => None,
        };
        self.views.values().chain(fallback)
    }

    /// Whether any shown terminal has output the window has not drawn.
    pub(crate) fn updated(&self, cx: &App) -> bool {
        self.shown().any(|view| view.read(cx).changed())
    }

    /// Whether a herdr-core terminal holds the keyboard.
    pub(crate) fn focused(&self, window: &Window, cx: &App) -> bool {
        self.shown()
            .any(|view| view.read(cx).focus_handle().contains_focused(window, cx))
    }

    /// The tab and pane whose view holds the keyboard.
    fn focused_pane(&self, window: &Window, cx: &App) -> Option<(TabId, TerminalId)> {
        self.views
            .iter()
            .find(|(_, view)| view.read(cx).focus_handle().is_focused(window))
            .map(|(key, _)| key.clone())
    }
}

impl HerdrWindow {
    fn core_target_supported(&self) -> bool {
        self.endpoints
            .get(self.selected_endpoint)
            .is_some_and(|endpoint| supports_core_target(&endpoint.connection.target))
    }

    /// A changed endpoint retires every inbox, pending spawn, and view before
    /// another core may supply an identically named terminal. No worker is joined.
    fn select_core_target(&mut self, cx: &mut Context<Self>) -> bool {
        let endpoint = self.endpoints.get(self.selected_endpoint);
        let scope = endpoint.map(crate::browser::scope);
        #[cfg(target_os = "ios")]
        let target = endpoint
            .map(|endpoint| endpoint.connection.target.clone())
            .unwrap_or_default();
        #[cfg(not(target_os = "ios"))]
        let target = herdr_client::ConnectTarget::Local;
        let changed = self.core_terminals.hub.retarget(&target);
        if !changed && self.core_terminals.scope == scope {
            return false;
        }
        self.save_core_panes(cx);
        self.core_terminals.hub.reset();
        let hub = std::mem::take(&mut self.core_terminals.hub);
        self.core_terminals = CoreTerminals {
            hub,
            scope,
            ..CoreTerminals::default()
        };
        true
    }

    /// The browser key for a core-only host is client-local UI state. It
    /// never creates a daemon workspace or fabricates a daemon snapshot.
    pub(crate) fn core_workspace_key(&self) -> Option<(Scope, String)> {
        let endpoint = self.endpoints.get(self.selected_endpoint)?;
        if self.core_terminals.hub.target() != &endpoint.connection.target
            || self.core_terminals.scope.as_ref() != Some(&crate::browser::scope(endpoint))
        {
            return None;
        }
        Some((
            self.core_terminals.scope.clone()?,
            self.core_terminals.workspace.clone()?,
        ))
    }

    /// The actual core connection for this endpoint, projected for painting
    /// without changing Herdr 1's status, snapshot, or authoritative inbox.
    pub(crate) fn core_connection_status(
        &self,
        endpoint_id: &str,
    ) -> crate::state::ConnectionStatus {
        use crate::state::ConnectionStatus;
        if !self.core_endpoint_matches(endpoint_id) {
            return ConnectionStatus::Disconnected;
        }
        match self.core_terminals.hub.state() {
            State::Connecting(_) => ConnectionStatus::Connecting,
            State::Detached => ConnectionStatus::Detached,
            State::Ready(backend) if !backend.core().is_lost() => ConnectionStatus::Connected,
            State::Idle | State::Ready(_) | State::Unavailable(_) => ConnectionStatus::Disconnected,
        }
    }

    fn core_endpoint_matches(&self, endpoint_id: &str) -> bool {
        self.endpoints
            .get(self.selected_endpoint)
            .is_some_and(|endpoint| {
                endpoint.id == endpoint_id
                    && self.core_terminals.scope.as_ref() == Some(&crate::browser::scope(endpoint))
                    && self.core_terminals.hub.target() == &endpoint.connection.target
            })
    }

    pub(crate) fn core_connection_error(&self, endpoint_id: &str) -> Option<&Error> {
        if !self.core_endpoint_matches(endpoint_id) {
            return None;
        }
        match self.core_terminals.hub.state() {
            State::Unavailable(error) => Some(error.as_ref()),
            _ => None,
        }
    }

    /// Detaching only signals the connection and retires local views/inboxes.
    /// It never kills a terminal, stops a daemon, or joins a worker on the UI thread.
    pub(crate) fn detach_core_connection(&mut self, cx: &mut Context<Self>) {
        self.save_core_panes(cx);
        self.select_core_target(cx);
        self.core_terminals.hub.detach();
        self.core_terminals.reset_views();
        cx.notify();
    }

    /// A changed backend policy retires pending terminal work as well as its
    /// old connection, without reconnecting or carrying its workspace forward.
    #[cfg(any(target_os = "ios", test))]
    pub(crate) fn retire_core_connection(&mut self, cx: &mut Context<Self>) {
        self.save_core_panes(cx);
        self.core_terminals.hub.reset();
        self.core_terminals.reset_views();
        self.leave_core_workspace();
    }

    /// Reconnects the same namespace after an explicit user action. Restored
    /// tabs attach when ready; reconnect never starts another shell.
    pub(crate) fn reconnect_core_connection(&mut self, cx: &mut Context<Self>) {
        self.save_core_panes(cx);
        self.select_core_target(cx);
        if !self.core_target_supported() {
            return;
        }
        #[cfg(target_os = "ios")]
        if self.browser_key().is_none() {
            self.open_core_workspace();
        }
        let focus = self
            .active_group()
            .and_then(|group| self.group_pick(group))
            .and_then(|pick| match pick {
                crate::browser::Pick::Page(tab) if self.core_terminals.tabs.contains_key(&tab) => {
                    Some(tab)
                }
                _ => None,
            });
        self.core_terminals.hub.reset();
        self.core_terminals.reset_views();
        self.core_terminals.focus_wanted = focus;
        self.core_terminals.hub.connect(Start::IfMissing);
        cx.notify();
    }

    /// Explicit daemon-workspace navigation can leave the app-owned core workspace.
    pub(crate) fn leave_core_workspace(&mut self) {
        self.core_terminals.workspace = None;
    }

    /// Opens only the saved tabs' client namespace. Selecting a core-only
    /// device never starts its daemon or opens an SSH connection.
    #[cfg(target_os = "ios")]
    pub(crate) fn show_core_workspace(&mut self, cx: &mut Context<Self>) {
        self.select_core_target(cx);
        self.open_core_workspace();
        cx.notify();
    }

    #[cfg(any(target_os = "ios", test))]
    fn open_core_workspace(&mut self) -> Option<(Scope, String)> {
        let herdr_client::ConnectTarget::Ssh { session, .. } = self.core_terminals.hub.target()
        else {
            return None;
        };
        // Reserved only in the client's tab/layout stores, never sent to Herdr.
        // Sessions keep separate tab strips as well as separate transport fences.
        self.core_terminals.workspace = Some(format!("herdr-ui:core:{session}"));
        self.core_workspace_key()
    }

    /// The host menu's explicit action; a failed Herdr 1 probe never calls it.
    #[cfg(target_os = "ios")]
    pub(crate) fn new_core_terminal_for_host(
        &mut self,
        endpoint_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.select_endpoint(endpoint_id, cx) {
            self.new_core_terminal(window, cx);
        }
    }

    /// The `new_core_terminal` command: starts a shell in herdr-core and
    /// shows it in a tab of the focused workspace. With no core running, the
    /// user's own `herdr-core` or the bundled one starts.
    pub(crate) fn new_core_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.select_core_target(cx);
        if !self.core_target_supported() {
            self.show_flash(
                Flash::warning(fl!(
                    "core-terminals-unavailable",
                    reason = Error::UnsupportedTarget.to_string()
                )),
                cx,
            );
            return;
        }
        let key = self.browser_key();
        #[cfg(target_os = "ios")]
        let key = key.or_else(|| self.open_core_workspace());
        let Some((scope, workspace)) = key else {
            self.show_flash(Flash::warning(fl!("core-terminals-open-workspace")), cx);
            return;
        };
        self.dismiss_menu(window, cx);
        self.core_terminals.wanted = Some(Purpose::Tab { scope, workspace });
        self.core_terminals.hub.connect(Start::IfMissing);
        self.start_core_shell();
    }

    /// Starts the wanted shell once the hub is connected, in the folder of
    /// the pane it splits when herdr-core knows it.
    fn start_core_shell(&mut self) {
        let terminals = &mut self.core_terminals;
        if terminals.starting.is_some() {
            return;
        }
        let Some(backend) = terminals.hub.backend().cloned() else {
            return;
        };
        let Some(purpose) = terminals.wanted.take() else {
            return;
        };
        let cwd = match &purpose {
            Purpose::Split { at, .. } => terminals.infos.get(at).and_then(|info| info.cwd.clone()),
            Purpose::Tab { .. } => None,
        };
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let spawn = Spawn {
            cwd,
            ..Spawn::default()
        };
        let spawned = std::thread::Builder::new()
            .name("herdr-core-spawn".into())
            .spawn(move || {
                let _ = send.send(backend.spawn(&spawn));
            });
        match spawned {
            Ok(_) => terminals.starting = Some((purpose, receive)),
            Err(error) => tracing::warn!(%error, "Could not start a herdr-core spawn worker"),
        }
    }

    /// Runs every window tick: connects for restored tabs, places a started
    /// terminal, gives each shown pane its view, and takes new screens.
    pub(crate) fn poll_core_terminals(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.select_core_target(cx) {
            cx.notify();
        }
        let connected = self.core_terminals.hub.backend().is_some();
        if self.core_terminals.hub.poll() {
            // A lost connection ended every pane's worker. Their views go, so
            // the next sync joins the core again and attaches each terminal
            // anew; nothing is started or replayed. A pane that held the
            // keyboard has its tab take it back once the new view is there.
            if connected && self.core_terminals.hub.backend().is_none() {
                if let Some((tab, _)) = self.core_terminals.focused_pane(window, cx) {
                    self.core_terminals.focus_wanted = Some(tab);
                }
                self.core_terminals.views.clear();
            }
            if let State::Unavailable(error) = self.core_terminals.hub.state()
                && self.core_terminals.wanted.take().is_some()
            {
                let reason = error.to_string();
                self.show_flash(
                    Flash::warning(fl!("core-terminals-unavailable", reason = reason)),
                    cx,
                );
            }
            self.start_core_shell();
            cx.notify();
        }
        self.place_started_core_terminal(window, cx);
        self.sync_core_views(cx);
        if let Some(tab) = self.core_terminals.focus_wanted
            && self.core_pane_view(tab).is_some()
        {
            // Focus the user gave something else while the pane waited, such
            // as a picker's search field, is newer than the wish: keep it.
            if keyboard_free(window.focused(cx).as_ref(), &self.focus) {
                self.focus_core_terminal(tab, window, cx);
            } else {
                self.core_terminals.focus_wanted = None;
            }
        }
        self.poll_core_fallback(cx);
        self.refresh_core_looks(cx);
        if let Some(view) = self.core_fallback_view() {
            view.update(cx, |view, cx| view.poll(cx));
        }
        self.poll_core_panes(window, cx);
        // Split chrome and the sidebar's rows name panes from what herdr-core
        // reports, so it is read while any herdr-core tab is known.
        let listed = !self.core_terminals.tabs.is_empty();
        let backend = listed.then(|| self.core_terminals.hub.backend()).flatten();
        if self
            .core_terminals
            .infos
            .poll(backend, std::time::Instant::now())
        {
            cx.notify();
        }
        if !cx.has_active_drag() {
            self.save_core_panes(cx);
        }
        // A zoom in motion draws a frame each tick until it lands.
        let now = crate::motion::now();
        if self
            .core_terminals
            .tabs
            .values()
            .any(|tab| tab.zoom.moving(now))
        {
            cx.notify();
        }
    }

    fn place_started_core_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, started)) = &self.core_terminals.starting else {
            return;
        };
        let outcome = match started.try_recv() {
            Ok(outcome) => outcome,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.core_terminals.starting = None;
                return;
            }
        };
        let Some((purpose, _)) = self.core_terminals.starting.take() else {
            return;
        };
        let info = match outcome {
            Ok(info) => info,
            Err(error) => {
                let reason = error.to_string();
                self.show_flash(
                    Flash::warning(fl!("core-terminals-start-failed", reason = reason)),
                    cx,
                );
                return;
            }
        };
        match purpose {
            Purpose::Tab { scope, workspace } => {
                let location = Location::CoreTerminal {
                    panes: Panes::single(info.id),
                    observe: false,
                    remote: Remote::from_target(self.core_terminals.hub.target()),
                };
                let Some(id) = Store::update(cx, |store| {
                    store.open(scope, &workspace, Some(location), None)
                }) else {
                    self.show_flash(Flash::warning(fl!("core-terminals-too-many-tabs")), cx);
                    return;
                };
                self.sync_core_views(cx);
                self.show_browser_tab(id, window, cx);
            }
            Purpose::Split { tab, at, axis } => {
                self.place_split(tab, &at, axis, info.id, window, cx)
            }
        }
        cx.notify();
    }

    /// Gives a herdr-core tab's active pane the keyboard as the tab is
    /// shown, or as soon as its view exists for a restored tab still waiting
    /// on the hub.
    pub(crate) fn focus_core_terminal(&mut self, tab: TabId, window: &mut Window, cx: &mut App) {
        self.core_terminals.focus_wanted = None;
        match self.core_pane_view(tab) {
            Some(view) => window.focus(&view.read(cx).focus_handle().clone(), cx),
            None => self.core_terminals.focus_wanted = Some(tab),
        }
    }

    /// The view of `tab`'s active pane.
    fn core_pane_view(&self, tab: TabId) -> Option<&Entity<CoreTerminalView>> {
        let active = &self.core_terminals.tabs.get(&tab)?.active;
        self.core_terminals.views.get(&(tab, active.clone()))
    }

    /// How the window draws terminals, for its herdr-core views.
    pub(crate) fn core_look(&self) -> Look {
        Look {
            background: self.fill().base(self.terminal_theme().background),
            cell_height: self.config.terminal.line_height(),
            cell_width: self.cell_width,
            font: self.config.terminal.font(),
            ground: self.ground(),
            option_as_alt: self.config.option_as_alt,
            padding: self.config.terminal_padding,
        }
    }

    /// Keeps every view drawing as the window does after a font, theme, or
    /// option change.
    fn refresh_core_looks(&mut self, cx: &mut Context<Self>) {
        if self.core_terminals.shown().next().is_none() {
            return;
        }
        let look = self.core_look();
        if self.core_terminals.look.as_ref() == Some(&look) {
            return;
        }
        let views: Vec<Entity<CoreTerminalView>> = self.core_terminals.shown().cloned().collect();
        for view in views {
            view.update(cx, |view, cx| view.set_look(look.clone(), cx));
        }
        self.core_terminals.look = Some(look);
    }
}

/// Whether a pane waiting for the keyboard may take it: nothing holds it, or
/// only the window itself does.
fn keyboard_free(focused: Option<&FocusHandle>, window: &FocusHandle) -> bool {
    focused.is_none_or(|focused| focused == window)
}
