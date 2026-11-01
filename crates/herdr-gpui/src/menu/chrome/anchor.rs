//! Pointer popovers must fit the usable viewport, including a software keyboard.
use gpui::{
    Anchor, AnyElement, App, Bounds, Display, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement as _, Pixels, Point, Position, Style,
    Window, anchored, point, px,
};

pub(super) fn place(
    panel: impl IntoElement,
    position: Point<Pixels>,
    anchor: Anchor,
    window: &Window,
) -> AnyElement {
    let visible = window.fully_visible_bounds();
    if visible == Bounds::new(Point::default(), window.viewport_size()) {
        return anchored()
            .position(position)
            .anchor(anchor)
            .snap_to_window_with_margin(px(12.))
            .child(panel)
            .into_any_element();
    }
    // GPUI's anchored margin is applied only after crossing the full drawable
    // edge. Measure the child to fit it even when only the keyboard covers it.
    let margin = px(12.) + window.client_inset().unwrap_or_default();
    ViewportAnchor {
        anchor,
        child: panel.into_any_element(),
        limits: visible.inset(margin),
        position,
    }
    .into_any_element()
}

struct ViewportAnchor {
    anchor: Anchor,
    child: AnyElement,
    limits: Bounds<Pixels>,
    position: Point<Pixels>,
}

impl IntoElement for ViewportAnchor {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for ViewportAnchor {
    type RequestLayoutState = LayoutId;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, LayoutId) {
        let child = self.child.request_layout(window, cx);
        let style = Style {
            position: Position::Absolute,
            display: Display::Flex,
            ..Default::default()
        };
        (window.request_layout(style, [child], cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child_layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = window.layout_bounds(*child_layout);
        let desired = Bounds::from_anchor_and_size(self.anchor, self.position, child.size);
        let origin = point(
            desired
                .left()
                .min(self.limits.right() - child.size.width)
                .max(self.limits.left()),
            desired
                .top()
                .min(self.limits.bottom() - child.size.height)
                .max(self.limits.top()),
        );
        let offset = origin - bounds.origin;
        window.with_element_offset(point(offset.x.round(), offset.y.round()), |window| {
            self.child.prepaint(window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
