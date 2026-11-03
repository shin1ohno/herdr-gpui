#![allow(clippy::unwrap_used)]

use super::*;
use crate::{browser::Store, sidebar::layout_tests::fixture_window};
use core::prelude::v1::test;

#[gpui::test]
fn core_commits_consume_the_shared_latch_while_the_owner_is_leased(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(fixture_window);
    owner.update_in(cx, |owner, window, cx| {
        let tab = Store::update(cx, |store| {
            store.open(crate::browser::scope(&owner.endpoints[0]), "w1", None, None)
        })
        .unwrap();
        let id = TerminalId::new("latch-core").unwrap();
        let (link, commands) = Link::recording();
        let look = owner.core_look();
        let painter = owner.painter.clone();
        let core = cx.new(|cx| CoreTerminalView::new(id.clone(), link, look, painter, false, cx));
        owner.core_terminals.views.insert((tab, id), core.clone());
        owner.bind_ipad_latch(cx);
        owner.ipad.latch.toggle_control();
        core.update(cx, |view, cx| {
            window.focus(&view.focus, cx);
            view.screen.frame = Some(Arc::new(FrameData {
                width: 80,
                height: 24,
                cells: Vec::new(),
                cursor: None,
                hyperlinks: Vec::new(),
                graphics: Vec::new(),
            }));
            view.replace_text_in_range(None, "b", window, cx);
        });
        assert!(matches!(
            commands.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        assert!(
            owner.ipad.latch.take("b").is_none(),
            "the same latch was consumed"
        );
    });
    cx.run_until_parked();
    assert!(owner.read_with(cx, |owner, _| owner.prefix_armed));
    assert!(!owner.read_with(cx, |owner, _| owner.ipad.keyboard.synthetic));
}
