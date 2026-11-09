#![allow(clippy::unwrap_used)]

use super::*;
use crate::contrast::{Contrast, ratio};
use core::prelude::v1::test;

#[test]
fn selected_tabs_use_themes_primary_and_readable_text_in_both_group_states() {
    for name in Theme::BUILTIN_NAMES {
        for contrast in [Contrast::Standard, Contrast::High] {
            let theme = Theme::builtin(name).unwrap().with_contrast(contrast);
            for active in [false, true] {
                let (fill, text) = Ipad.colors(
                    TabState {
                        selected: true,
                        active,
                    },
                    &theme,
                );
                assert_eq!(fill, theme.primary_wash(), "{name}");
                assert_ne!(fill, theme.active, "{name}: separate selection from rules");
                assert!(ratio(text, fill) >= 4.5, "{name}: title contrast");
                for background in [fill, theme.surface] {
                    assert!(
                        ratio(selection_color(&theme), background) >= contrast.mark_ratio(),
                        "{name}: selection underline contrast"
                    );
                }
                assert_eq!(
                    Ipad.colors(
                        TabState {
                            selected: false,
                            active
                        },
                        &theme
                    ),
                    (theme.surface, theme.subtext())
                );
            }
        }
    }
}

#[test]
fn nord_tab_selection_is_purple_instead_of_the_cyan_pane_accent() {
    let _ipad = crate::ipad_ui::ipad_layout();
    let theme = Theme::builtin("Nord").unwrap();
    assert_eq!(crate::ipad_ui::accent(&theme), 0x88c0d0);
    assert_eq!(theme.primary(), 0xb48ead);
    assert!(ratio(selection_color(&theme), theme.primary_wash()) >= 3.);
    assert_ne!(selection_color(&theme), crate::ipad_ui::accent(&theme));
}
