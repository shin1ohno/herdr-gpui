//! The in-window shell. The editor and its asynchronous save chain stay shared.
// i18n: migrated
use super::*;

pub(super) const COMPACT_BELOW: f32 = 560.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Shell {
    Window,
    Sheet,
}

#[derive(Default)]
pub(super) struct Sheet {
    pub(super) page: bool,
    ready: bool,
    generation: u64,
}

pub(crate) fn sheet_size(window: &Window) -> Size<Pixels> {
    use crate::ipad_ui::metrics::{SETTINGS_SHEET_HEIGHT, SETTINGS_SHEET_WIDTH};
    let visible = window.fully_visible_bounds().size;
    size(
        (visible.width - px(32.))
            .max(px(0.))
            .min(px(SETTINGS_SHEET_WIDTH)),
        (visible.height - px(32.))
            .max(px(0.))
            .min(px(SETTINGS_SHEET_HEIGHT)),
    )
}

fn source_window(source: &WeakEntity<HerdrWindow>, cx: &App) -> Option<AnyWindowHandle> {
    cx.windows().into_iter().find(|handle| {
        handle.downcast::<HerdrWindow>().is_some_and(|handle| {
            handle
                .entity(cx)
                .is_ok_and(|entity| entity.entity_id() == source.entity_id())
        })
    })
}

pub(super) fn open_with(
    source: WeakEntity<HerdrWindow>,
    cx: &mut App,
    initialize: impl FnOnce(&mut SettingsWindow, &mut Window, &mut Context<SettingsWindow>) + 'static,
) {
    let retained = cx
        .default_global::<SettingsWindowHandle>()
        .model
        .as_ref()
        .and_then(WeakEntity::upgrade);
    // Like the desktop singleton, an editor already mounted in a scene stays
    // there. Never mount its live drafts in two trees at once.
    let owner = retained
        .as_ref()
        .filter(|model| {
            let view = model.read(cx);
            view.shell == Shell::Sheet
                && view.source.upgrade().is_some_and(|source| {
                    source
                        .read(cx)
                        .menu
                        .settings
                        .as_ref()
                        .is_some_and(|shown| shown == *model)
                })
        })
        .map(|model| model.read(cx).source.clone())
        .unwrap_or(source.clone());
    let Some(handle) = source_window(&owner, cx) else {
        return;
    };
    let _ = handle.update(cx, |_, window, cx| {
        let model = retained.unwrap_or_else(|| {
            cx.new(|cx| {
                let mut view = SettingsWindow::new(source.clone(), cx);
                view.shell = Shell::Sheet;
                view.section = Section::General;
                initialize(&mut view, window, cx);
                view
            })
        });
        model.update(cx, |view, cx| {
            view.retarget_source(owner.clone(), cx);
            view.shell = Shell::Sheet;
            view.sheet.ready = false;
            view.sheet.generation = view.sheet.generation.wrapping_add(1);
            view.closing = None;
        });
        let Some(owner) = owner.upgrade() else {
            return;
        };
        let installed = owner.update(cx, |owner, cx| {
            owner.install_settings_sheet(model.clone(), window, cx)
        });
        if installed {
            let registry = cx.default_global::<SettingsWindowHandle>();
            registry.window = None;
            registry.model = Some(model.downgrade());
            let focus = model.read(cx).focus.clone();
            window.focus(&focus, cx);
            window.activate_window();
        }
    });
}

pub(super) fn select_open_section(section: Section, cx: &mut App) {
    if !crate::ipad_ui::layout() {
        if let Some(handle) = cx.default_global::<SettingsWindowHandle>().window {
            let _ = handle.update(cx, |view, window, cx| {
                view.select_section(section, window, cx)
            });
        }
        return;
    }
    let Some(model) = cx
        .default_global::<SettingsWindowHandle>()
        .model
        .as_ref()
        .and_then(WeakEntity::upgrade)
    else {
        return;
    };
    let Some(handle) = source_window(&model.read(cx).source, cx) else {
        return;
    };
    let _ = handle.update(cx, |_, window, cx| {
        model.update(cx, |view, cx| view.select_section(section, window, cx))
    });
}

impl SettingsWindow {
    pub(super) fn shown_section(&self, section: Section) -> Section {
        if self.shell == Shell::Sheet && !Section::shown().any(|shown| shown == section) {
            Section::General
        } else {
            section
        }
    }

    pub(super) fn filter_sheet_search(&mut self) {
        if self.shell == Shell::Sheet
            && let Some(results) = &mut self.search_results
        {
            results.retain(|index| {
                catalog::CATALOG
                    .get(*index)
                    .is_some_and(|entry| Section::shown().any(|section| section == entry.section))
            });
        }
    }

