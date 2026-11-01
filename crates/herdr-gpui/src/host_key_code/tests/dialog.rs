#![allow(clippy::unwrap_used)]

use super::*;
use crate::i18n::leak_tests::{assert_translated, draw, under_en_xa};
use core::prelude::v1::test;

fn ready(window: &mut Window, cx: &mut Context<HostKeyWindow>) -> HostKeyWindow {
    let mut view = HostKeyWindow::new(Config::default(), Theme::default(), window, cx);
    let code = crate::host_key_code::tests::code();
    let qr = Arc::new(Qr::encode(&code.to_url()).unwrap());
    view.state = State::Ready { code, qr };
    view
}

#[gpui::test]
fn desktop_dialog_draws_code_and_every_fingerprint_at_narrow_width(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(ready);
    cx.simulate_resize(size(px(420.), px(740.)));
    draw(cx);
    for selector in [
        "host-key-code-window",
        "host-key-code-qr",
        "host-key-code-copy",
        "host-key-code-close",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    let qr = cx.debug_bounds("host-key-code-qr").unwrap();
    assert!(qr.size.width <= px(420.));
    let point = cx.debug_bounds("host-key-code-copy").unwrap().center();
    cx.simulate_click(point, Default::default());
    let copied = cx.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap());
    view.read_with(cx, |view, _| {
        let State::Ready { code, .. } = &view.state else {
            panic!("not ready");
        };
        assert_eq!(copied, code.to_url());
    });
}

#[test]
fn desktop_dialog_messages_are_translated_under_en_xa() {
    under_en_xa("Host key code en-XA", |cx, recorder| {
        let (view, cx) = cx.add_window_view(ready);
        cx.simulate_resize(size(px(680.), px(1000.)));
        draw(cx);
        let data = view.read_with(cx, |view, _| {
            let State::Ready { code, .. } = &view.state else {
                panic!("not ready");
            };
            code.fingerprints()
                .iter()
                .flat_map(|key| [key.algorithm().to_owned(), key.fingerprint().to_owned()])
                .collect::<Vec<_>>()
        });
        assert_translated("Host-key code", &recorder.take(), &data);
        for state in [State::Loading, State::Failed(Error::NoHostKeys)] {
            view.update(cx, |view, cx| {
                view.state = state;
                cx.notify();
            });
            draw(cx);
            assert_translated("Host-key code state", &recorder.take(), &[]);
        }
    });
}
