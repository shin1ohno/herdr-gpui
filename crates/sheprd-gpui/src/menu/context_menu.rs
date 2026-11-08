//! Context menus share a pointer action and a touch action at the same anchor.
use gpui::{prelude::*, *};
use std::rc::Rc;

pub(crate) struct ContextMenuEvent {
    pub(crate) position: Point<Pixels>,
    /// Only a real mouse press participates in terminal mouse reporting.
    pub(crate) mouse: Option<MouseDownEvent>,
}

pub(crate) fn context_menu<E: InteractiveElement + ParentElement>(
    element: E,
    listener: impl Fn(&ContextMenuEvent, &mut Window, &mut App) + 'static,
) -> E {
    let listener = Rc::new(listener);
    let mouse_listener = listener.clone();
    let element = element.on_mouse_down(MouseButton::Right, move |event, window, cx| {
        mouse_listener(
            &ContextMenuEvent {
                position: event.position,
                mouse: Some(event.clone()),
            },
            window,
            cx,
        );
    });
    #[cfg(any(target_os = "ios", test))]
    let element = element.child(
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                window.on_mouse_event(move |event: &LongPressEvent, phase, window, cx| {
                    // Tooltips claim long presses during bubble. Capture the menu
                    // first, while still respecting scroll clipping and overlays.
                    if phase != DispatchPhase::Capture
                        || event.phase != TouchPhase::Started
                        || window.default_prevented()
                        || !hitbox.is_hovered_at(event.position, window)
                    {
                        return;
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                    listener(
                        &ContextMenuEvent {
                            position: event.position,
                            mouse: None,
                        },
                        window,
                        cx,
                    );
                });
            },
        )
        .absolute()
        .inset_0(),
    );
    element
}

#[cfg(test)]
mod tests;
