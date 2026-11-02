use super::*;
use crate::menu::devices::add_device::actions::{Primary, step_actions};
use gpui::{Modifiers, VisualContext as _, rgba};
use herdr_client::HostProbe;

fn open(cx: &mut TestAppContext) -> (gpui::Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    view.update_in(cx, |view, window, cx| {
        view.open_add_device(window, cx);
        view.menu.device_setup.as_mut().unwrap().discovery = None;
    });
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn ssh_sheet_has_fixed_width_header_radius_and_theme_backdrop(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = open(cx);
    let request = setup::Request::new("dev@sheet.test", "Remote", "").unwrap();
    for step in [
        Step::Form,
        Step::Checking(request.clone()),
        Step::AwaitingLocalNetwork(request.clone()),
        Step::Saving(request.clone(), HostProbe::Running),
        Step::Saved,
        Step::SavedWarning("add-device-unexpected-response"),
        Step::FetchingKey(request.clone()),
        Step::KeyCodeRefused,
        Step::Confirm(request.clone(), Offer::Update),
        Step::Verifying(request.clone(), Offer::Update),
        Step::Activating(
            request.clone(),
            setup::TerminalSetup::fixture("unused"),
            Offer::Update,
        ),
        Step::Opening(
            request.clone(),
            super::super::LocalSpace {
                request: "request".into(),
                boot: "boot".into(),
                setup: setup::TerminalSetup::fixture("unused"),
                offer: Offer::Update,
            },
        ),
        #[cfg(not(target_os = "ios"))]
        Step::TrustKey(
            request.clone(),
            herdr_client::NewHostKey::fixture_for_tests(
                std::path::PathBuf::from("unused"),
                Vec::new(),
            ),
        ),
    ] {
        view.update(cx, |view, cx| {
            view.menu.device_setup.as_mut().unwrap().step = step;
            cx.notify();
        });
        draw(cx);
        let panel = cx.debug_bounds("menu-panel").unwrap();
        assert_eq!(panel.size.width, px(620.));
        assert_eq!(
            cx.debug_bounds("device-setup-header").unwrap().size.height,
            px(60.)
        );
        cx.update(|window, _| {
            let panel = panel.scale(window.scale_factor());
            let quads = window.painted_quads();
            assert!(quads.iter().any(|quad| quad.bounds == panel
                && (quad.corner_radii.top_left.0 - 18. * window.scale_factor()).abs() < 0.01));
        });
    }
    draw(cx);
    cx.update(|window, cx| {
        let background = view.read(cx).theme.background;
        assert!(window.painted_quads().iter().any(|quad| {
            quad.background.as_solid() == Some(rgba((background << 8) | 168).into())
        }));
    });
}

#[gpui::test]
fn sheet_fields_have_touch_frames_columns_and_padding_focus(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = open(cx);
    draw(cx);
    for selector in [
        "device-setup-input-0",
        "device-setup-input-1",
        "device-setup-input-2",
    ] {
        assert_eq!(cx.debug_bounds(selector).unwrap().size.height, px(48.));
    }
    let label = cx.debug_bounds("device-setup-input-1").unwrap();
    let session = cx.debug_bounds("device-setup-input-2").unwrap();
    assert_eq!(label.top(), session.top());
    assert_eq!(label.size.width, session.size.width);
    assert!(label.right() < session.left());
    view.read_with(cx, |view, cx| {
        assert!(
            view.menu
                .device_setup
                .as_ref()
                .unwrap()
                .fields
                .iter()
                .all(|field| !field.read(cx).framed())
        );
    });
    // Padding belongs to the field too, not just the text's narrow line.
    cx.simulate_click(label.origin + point(px(2.), px(2.)), Modifiers::default());
    cx.update(|window, cx| {
        assert!(
            view.read(cx).menu.device_setup.as_ref().unwrap().fields[1]
                .read(cx)
                .focus
                .is_focused(window)
        );
    });
}

#[gpui::test]
fn sheet_header_keeps_primary_visible_with_keyboard_and_long_body(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = open(cx);
    for width in [1194., 360.] {
        cx.simulate_resize(size(px(width), px(834.)));
        let visible = Bounds::new(point(px(0.), px(24.)), size(px(width), px(420.)));
        cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
        view.update(cx, |view, cx| {
            view.menu.device_setup.as_mut().unwrap().step = Step::Form;
            view.set_device_error(Some("Long diagnostic\n".repeat(40)));
            cx.notify();
        });
        draw(cx);
        let panel = cx.debug_bounds("menu-panel").unwrap();
        assert!(panel.size.width <= px(width - 32.));
        for selector in [
            "device-setup-header",
            "device-setup-sheet-cancel",
            "device-setup-header-primary",
        ] {
            let bounds = cx.debug_bounds(selector).unwrap();
            assert!(
                bounds.left() >= visible.left() && bounds.right() <= visible.right(),
                "{selector}: {bounds:?}"
            );
            assert!(
                bounds.top() >= visible.top() && bounds.bottom() <= visible.bottom(),
                "{selector}: {bounds:?}"
            );
        }
        assert!(cx.debug_bounds("device-setup-footer").is_none());
    }
}

#[gpui::test]
fn sheet_style_is_ssh_only_and_desktop_keeps_its_frame(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    draw(cx);
    let desktop = cx.debug_bounds("menu-panel").unwrap();
    assert!(cx.debug_bounds("device-setup-close").is_some());
    let _ipad = crate::ipad_ui::ipad_layout();
    view.update(cx, |view, cx| {
        view.menu.page = Some(Page::AddWsl);
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        cx.debug_bounds("menu-panel").unwrap().size.width,
        desktop.size.width
    );
    assert!(cx.debug_bounds("device-setup-header-primary").is_none());
    #[cfg(feature = "cloud")]
    {
        view.update(cx, |view, cx| {
            view.menu.page = Some(Page::AddProvider);
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            cx.debug_bounds("menu-panel").unwrap().size.width,
            desktop.size.width
        );
        assert!(cx.debug_bounds("device-setup-header-primary").is_none());
    }
}

#[test]
fn sheet_backdrop_clamps_and_scales_without_changing_desktop_dim() {
    use crate::menu::appear::{backdrop, sheet_backdrop};
    for (shown, alpha) in [(-1., 0), (0., 0), (0.5, 84), (1., 168), (2., 168)] {
        assert_eq!(sheet_backdrop(0x123456, shown), rgba(0x12345600 | alpha));
    }
    assert_eq!(backdrop(0x123456, 1.), rgba(0x123456b0));
}

#[test]
fn step_actions_keep_trust_and_recovery_separate_from_submission() {
    let request = setup::Request::new("dev@sheet.test", "Remote", "").unwrap();
    assert_eq!(step_actions(&Step::Form).primary, Some(Primary::Submit));
    assert!(!step_actions(&Step::Form).cancel);
    assert_eq!(
        step_actions(&Step::Checking(request.clone())).primary,
        Some(Primary::Checking)
    );
    assert!(step_actions(&Step::KeyCodeRefused).primary.is_none());
    let awaiting = step_actions(&Step::AwaitingLocalNetwork(request.clone()));
    assert!(awaiting.cancel && awaiting.primary.is_none());
    #[cfg(not(target_os = "ios"))]
    {
        let key = herdr_client::NewHostKey::fixture_for_tests(
            std::path::PathBuf::from("unused"),
            Vec::new(),
        );
        let actions = step_actions(&Step::TrustKey(request, key));
        assert!(actions.cancel);
        assert_eq!(actions.primary, Some(Primary::Trust));
        assert_eq!(actions.primary.unwrap().label(), "Trust key");
    }
}
