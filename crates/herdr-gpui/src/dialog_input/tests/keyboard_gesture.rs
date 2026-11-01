use super::DialogInput;
use crate::{HerdrWindow, menu::Page};
use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler as _, Modifiers, Render, Window,
    prelude::*,
};

struct Field(Entity<HerdrWindow>);

impl Render for Field {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.0.update(cx, |view, cx| view.render_dialog_input(cx))
    }
}

#[gpui::test]
fn repeated_dialog_field_taps_keep_text_input_and_composition(cx: &mut gpui::TestAppContext) {
    let (field, cx) = cx.add_window_view(|window, cx| {
        Field(cx.new(|cx| {
            let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
            view.menu.input = Some(DialogInput::default());
            view.menu.page = Some(Page::RenameTab);
            view
        }))
    });
    let view = field.read_with(cx, |field, _| field.0.clone());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = cx.debug_bounds("dialog-input").unwrap_or_default();
    cx.simulate_click(bounds.center(), Modifiers::none());
    assert!(cx.update(|window, cx| view.read(cx).menu.focus.is_focused(window)));
    cx.simulate_input("draft");
    assert_eq!(
        view.read_with(cx, |view, _| {
            view.menu.input.as_ref().map(|input| input.text.clone())
        }),
        Some("draft".into()),
    );
    cx.update(|window, _| window.dismiss_virtual_keyboard());
    cx.simulate_click(bounds.center(), Modifiers::none());
    assert!(cx.update(|window, cx| view.read(cx).menu.focus.is_focused(window)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.replace_and_mark_text_in_range(None, "中文", Some(0..2), window, cx);
        });
        window.draw(cx).clear(cx);
    });
    let before = view.read_with(cx, |view, _| {
        view.menu.input.as_ref().map(|input| {
            (
                input.text.clone(),
                input.selection.clone(),
                input.marked.clone(),
            )
        })
    });
    cx.update(|window, _| window.dismiss_virtual_keyboard());
    cx.simulate_click(bounds.center(), Modifiers::none());
    assert_eq!(
        view.read_with(cx, |view, _| {
            view.menu.input.as_ref().map(|input| {
                (
                    input.text.clone(),
                    input.selection.clone(),
                    input.marked.clone(),
                )
            })
        }),
        before,
    );
}
