use super::*;
use crate::{
    config::ShortcutStyle,
    i18n::leak_tests::{assert_translated, draw, under_en_xa},
    settings_window::pages::keyboard::{KEYBOARD_ROWS, prefix_value, samples, shortcut_values},
};

#[test]
fn keyboard_ipad_style_translates_and_renders_resolved_data() {
    check_style(ShortcutStyle::Ipad);
}

#[test]
fn keyboard_herdr_style_translates_and_renders_resolved_data() {
    check_style(ShortcutStyle::HerdrPrefix);
}

fn check_style(style: ShortcutStyle) {
    under_en_xa("Keyboard en-XA", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let main = cx.add_window(crate::sidebar::layout_tests::fixture_window);
        let weak = cx.update(|cx| {
            main.update(cx, |source, _, cx| {
                source.config.keybindings =
                    keyboard::resolved_keys("new_tab = 'cmd-alt-t'\n").unwrap();
                cx.weak_entity()
            })
            .unwrap()
        });
        recorder.take();
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut view = SettingsWindow::new(weak, cx);
            view.section = Section::Keyboard;
            view.config.shortcut_style = style;
            view
        });
        cx.simulate_resize(size(px(1194.), px(1600.)));
        draw(cx);
        let data = view.read_with(cx, |view, cx| {
            let owner = view.source.upgrade().unwrap();
            let keymap = owner.read(cx).keymap();
            ShortcutStyle::ALL
                .iter()
                .flat_map(|&style| {
                    samples(keymap, style).into_iter().chain(
                        KEYBOARD_ROWS.iter().flat_map(move |(_, commands)| {
                            shortcut_values(keymap, commands, style)
                        }),
                    )
                })
                .chain([prefix_value(keymap)])
                .collect::<Vec<_>>()
        });
        let shown = recorder.take();
        assert!(
            shown.iter().any(|line| line == "⌘⌥T"),
            "the real table/sample must follow the custom keymap: {shown:?}"
        );
        assert_translated("Settings > Keyboard", &shown, &data);
        for entry in catalog::CATALOG
            .iter()
            .filter(|entry| entry.section == Section::Keyboard)
        {
            assert_eq!(entry.availability, catalog::Availability::Ipad);
            assert!(cx.debug_bounds(entry.id).is_some(), "missing {}", entry.id);
        }
    });
}

#[test]
fn keyboard_compact_sheet_keeps_the_table_inside_the_page() {
    under_en_xa("Keyboard compact en-XA", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
        cx.simulate_resize(size(px(320.), px(834.)));
        let fixture = cx.update(|_, cx| sheet::Fixture::prepare(owner.downgrade(), false, cx));
        draw(cx);
        recorder.take();
        cx.update(|_, cx| open_at(owner.downgrade(), Some(Section::Keyboard), cx));
        cx.run_until_parked();
        draw(cx);
        let table = cx.debug_bounds("keyboard-shortcuts").unwrap();
        assert!(table.size.width <= px(288.), "{table:?}");
        assert!(table.size.width > px(0.));
        let data = fixture.model.read_with(cx, |view, cx| {
            let owner = view.source.upgrade().unwrap();
            let keymap = owner.read(cx).keymap();
            ShortcutStyle::ALL
                .iter()
                .flat_map(|&style| {
                    samples(keymap, style).into_iter().chain(
                        KEYBOARD_ROWS.iter().flat_map(move |(_, commands)| {
                            shortcut_values(keymap, commands, style)
                        }),
                    )
                })
                .chain([prefix_value(keymap)])
                .collect::<Vec<_>>()
        });
        assert_translated("Keyboard compact sheet", &recorder.take(), &data);
    });
}
