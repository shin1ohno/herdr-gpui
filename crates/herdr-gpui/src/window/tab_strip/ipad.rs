//! Preparing touch tab faces without putting per-tab data on the strip context.
use super::*;
use crate::{config::FontConfig, tab_layout::ipad::TabHint};
use herdr_client::protocol::{ClientShellSnapshot, ClientShellTab};

pub(in crate::window) fn text_width(
    text: &SharedString,
    font: &FontConfig,
    window: &Window,
) -> f32 {
    let run = TextRun {
        len: text.len(),
        font: font.font(),
        color: black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(
        window
            .text_system()
            .shape_line(text.clone(), px(font.size), &[run], None)
            .width,
    )
    .ceil()
}

impl HerdrWindow {
    /// Compose the existing listing APIs: group_tabs applies hidden-tab policy,
    /// open_workspace_tabs supplies the global order used by TabNumber.
    pub(super) fn ipad_numbered_tabs(&self, cx: &App) -> Vec<Pick> {
        if !crate::ipad_ui::layout() {
            return Vec::new();
        }
        let visible: std::collections::HashSet<_> = self
            .group_slots()
            .into_iter()
            .flat_map(|slot| self.group_tabs(slot.id, cx))
            .collect();
        self.open_workspace_tabs(cx)
            .into_iter()
            .filter(|pick| visible.contains(pick))
            .take(9)
            .collect()
    }

    fn ipad_tab_hint(&self, pick: &Pick, numbered: &[Pick], window: &Window) -> Option<TabHint> {
        if !crate::ipad_ui::layout() || !self.ipad.keyboard.hardware(window) {
            return None;
        }
        let index = numbered.iter().position(|tab| tab == pick)?;
        let number = u8::try_from(index + 1).ok().filter(|number| *number <= 9)?;
        let text: SharedString = self.shortcut_hint(Command::TabNumber(number))?.into();
        Some(TabHint {
            style: self.config.shortcut_style,
            width: text_width(&text, &self.config.tabs, window) + 12.,
            text,
        })
    }

    pub(super) fn ipad_herdr_mark(
        &self,
        snapshot: &ClientShellSnapshot,
        tab: &ClientShellTab,
        numbered: &[Pick],
        window: &Window,
    ) -> Mark {
        let mut mark = herdr_mark(snapshot, tab);
        if crate::ipad_ui::layout()
            && let Mark::Herdr { hint, .. } = &mut mark
        {
            *hint = self.ipad_tab_hint(&Pick::Herdr(tab.tab_id.clone()), numbered, window);
        }
        mark
    }

    pub(super) fn ipad_page_extra(&self, pick: &Pick, numbered: &[Pick], window: &Window) -> f32 {
        self.ipad_tab_hint(pick, numbered, window)
            .map_or(0., |hint| hint.occupied_width())
    }

    /// iPad owns page faces as well as Herdr faces, so the shortcut remains
    /// per-tab data without changing the desktop browser renderer's contract.
    pub(super) fn touch_browser_tab_entries(
        &self,
        slot: Slot,
        shown: Option<crate::browser::TabId>,
        faces: &TabContext<'_>,
        numbered: &[Pick],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<(crate::browser::TabId, Stateful<Div>)> {
        if !crate::ipad_ui::layout() {
            return self.browser_tab_entries(slot, shown, faces, cx);
        }
        let layout = self.tab_layout();
        self.browser_tab_labels(cx)
            .into_iter()
            .filter_map(|(pick, label)| {
                let Pick::Page(id) = pick else {
                    return None;
                };
                if !self.group_lists(slot.id, &Pick::Page(id)) {
                    return None;
                }
                let tab = cx.try_global::<crate::browser::Store>()?.get(id)?;
                let icon = match (tab.place.editor(), &tab.location) {
                    (Some(editor), _) => editor.icon(),
                    (_, Some(crate::browser::Location::Review { .. })) => "icons/diff-unified.svg",
                    (_, Some(crate::browser::Location::Code { .. })) => "icons/code.svg",
                    (_, Some(crate::browser::Location::Orchestrator { .. })) => "icons/pulse.svg",
                    (_, Some(crate::browser::Location::Devices)) => "icons/devices.svg",
                    (_, Some(crate::browser::Location::CoreTerminal { .. })) => {
                        "icons/terminal.svg"
                    }
                    _ => "icons/globe.svg",
                };
                let mark = match self.ipad_tab_hint(&Pick::Page(id), numbered, window) {
                    Some(hint) => Mark::Hinted {
                        mark: Box::new(Mark::Icon(icon)),
                        hint,
                    },
                    None => Mark::Icon(icon),
                };
                let base = faces.stop(
                    div()
                        .id(SharedString::from(format!("browser-tab-{id}")))
                        .debug_selector(move || slot.selector(&format!("browser-tab-{id}")))
                        .role(Role::Tab)
                        .aria_label(label.clone())
                        .aria_selected(shown == Some(id)),
                );
                let face = TabFace {
                    label,
                    mark,
                    zoomed: false,
                    muted: false,
                    probe: Probe::Page(id),
                    close: Box::new(cx.listener(move |this, _, window, cx| {
                        this.close_strip_tab(Pick::Page(id), window, cx)
                    })),
                };
                Some((
                    id,
                    layout
                        .tab(
                            base,
                            face,
                            self.tab_state(shown == Some(id), slot.id),
                            faces,
                        )
                        .map(|tab| self.grow_tab(tab, slot.id, &Pick::Page(id)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.show_browser_tab_in(Some(slot.id), id, window, cx)
                        })),
                ))
            })
            .collect()
    }

    pub(super) fn touch_strip_new_tab(
        &self,
        slot: Slot,
        plus: Stateful<Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if crate::ipad_ui::layout() {
            plus.into_any_element()
        } else {
            self.strip_new_tab(slot, plus, window, cx)
        }
    }
}

/// The focused agent wins when several share a tab; W0 resolves ACP identities.
pub(super) fn herdr_mark(snapshot: &ClientShellSnapshot, tab: &ClientShellTab) -> Mark {
    let agent = snapshot
        .agents
        .iter()
        .filter(|agent| agent.tab_id == tab.tab_id)
        .max_by_key(|agent| agent.focused);
    Mark::Herdr {
        status: tab.agent_status,
        agent: agent.map(crate::icons::of_agent),
        chat: agent.is_some_and(|agent| {
            agent.agent.as_deref() == Some("acp")
                || agent
                    .tokens
                    .iter()
                    .any(|(key, _)| key == herdr_chat_protocol::TOKEN)
        }),
        hint: None,
    }
}
