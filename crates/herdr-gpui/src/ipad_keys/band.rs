use super::{Key, Page, catalog::Side};
use crate::{HerdrWindow, browser::Shown, controls::Command, i18n::fl};
use gpui::{prelude::*, *};

pub(super) const BAND: f32 = 76.;
pub(super) const KEY_ROW: f32 = 56.;
pub(super) const PAGER: f32 = 36.;
const KEY: f32 = 44.;
const VISIBLE_DOTS: usize = 3;

impl HerdrWindow {
    pub(crate) fn ipad_layers(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        self.bind_ipad_latch(cx);
        self.sync_ipad_font(window, cx);
        if !crate::ipad_ui::layout() {
            return Vec::new();
        }
        let soft = Self::ipad_soft_keyboard_shown(window);
        self.ipad_core_single_pane(soft || self.ipad_compact(), cx);
        let mut layers = Vec::new();
        if self.ipad_compact() {
            layers.push(self.ipad_compact_layers(window, cx));
        } else if soft {
            layers.push(self.ipad_key_band(window, cx).into_any_element());
        }
        layers.extend(self.ipad_sidebar_overlay(window, cx));
        layers
    }

    fn ipad_key_band(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let pill = || {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(2.))
                .p(px(4.))
                .rounded(px(14.))
                .bg(rgb(self.theme.active))
        };
        div()
            .id("ipad-key-band")
            .debug_selector(|| "ipad-key-band".into())
            .flex_none()
            .h(px(BAND))
            .w_full()
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .bg(rgb(self.theme.surface))
            .overflow_x_scroll()
            .child(
                pill().children(
                    Key::ALL
                        .iter()
                        .filter(|key| key.side() == Side::Left)
                        .map(|key| self.ipad_key_button(*key, cx)),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(160.))
                    .children(self.ipad_pager(window, cx)),
            )
            .child(
                pill().children(
                    Key::ALL
                        .iter()
                        .filter(|key| key.side() == Side::Right)
                        .map(|key| self.ipad_key_button(*key, cx)),
                ),
            )
    }

    pub(super) fn ipad_key_button(&self, key: Key, cx: &mut Context<Self>) -> Stateful<Div> {
        let latched = match key {
            Key::Control => self.ipad.latch.control(),
            Key::Alt => self.ipad.latch.alt(),
            Key::More => self.ipad.more_keys.is_open(),
            _ => false,
        };
        let label = key.label();
        let foreground = if latched {
            self.theme.text_on(self.theme.primary())
        } else {
            self.theme.foreground
        };
        let mut button = div()
            .id(SharedString::from(format!("ipad-key-{key:?}")))
            .debug_selector(move || format!("ipad-key-{key:?}"))
            .role(Role::Button)
            .aria_label(label.clone())
            .flex_none()
            .w(px(key.width()))
            .h(px(KEY))
            .rounded(px(10.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .text_color(rgb(foreground))
            .when(latched, |button| button.bg(rgb(self.theme.primary())))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.ipad_key(key, window, cx);
            }));
        // GPUI paints an svg only with its own text color; it inherits none,
        // so the icon takes the label's color, latched or not.
        let icon = |path| svg().path(path).size(px(20.)).text_color(rgb(foreground));
        button = match key {
            Key::Hide => button.child(icon("icons/keyboard.svg")),
            Key::More => button.child(icon("icons/more.svg")),
            _ => button.child(label),
        };
        button
    }

    pub(super) fn ipad_pane_position(&self, cx: &App) -> Option<(usize, usize)> {
        if let Some((_, index, count)) = self.ipad_core_position(cx) {
            return Some((index, count));
        }
        if self
            .active_group()
            .is_some_and(|group| !matches!(self.group_shown(group, cx), Shown::Terminal))
        {
            return None;
        }
        let snapshot = self.live.snapshot.as_ref()?;
        let mut panes = snapshot.panes.iter().filter(|pane| {
            Some(&pane.workspace_id) == snapshot.focused_workspace_id.as_ref()
                && Some(&pane.tab_id) == snapshot.focused_tab_id.as_ref()
        });
        let index = panes
            .by_ref()
            .position(|pane| Some(&pane.pane_id) == snapshot.focused_pane_id.as_ref())?;
        Some((index, index + 1 + panes.count()))
    }

    pub(super) fn ipad_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if !crate::ipad_ui::active() || self.menu.page.is_some() {
            return;
        }
        if let Some((tab, _, _)) = self.ipad_core_position(cx) {
            self.ipad_core_page(tab, page, window, cx);
            return;
        }
        match page {
            Page::Next => self.command(Command::NextPane, window, cx),
            Page::Previous => self.command(Command::PreviousPane, window, cx),
            Page::At(index) => {
                let pane = self.live.snapshot.as_ref().and_then(|snapshot| {
                    snapshot
                        .panes
                        .iter()
                        .filter(|pane| {
                            Some(&pane.workspace_id) == snapshot.focused_workspace_id.as_ref()
                                && Some(&pane.tab_id) == snapshot.focused_tab_id.as_ref()
                        })
                        .nth(index)
                        .map(|pane| pane.pane_id.clone())
                });
                if let Some(pane) = pane
                    && let Some(endpoint) = self.endpoints.get(self.selected_endpoint)
                {
                    let endpoint = endpoint.id.clone();
                    self.navigate_endpoint(&endpoint, crate::NavigationTarget::Pane(&pane), cx);
                }
            }
        }
    }

    pub(super) fn ipad_pager(&self, _: &Window, cx: &mut Context<Self>) -> Option<Div> {
        let (index, count) = self.ipad_pane_position(cx)?;
        let owner = cx.weak_entity();
        // Bubble listeners run in reverse paint order. Register the strip
        // first so a dot can claim its own touch before the strip does.
        let mut row = drag_target(
            div()
                .debug_selector(|| "ipad-pane-pager".into())
                .h(px(PAGER))
                .flex_none()
                .min_w_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.))
                .relative(),
            owner.clone(),
            None,
        )
        .child(div().text_size(px(12.)).whitespace_nowrap().child(fl!(
            "ipad-keys-pager",
            index = index + 1,
            count = count
        )));
        let start = index
            .saturating_sub(1)
            .min(count.saturating_sub(VISIBLE_DOTS));
        for at in start..count.min(start + VISIBLE_DOTS) {
            let dot = div()
                .id(("ipad-pane-dot", at))
                .debug_selector(move || format!("ipad-pane-dot-{at}"))
                .role(Role::Button)
                .aria_label(fl!("ipad-keys-pager", index = at + 1, count = count))
                .w(px(KEY))
                .h(px(PAGER))
                .flex_none()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(8.))
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(self.theme.foreground))
                        .when(at == index, |dot| dot.bg(rgb(self.theme.foreground))),
                )
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.ipad_page(Page::At(at), window, cx);
                }));
            row = row.child(drag_target(dot, owner.clone(), Some(Page::At(at))));
        }
        Some(row)
    }
}

fn drag_target<E: ParentElement>(
    element: E,
    owner: WeakEntity<HerdrWindow>,
    tap: Option<Page>,
) -> E {
    element.child(
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                    if !crate::ipad_ui::active()
                        || phase != DispatchPhase::Bubble
                        || !hitbox.is_hovered_at(event.start_position, window)
                        || window.default_prevented()
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    if let Some(page) = drag_page(event, tap) {
                        let _ = owner.update(cx, |this, cx| this.ipad_page(page, window, cx));
                    }
                });
            },
        )
        .absolute()
        .inset_0(),
    )
}

pub(super) fn drag_page(event: &TouchDragEvent, tap: Option<Page>) -> Option<Page> {
    if event.phase != TouchPhase::Ended {
        return None;
    }
    let delta = event.position - event.start_position;
    let x = f32::from(delta.x);
    let y = f32::from(delta.y);
    if x.abs() >= 32. && x.abs() > y.abs() {
        Some(if x < 0. { Page::Next } else { Page::Previous })
    } else if x.abs() <= 8. && y.abs() <= 8. {
        tap
    } else {
        None
    }
}
