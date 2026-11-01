#![allow(clippy::unwrap_used)]

use super::{Offer, Page, Setup, Step, setup};
use crate::{HerdrWindow, search_input::SearchInput, sidebar::layout_tests::fixture_window};
use gpui::{
    AppContext as _, Bounds, Context, Pixels, ScrollDelta, ScrollHandle, ScrollWheelEvent,
    TestAppContext, VisualTestContext, point, px, size,
};
use herdr_client::HostKeys;

mod recheck;
mod visible_viewport;

fn show_form(view: &mut HerdrWindow, step: Step, cx: &mut Context<HerdrWindow>) {
    view.menu.device_setup = Some(Setup {
        host_key_code: Default::default(),
        fields: std::array::from_fn(|_| cx.new(SearchInput::new)),
        step,
        claim: None,
        task: None,
        discovery: None,
        suggested_label: None,
        _edits: Vec::new(),
        highlight: None,
        suggestion_scroll: ScrollHandle::new(),
        error: None,
        host_keys: HostKeys::Verify,
        #[cfg(target_os = "ios")]
        device_key: Default::default(),
    });
    view.menu.page = Some(Page::AddDevice);
    cx.notify();
}