    pub(crate) fn sheet_ready(&self) -> bool {
        self.sheet.ready
    }

    /// The owner may hold a mutable lease when a backdrop/key asks to close.
    /// Defer the editor's flush, which can publish drafts back to that owner.
    pub(crate) fn flush_for_dismiss(model: Entity<Self>, handle: AnyWindowHandle, cx: &mut App) {
        let generation = model.read(cx).sheet.generation;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                model.update(cx, |view, cx| {
                    if view.sheet.generation == generation && view.should_close(window, cx) {
                        view.finish_shell_close(handle, cx);
                    }
                })
            });
        });
    }

    pub(super) fn request_sheet_dismiss(&self, window: &Window, cx: &mut Context<Self>) {
        self.request_sheet_dismiss_for_handle(window.window_handle(), cx);
    }

    pub(super) fn finish_shell_close(&mut self, handle: AnyWindowHandle, cx: &mut Context<Self>) {
        if self.shell == Shell::Sheet {
            self.sheet.ready = true;
            self.closing = None;
            self.request_sheet_dismiss_for_handle(handle, cx);
        } else {
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            });
        }
    }

    fn request_sheet_dismiss_for_handle(&self, handle: AnyWindowHandle, cx: &mut Context<Self>) {
        let source = self.source.clone();
        let model = cx.weak_entity();
        let generation = self.sheet.generation;
        cx.defer(move |cx| {
            let current = model
                .upgrade()
                .is_some_and(|model| model.read(cx).sheet.generation == generation);
            if !current {
                return;
            }
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(source) = source.upgrade() {
                    source.update(cx, |source, cx| {
                        if source
                            .menu
                            .settings
                            .as_ref()
                            .is_some_and(|shown| shown.entity_id() == model.entity_id())
                        {
                            source.dismiss_menu(window, cx);
                        }
                    });
                }
            });
        });
    }

    pub(crate) fn sheet_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        if key == "w" && (event.keystroke.modifiers.control || event.keystroke.modifiers.platform) {
            cx.stop_propagation();
            window.prevent_default();
            return true;
        }
        let inputs = std::iter::once(&self.nav_search)
            .chain(self.number_edit.as_ref().map(|edit| &edit.input))
            .chain(self.hex_edit.as_ref().map(|edit| &edit.input));
        let mut known_input = false;
        for input in inputs {
            let input = input.read(cx);
            if input.focus.is_focused(window) {
                known_input = true;
                if input.is_composing() {
                    return false;
                }
            }
        }
        let root = self.focus.is_focused(window);
        if !root && !known_input && self.dropdown.is_none() {
            return false;
        }
        if !root
            && !matches!(
                key,
                "escape"
                    | "enter"
                    | "up"
                    | "down"
                    | "left"
                    | "right"
                    | "pageup"
                    | "pagedown"
                    | "tab"
            )
        {
            return false;
        }
        let local = self.dropdown.is_some()
            || self.number_edit.is_some()
            || self.hex_edit.is_some()
            || (self.search_results.is_some() && self.nav_search.read(cx).focus.is_focused(window))
            || self.theme_hovered();
        self.settings_key(event, window, cx);
        if root || key == "escape" {
            cx.stop_propagation();
            window.prevent_default();
        }
        key == "escape" && !local
    }

    pub(super) fn render_sheet(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.viewport_width = f32::from(sheet_size(window).width);
        self.filter_sheet_search();
        let compact = self.viewport_width < COMPACT_BELOW;
        let page = !compact || self.sheet.page || self.search_results.is_some();
        let heading = if self.search_results.is_some() {
            fl!("settings-results")
        } else {
            self.section.label()
        };
        let mut body = div().flex_1().min_h_0().min_w_0().row().overflow_hidden();
        if !compact || !page {
            body = body.child(self.navigation(cx));
        }
        if page {
            let content = if self.search_results.is_some() {
                self.render_search_results(cx).into_any_element()
            } else {
                match self.section {
                    Section::Theme => self.render_theme(window, cx).into_any_element(),
                    Section::Integrations => {
                        self.render_integration_controls(cx).into_any_element()
                    }
                    Section::Plugins => self.render_plugin_controls(cx).into_any_element(),
                    Section::Code => self.render_code_controls(window, cx).into_any_element(),
                    _ => self.render_controls(window, cx).into_any_element(),
                }
            };
            let status = self.footer_text();
            body = body.child(
                self.scroll_area(
                    scrollbar::Pane::Body,
                    div()
                        .id("settings-body")
                        .debug_selector(|| "settings-body".into())
                        .role(Role::TabPanel)
                        .aria_label(heading.clone())
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.body_scroll)
                        .p(px(if compact { 16. } else { 28. }))
                        .child(
                            div()
                                .text_size(px(18.))
                                .line_height(px(26.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(heading),
                        )
                        .children(
                            self.section
                                .lead()
                                .filter(|_| self.search_results.is_none())
                                .map(|lead| div().mb(px(16.)).child(lead)),
                        )
                        .child(content)
                        .child(
                            div()
                                .id("ipad-settings-sheet-status")
                                .debug_selector(|| "ipad-settings-sheet-status".into())
                                .role(Role::Status)
                                .aria_label(status.clone())
                                .mt(px(16.))
                                .text_color(rgb(self.theme.subtext()))
                                .child(status),
                        ),
                    cx,
                ),
            );
        }
        let body = div()
            .debug_selector(|| "ipad-settings-sheet-body".into())
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .children(self.error.as_ref().map(|error| {
                div()
                    .id("ipad-settings-sheet-error")
                    .debug_selector(|| "ipad-settings-sheet-error".into())
                    .role(Role::Status)
                    .aria_label(error.clone())
                    .flex_none()
                    .max_h(px(80.))
                    .overflow_y_scroll()
                    .p(px(12.))
                    .child(error.clone())
            }))
            .when(compact && self.search_results.is_some(), |body| {
                body.child(
                    div()
                        .debug_selector(|| "settings-nav-search".into())
                        .flex_none()
                        .p(px(12.))
                        .child(self.nav_search.clone()),
                )
            })
            .child(body);
        let header = div()
            .debug_selector(|| "ipad-settings-sheet-header".into())
            .h(px(60.))
            .flex_none()
            .px(px(8.))
            .row()
            .items_center()
            .border_b_1()
            .border_color(rgb(self.theme.active))
            .child(div().w(px(72.)).flex_none().when(compact && page, |slot| {
                slot.child(
                    self.sheet_button("ipad-settings-sheet-back", fl!("ipad-settings-sheet-back"))
                        .on_click(cx.listener(Self::back_to_sections)),
                )
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(fl!("ipad-settings-sheet-title")),
            )
            .child(
                self.sheet_button("ipad-settings-sheet-done", fl!("ipad-settings-sheet-done"))
                    .w(px(72.))
                    .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
            );
        div()
            .id("ipad-settings-sheet")
            .debug_selector(|| "ipad-settings-sheet".into())
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .text_font(&self.config.ui)
            .text_size(px(12.))
            .line_height(px(18.))
            .text_start()
            .bg(rgb(self.theme.surface))
            .text_color(rgb(self.theme.foreground))
            .on_action(cx.listener(|view, action: &crate::RunCommand, window, cx| {
                cx.stop_propagation();
                match action.command {
                    crate::controls::Command::CloseWindow => view.close(window, cx),
                    crate::controls::Command::Settings => window.focus(&view.focus, cx),
                    _ => {}
                }
            }))
            .on_action(cx.listener(|view, _: &Close, window, cx| {
                cx.stop_propagation();
                view.close(window, cx);
            }))
            .child(header)
            .child(body)
            .children(self.render_control_font_picker(window, cx))
    }

    fn back_to_sections(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Match section navigation before removing editors from the tree.
        self.hover_theme(None, cx);
        self.drop_number_preview(cx);
        if !self.finish_number_edit(true, cx) {
            self.finish_number_edit(false, cx);
        }
        if !self.finish_hex_edit(true, cx) {
            self.finish_hex_edit(false, cx);
        }
        if !self.finish_code_edit(true, cx) {
            self.finish_code_edit(false, cx);
        }
        self.dropdown = None;
        self.dismiss_control_font_picker(window, cx);
        self.sheet.page = false;
        self.clear_search(cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn sheet_button(&self, id: &'static str, label: String) -> Stateful<Div> {
        div()
            .id(id)
            .debug_selector(move || id.into())
            .role(Role::Button)
            .aria_label(label.clone())
            .h(px(44.))
            .min_w(px(44.))
            .px(px(8.))
            .row()
            .items_center()
            .justify_center()
            .rounded(px(corners::CONTROL))
            .cursor_pointer()
            .text_color(rgb(crate::ipad_ui::accent(&self.theme)))
            .hover(|style| style.bg(rgb(self.theme.active)))
            .child(label)
    }
}
