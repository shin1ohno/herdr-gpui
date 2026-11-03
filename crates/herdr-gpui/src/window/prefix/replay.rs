//! A Settings chord waits for the same save and dismissal as the Done button.

use super::{Bound, HerdrWindow};
use crate::{menu::Page, settings_window::SettingsWindow};
use gpui::{App, Context, Entity, Subscription, WeakEntity, Window};

pub(crate) struct Replay {
    _watch: Subscription,
    bound: Bound,
    generation: u64,
    model: WeakEntity<SettingsWindow>,
}

impl Replay {
    pub(crate) fn run(
        self,
        owner: &mut HerdrWindow,
        window: &mut Window,
        cx: &mut Context<HerdrWindow>,
    ) {
        owner.run_prefix_bound(self.bound, window, cx);
    }
}

impl HerdrWindow {
    pub(super) fn defer_settings_prefix(
        &mut self,
        bound: Bound,
        model: &Entity<SettingsWindow>,
        cx: &mut Context<Self>,
    ) {
        // One close attempt owns one chord; keys arriving while its save is
        // finishing must not accumulate commands for later execution.
        if self.menu.settings_prefix.is_some() {
            return;
        }
        let generation = model.read(cx).sheet_generation();
        let watch = cx.observe(model, move |owner, model, cx| {
            if owner.menu.settings_prefix.as_ref().is_some_and(|replay| {
                replay.model.entity_id() == model.entity_id() && replay.generation == generation
            }) && !model.read(cx).sheet_dismiss_pending(generation)
            {
                // Save failures abandon closing and notify the editor. A
                // later Done/retry must not run this abandoned chord.
                owner.menu.settings_prefix = None;
            }
        });
        self.menu.settings_prefix = Some(Replay {
            _watch: watch,
            bound,
            generation,
            model: model.downgrade(),
        });
    }

    /// Take the continuation before menu reset, which discards all other paths.
    pub(crate) fn take_settings_prefix(&mut self, cx: &App) -> Option<Replay> {
        let replay = self.menu.settings_prefix.take()?;
        let model = self.menu.settings.as_ref()?;
        (self.menu.page == Some(Page::Settings)
            && replay.model.entity_id() == model.entity_id()
            && replay.generation == model.read(cx).sheet_generation()
            && model.read(cx).sheet_ready()
            && self.menu_target_current())
        .then_some(replay)
    }
}
