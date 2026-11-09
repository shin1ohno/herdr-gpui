#![allow(clippy::unwrap_used)]

use super::{Offer, Page, Setup, Step, setup};
use crate::{SheprdWindow, search_input::SearchInput, sidebar::layout_tests::fixture_window};
use gpui::{
    AppContext as _, Bounds, Context, Pixels, ScrollDelta, ScrollEvent, ScrollHandle,
    TestAppContext, VisualTestContext, point, px, size,
};
use sheprd_client::HostKeys;

mod ipad_en_xa;
mod ipad_sheet;
mod recheck;
mod visible_viewport;

fn show_form(view: &mut SheprdWindow, step: Step, cx: &mut Context<SheprdWindow>) {
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
