//! Mounts the existing Settings editor in the owning window's menu layer.
// i18n: migrated
use crate::direction::Logical;
use crate::{HerdrWindow, menu::Page, settings_window::SettingsWindow};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(crate) fn install_settings_sheet(
        &mut self,
        model: Entity<SettingsWindow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.menu.page == Some(Page::Settings) {
            return self.menu.settings.as_ref() == Some(&model);
        }
        if !self.open_menu(window, cx) {
            return false;
        }
        self.menu.page = Some(Page::Settings);
        self.menu.settings = Some(model);
        cx.notify();
        true
    }

    pub(in crate::menu) fn flush_settings_sheet(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(model) = self.menu.settings.as_ref() else {
            return true;
        };
        if model.read(cx).sheet_ready() {
            self.menu.appear.keep_settings(model.clone());
            return true;
        }
        SettingsWindow::flush_for_dismiss(model.clone(), window.window_handle(), cx);
        false
    }

    pub(in crate::menu) fn settings_sheet_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dismiss =
            self.menu.settings.as_ref().is_some_and(|model| {
                model.update(cx, |view, cx| view.sheet_key(event, window, cx))
            });
        if dismiss {
            self.dismiss_menu(window, cx);
        }
    }

    pub(in crate::menu) fn render_settings_sheet(
        &self,
        leaving: Option<f32>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let now = crate::motion::now();
        let shown = leaving.unwrap_or_else(|| self.menu.appear.backdrop(now));
        let size = crate::settings_window::sheet_size(window);
        let panel = div()
            .id("menu-panel")
            .debug_selector(|| "menu-panel".into())
            .w(size.width)
            .h(size.height)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(crate::ipad_ui::metrics::SHEET_RADIUS))
            .border_1()
            .border_color(rgb(self.theme.active))
            .bg(rgb(self.theme.surface))
            .shadow_lg()
            .opacity(shown)
            .top(super::super::appear::Appear::offset(true, shown))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .children(self.menu.settings.clone());
        if leaving.is_some() {
            return div()
                .id("menu-leaving")
                .absolute()
                .inset_0()
                .row()
                .items_center()
                .justify_center()
                .map(|layer| crate::window::viewport::pad(layer, window))
                .child(panel);
        }
        div()
            .id("menu-overlay")
            .debug_selector(|| "menu-overlay".into())
            .role(Page::Settings.accessible_role())
            .aria_label(Page::Settings.accessible_name())
            .absolute()
            .inset_0()
            .row()
            .items_center()
            .justify_center()
            .map(|overlay| crate::window::viewport::pad(overlay, window))
            .bg(super::super::appear::sheet_backdrop(
                self.theme.background,
                self.menu.appear.backdrop(now),
            ))
            .occlude()
            .track_focus(&self.menu.focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, window, cx| {
                    cx.stop_propagation();
                    view.dismiss_menu(window, cx);
                }),
            )
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(Self::menu_key_down))
            .child(panel)
    }
}
