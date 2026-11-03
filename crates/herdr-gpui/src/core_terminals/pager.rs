//! Sequential focus changes preserve the pane already filling a Core tab.
use crate::{
    HerdrWindow,
    browser::{Shown, TabId},
    ipad_keys::Page,
};
use gpui::{App, Context, Window};

impl HerdrWindow {
    pub(crate) fn ipad_core_position(&self, cx: &App) -> Option<(TabId, usize, usize)> {
        let Shown::Page(tab) = self.group_shown(self.active_group()?, cx) else {
            return None;
        };
        let shown = self.core_terminals.tabs.get(&tab)?;
        let panes = shown.panes.terminals();
        let current = panes.iter().position(|pane| **pane == shown.active)?;
        Some((tab, current, panes.len()))
    }

    pub(crate) fn ipad_core_page(
        &mut self,
        tab: TabId,
        page: Page,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(shown) = self.core_terminals.tabs.get_mut(&tab) else {
            return;
        };
        let panes = shown.panes.terminals();
        let Some(current) = panes.iter().position(|pane| **pane == shown.active) else {
            return;
        };
        let Some(next) = page.index(current, panes.len()) else {
            return;
        };
        let next = panes[next].clone();
        if shown.zoom.is_open() || self.ipad.pane_zoom.contains_key(&tab) {
            shown.set_zoom(Some(next.clone()));
        }
        shown.active = next;
        self.focus_core_terminal(tab, window, cx);
        cx.notify();
    }

    /// Restore only client presentation, including an explicit zoom that was
    /// already in use. The pane tree and its saved layout are never changed.
    pub(crate) fn ipad_core_single_pane(&mut self, single: bool, cx: &mut Context<Self>) {
        self.ipad
            .pane_zoom
            .retain(|tab, _| self.core_terminals.tabs.contains_key(tab));
        if !single {
            for (tab, restore) in self.ipad.pane_zoom.drain() {
                if let Some(shown) = self.core_terminals.tabs.get_mut(&tab) {
                    let restore = restore
                        .filter(|pane| shown.panes.contains(pane))
                        .map(|_| shown.active.clone());
                    shown.set_zoom(restore);
                    cx.notify();
                }
            }
            return;
        }
        let Some((tab, _, count)) = self.ipad_core_position(cx) else {
            return;
        };
        if count <= 1 || self.ipad.pane_zoom.contains_key(&tab) {
            return;
        }
        if let Some(shown) = self.core_terminals.tabs.get_mut(&tab) {
            let restore = shown.zoom.is_open().then(|| shown.zoomed.clone()).flatten();
            self.ipad.pane_zoom.insert(tab, restore);
            shown.set_zoom(Some(shown.active.clone()));
            cx.notify();
        }
    }
}
