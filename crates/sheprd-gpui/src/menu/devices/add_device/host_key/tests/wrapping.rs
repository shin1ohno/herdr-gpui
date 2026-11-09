#![allow(clippy::unwrap_used)]
use crate::SheprdWindow;
use crate::sidebar::layout_tests::fixture_window;
use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px,
    size,
};
use sheprd_client::HostKeyFingerprint;

struct Prompt {
    view: Entity<SheprdWindow>,
}

impl Render for Prompt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .text_size(px(14.))
            .line_height(px(20.))
            .child(self.view.read(cx).render_ipad_host_key(
                "dev@computer.example",
                &[HostKeyFingerprint {
                    kind: "ED25519".into(),
                    bits: 256,
                    fingerprint: "SHA256:abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG".into(),
                }],
            ))
    }
}

#[gpui::test]
fn ipad_host_key_explanation_wraps_and_keeps_its_height_in_a_narrow_dialog(
    cx: &mut TestAppContext,
) {
    let view = {
        let (view, _) = cx.add_window_view(fixture_window);
        view
    };
    let (_, cx) = cx.add_window_view(|_, _| Prompt { view });
    cx.simulate_resize(size(px(900.), px(800.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let wide = cx.debug_bounds("device-setup-host-key-check").unwrap();

    cx.simulate_resize(size(px(320.), px(800.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let section = cx.debug_bounds("device-setup-host-key").unwrap();
    let advice = cx.debug_bounds("device-setup-host-key-check").unwrap();
    assert!(
        advice.size.height > wide.size.height,
        "{advice:?}, {wide:?}"
    );
    assert!(advice.left() >= section.left());
    assert!(advice.right() <= section.right());
    assert!(advice.bottom() <= section.bottom());
    assert!(section.size.width <= px(320.));
}
