//! The touch search entry shares the sidebar's query and navigation model.
use super::{super::agents::Indicators, Target};
use crate::{
    Command, HerdrWindow, direction::Logical, i18n::fl, ipad_ui::metrics::TOUCH, menu::Page,
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    pub(in crate::sidebar) fn sidebar_search_trigger(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.sidebar_search_icon(id, "icons/search.svg", fl!("sidebar-search-spaces-label"))
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                this.open_sidebar_search_popup(
                    event.position() + point(px(0.), px(TOUCH / 2.)),
                    window,
                    cx,
                );
            }))
    }

    pub(in crate::sidebar) fn sidebar_new_space_trigger(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.sidebar_search_icon(id, "icons/plus.svg", fl!("ipad-sidebar-new-space"))
            .on_click(
                cx.listener(|this, _, window, cx| this.command(Command::Workspace, window, cx)),
            )
    }

    fn sidebar_search_icon(
        &self,
        id: &'static str,
        icon: &'static str,
        label: String,
    ) -> Stateful<Div> {
        let (foreground, surface) = (self.theme.foreground, self.theme.surface);
        let hint: SharedString = label.clone().into();
        div()
            .id(id)
            .debug_selector(move || id.into())
            .map(|button| self.sidebar_button(button, label))
            .size(px(TOUCH))
            .flex_none()
            .row()
            .items_center()
            .justify_center()
            .rounded(px(crate::config::corners::CONTROL))
            .cursor_pointer()
            .hover(|button| button.bg(rgb(self.theme.active)))
            .tooltip(move |_, cx| {
                cx.new(|_| crate::menu::SettingsHint {
                    text: hint.clone(),
                    foreground,
                    surface,
                })
                .into()
            })
            .child(svg().path(icon).size(px(18.)).text_color(rgb(foreground)))
    }

    pub(crate) fn open_sidebar_search_popup(
        &mut self,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu.page == Some(Page::SidebarSearch) {
            window.focus(&self.sidebar_search.focus(cx), cx);
            return;
        }
        if self.menu.page.is_some() {
            return;
        }
        let return_focus = window.focused(cx);
        if !self.open_menu(window, cx) {
            return;
        }
        self.sidebar_search.return_focus = return_focus;
        self.sidebar_search.input.update(cx, |input, cx| {
            input.clear(cx);
            input.set_label(&fl!("sidebar-search-spaces-label"), cx);
            input.set_placeholder(&fl!("sidebar-search-spaces-placeholder"), cx);
            window.focus(&input.focus, cx);
        });
        self.menu.anchor = anchor;
        self.menu.page = Some(Page::SidebarSearch);
        cx.notify();
    }

    pub(crate) fn take_sidebar_search_return_focus(&mut self) -> Option<FocusHandle> {
        (self.menu.page == Some(Page::SidebarSearch))
            .then(|| self.sidebar_search.return_focus.take())
            .flatten()
    }

    pub(crate) fn sidebar_search_popup_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu.page == Some(Page::SidebarSearch) {
            self.sidebar_search_key(event, window, cx);
        }
    }

    pub(super) fn sidebar_search_target_available(&self, target: &Target) -> bool {
        let id = match target {
            Target::Device { endpoint } | Target::Workspace { endpoint, .. } => endpoint,
        };
        let Some((index, endpoint)) = self
            .endpoints
            .iter()
            .enumerate()
            .find(|(_, endpoint)| endpoint.id == *id)
        else {
            return false;
        };
        if !endpoint.enabled {
            return false;
        }
        let live = if index == self.selected_endpoint {
            &self.live
        } else {
            &endpoint.live
        };
        match target {
            Target::Device { .. } => true,
            Target::Workspace { workspace, .. } => {
                live.status.is_connected()
                    && live.snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot
                            .workspaces
                            .iter()
                            .any(|candidate| candidate.workspace_id == *workspace)
                    })
            }
        }
    }

    pub(crate) fn render_sidebar_search_popup(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let look = crate::sidebar::layout::for_config(&self.config, self.fill());
        let indicators = Indicators::new(
            self.settings.shared.as_ref(),
            self.config.status_icons,
            false,
            self.theme_light,
            &self.theme,
        );
        div()
            .id("sidebar-search-popup")
            .debug_selector(|| "sidebar-search-popup".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap(px(8.))
            .child(
                div()
                    .row()
                    .items_center()
                    .min_w_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(fl!("sidebar-search-spaces-label")),
                    )
                    .child(
                        self.sidebar_search_icon(
                            "sidebar-search-close",
                            "icons/x.svg",
                            fl!("sidebar-search-close"),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.menu.page == Some(Page::SidebarSearch) {
                                this.dismiss_menu(window, cx);
                            }
                        })),
                    ),
            )
            .child(
                div()
                    .id("sidebar-search")
                    .debug_selector(|| "sidebar-search".into())
                    .h(px(TOUCH))
                    .flex_none()
                    .row()
                    .items_center()
                    .min_w_0()
                    .px(px(8.))
                    .rounded(px(crate::config::corners::CONTROL))
                    .border_1()
                    .border_color(rgb(self.theme.active))
                    .child(self.sidebar_search.input.clone()),
            )
            .when_some(self.menu.error.clone(), |body, error| {
                body.child(
                    div()
                        .id("sidebar-search-error")
                        .text_color(rgb(self.theme.palette[1]))
                        .child(error),
                )
            })
            .when(self.sidebar_search.query().is_none(), |body| {
                body.child(
                    div()
                        .text_color(rgb(self.theme.muted))
                        .child(fl!("sidebar-search-start")),
                )
            })
            .when(self.sidebar_search.query().is_some(), |body| {
                body.child(self.sidebar_search_results(
                    self.sidebar_search_hits(cx),
                    indicators,
                    look,
                    cx,
                ))
            })
    }
}
