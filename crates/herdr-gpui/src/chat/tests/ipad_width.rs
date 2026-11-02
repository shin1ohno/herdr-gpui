use super::{
    overlay::{attach_with, open, poll},
    selection::acknowledge_size,
    *,
};
use crate::config::{ChatLayoutMode, Config, chat::ChatConfig};
use gpui::{TestAppContext, px, size};
use herdr_client::ConnectTarget;

#[test]
fn chat_default_layout_follows_the_target_and_explicit_choices_win() {
    let expected = if cfg!(target_os = "ios") {
        ChatLayoutMode::Bubbles
    } else {
        ChatLayoutMode::Timeline
    };
    assert_eq!(ChatLayoutMode::default(), expected);
    assert_eq!(ChatConfig::default().layout, expected);
    assert_eq!(Config::default().chat.layout, expected);
    assert_eq!(toml::from_str::<ChatConfig>("").unwrap().layout, expected);
    assert_eq!(
        toml::from_str::<ChatConfig>("layout = 'cards'")
            .unwrap()
            .layout,
        ChatLayoutMode::Cards
    );
    let _ipad = crate::ipad_ui::ipad_layout();
    assert_eq!(ChatLayoutMode::default(), expected);
}

#[gpui::test]
fn ipad_reading_column_centers_760_and_preserves_custom_full_and_narrow_widths(
    cx: &mut TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| {
        view.sidebar_width = Some(150.);
        view.config.sidebar_style.activity_bar = false;
    });
    cx.simulate_resize(size(px(1800.), px(860.)));
    cx.run_until_parked();
    let program = Program::new();
    let _stream = attach_with(
        &view,
        &program,
        snapshot(
            cursor(1, 0),
            vec![agent(1, &"A readable paragraph. ".repeat(80))],
        ),
        cx,
    );
    acknowledge_size(&view, cx);
    for &layout in ChatLayoutMode::ALL {
        view.update(cx, |view, cx| {
            view.set_chat_layout_with(layout, |_| Ok(()), cx)
        });
        for (width, expected) in [(800, Some(760.)), (1200, Some(1200.)), (0, None)] {
            view.update(cx, |view, _| view.config.chat.max_width = width);
            poll(&view, &program, cx);
            let column = cx.debug_bounds("chat-reading-column").unwrap();
            let pane = cx.debug_bounds("chat-view").unwrap();
            assert!(column.left() >= pane.left() && column.right() <= pane.right());
            if let Some(expected) = expected {
                assert!(
                    (column.size.width - px(expected)).abs() < px(2.),
                    "{layout:?}: {column:?}"
                );
            } else {
                assert!(column.size.width > px(1400.), "{layout:?}: {column:?}");
            }
            assert!((column.center().x - pane.center().x).abs() < px(2.));
            if width == 800 && layout == ChatLayoutMode::Bubbles {
                let measure = view.read_with(cx, |view, _| view.chat_text_size()) * 34.;
                assert!(cx.debug_bounds("chat-prose").unwrap().size.width <= px(measure + 1.));
            }
        }
    }
    cx.simulate_resize(size(px(650.), px(860.)));
    cx.run_until_parked();
    acknowledge_size(&view, cx);
    for width in [800, 1200, 0] {
        view.update(cx, |view, _| view.config.chat.max_width = width);
        poll(&view, &program, cx);
        let column = cx.debug_bounds("chat-reading-column").unwrap();
        let pane = cx.debug_bounds("chat-view").unwrap();
        assert!(column.size.width < px(500.));
        assert!(column.left() >= pane.left() && column.right() <= pane.right());
    }
}
