#![allow(clippy::unwrap_used)]

use crate::{SheprdWindow, sidebar::layout_tests::fixture_window};
use core::prelude::v1::test;
use gpui::{prelude::*, *};
use sheprd_client::HostKeyFingerprint;

struct Card {
    view: Entity<SheprdWindow>,
}

fn fingerprints() -> [HostKeyFingerprint; 1] {
    [HostKeyFingerprint {
        kind: "ED25519".into(),
        bits: 256,
        fingerprint: "SHA256:abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG".into(),
    }]
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(px(14.))
            .line_height(px(20.))
            .child(self.view.update(cx, |view, cx| {
                view.render_ipad_host_key_card("dev@card.test", &fingerprints(), cx)
            }))
    }
}

#[gpui::test]
fn direct_ipad_card_call_includes_warning_border_and_touch_actions(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        let mut card = view.render_ipad_host_key_card("dev@card.test", &fingerprints(), cx);
        assert_eq!(
            card.style().border_color,
            Some(crate::menu::colors::tint(&view.theme, 3).into())
        );
    });
    let (_, cx) = cx.add_window_view(|_, _| Card { view });
    cx.simulate_resize(size(px(320.), px(600.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let card = cx.debug_bounds("device-setup-host-key").unwrap();
    let fingerprints = cx.debug_bounds("device-setup-fingerprints").unwrap();
    let advice = cx.debug_bounds("device-setup-host-key-check").unwrap();
    for selector in ["device-setup-key-cancel", "device-setup-trust"] {
        let button = cx.debug_bounds(selector).unwrap();
        assert!(button.size.height >= px(44.));
        assert!(button.left() >= card.left() && button.right() <= card.right());
        assert!(button.bottom() <= card.bottom());
    }
    assert!(fingerprints.right() <= card.right());
    assert!(
        advice.bottom()
            <= cx
                .debug_bounds("device-setup-host-key-actions")
                .unwrap()
                .top()
    );
}

#[gpui::test]
fn card_cancel_returns_to_the_form_and_stale_trust_does_nothing(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.open_add_device(window, cx);
        let setup = view.menu.device_setup.as_mut().unwrap();
        setup.discovery = None;
        setup.step = super::super::super::Step::KeyCodeRefused;
    });
    let view_for_card = view.clone();
    let (_, cx) = cx.add_window_view(|_, _| Card {
        view: view_for_card,
    });
    cx.simulate_resize(size(px(620.), px(600.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let trust = cx.debug_bounds("device-setup-trust").unwrap().center();
    cx.simulate_click(trust, Modifiers::default());
    view.read_with(cx, |view, _| {
        let setup = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(
            setup.step,
            super::super::super::Step::KeyCodeRefused
        ));
        assert!(setup.task.is_none());
    });
    let cancel = cx.debug_bounds("device-setup-key-cancel").unwrap().center();
    cx.simulate_click(cancel, Modifiers::default());
    view.read_with(cx, |view, _| {
        let setup = view.menu.device_setup.as_ref().unwrap();
        assert!(matches!(setup.step, super::super::super::Step::Form));
        assert!(setup.task.is_none());
    });
}

#[test]
fn host_key_predicate_changes_only_with_the_layout_gate_on_the_host() {
    let _ipad = crate::ipad_ui::ipad_layout();
    assert!(super::super::ipad_host_key());
    let _desktop = crate::ipad_ui::desktop();
    assert_eq!(super::super::ipad_host_key(), cfg!(target_os = "ios"));
}
