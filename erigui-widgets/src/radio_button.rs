use erigui_core::{
    DrawContext, Event, EventResult, LayoutConstraints, MouseButton, MouseButtonEvent, Point, Rect,
    Size, Theme, Widget, WidgetId, WidgetState,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPosition {
    Left,
    Right,
    Top,
    Bottom,
}

/// Host-owned mutual-exclusion state shared between `RadioButton`s.
///
/// Replaces the Mojo-port `RadioGroupManager` global singleton. The
/// host instantiates one `RadioGroup` per logical group, wraps it in
/// `Rc<RefCell<_>>`, and hands a clone to each `RadioButton` via
/// `with_group`. Selection is O(1) — just a `WidgetId` compare — and
/// there is no global state, no Mutex, and no per-frame polling.
#[derive(Debug, Default, Clone)]
pub struct RadioGroup {
    selected: Option<WidgetId>,
}

impl RadioGroup {
    pub fn new() -> Self {
        Self { selected: None }
    }

    /// Currently selected button id, or `None` if nothing is selected.
    pub fn selected(&self) -> Option<WidgetId> {
        self.selected
    }

    /// Mark `id` as the group's selected button. Any previously
    /// selected button in this group is implicitly deselected — every
    /// `RadioButton` reads `is_selected(self.id())` on the next event
    /// or draw and updates its own visual state from that.
    pub fn select(&mut self, id: WidgetId) {
        self.selected = Some(id);
    }

    /// Returns true iff `id` is the currently selected button.
    pub fn is_selected(&self, id: WidgetId) -> bool {
        self.selected == Some(id)
    }

    /// Drop the selection entirely. Useful for tests and for hosts
    /// that want a "no choice" reset.
    pub fn clear(&mut self) {
        self.selected = None;
    }
}

pub struct RadioButton {
    state: WidgetState,
    text: String,
    group: Option<Rc<RefCell<RadioGroup>>>,
    /// Set true by `with_checked(true)` before a group is attached
    /// (or as a standalone toggle when no group is ever attached).
    /// When a group *is* attached, `checked()` is the group's view —
    /// this field is only consulted as a fallback for the groupless
    /// case and to seed the initial selection from `with_group`.
    initially_checked: bool,
    button_size: i32,
    label_position: LabelPosition,
    hover: bool,
    pressed: bool,
    animation_progress: f32,
    on_change: Option<Box<dyn FnMut(bool)>>,
}

impl RadioButton {
    pub fn new(id: WidgetId, text: impl Into<String>) -> Self {
        Self {
            state: WidgetState::new(id),
            text: text.into(),
            group: None,
            initially_checked: false,
            button_size: 16,
            label_position: LabelPosition::Right,
            hover: false,
            pressed: false,
            animation_progress: 0.0,
            on_change: None,
        }
    }

    /// Attach this radio button to a host-owned `RadioGroup`. All
    /// radios sharing the same `Rc<RefCell<RadioGroup>>` mutually
    /// exclude. If `with_checked(true)` was called before this, the
    /// initial selection seeds the group (last attached wins, matching
    /// the old singleton's last-write-wins behaviour).
    pub fn with_group(mut self, group: Rc<RefCell<RadioGroup>>) -> Self {
        if self.initially_checked {
            group.borrow_mut().select(self.state.id);
        }
        self.group = Some(group);
        self
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.initially_checked = checked;
        if checked {
            // If a group was attached first, propagate immediately.
            // Otherwise `with_group` will pick up `initially_checked`.
            if let Some(group) = &self.group {
                group.borrow_mut().select(self.state.id);
            }
            self.animation_progress = 1.0;
        } else {
            // Only clear group state if *we* are the currently
            // selected button — don't trample another radio's pick.
            if let Some(group) = &self.group {
                let mut g = group.borrow_mut();
                if g.is_selected(self.state.id) {
                    g.clear();
                }
            }
            self.animation_progress = 0.0;
        }
        self
    }

    pub fn with_label_position(mut self, position: LabelPosition) -> Self {
        self.label_position = position;
        self
    }

    pub fn with_on_change<F: FnMut(bool) + 'static>(mut self, f: F) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Whether this button is currently the selected one in its group.
    /// For groupless radios this falls back to `initially_checked`
    /// (which `set_checked` keeps in sync).
    pub fn is_checked(&self) -> bool {
        match &self.group {
            Some(g) => g.borrow().is_selected(self.state.id),
            None => self.initially_checked,
        }
    }

    /// Programmatic selection. With a group attached this writes the
    /// new selection into the shared `RadioGroup`; without one it
    /// flips the local fallback. Either way, the `on_change` callback
    /// fires only when the effective state actually changed.
    pub fn set_checked(&mut self, checked: bool) {
        let was_checked = self.is_checked();
        match &self.group {
            Some(group) => {
                let mut g = group.borrow_mut();
                if checked {
                    g.select(self.state.id);
                } else if g.is_selected(self.state.id) {
                    g.clear();
                }
            }
            None => {
                self.initially_checked = checked;
            }
        }
        // Snap animation immediately for THIS radio so the inner dot
        // appears on click without requiring Event::Update pumping.
        // Other radios in the group will catch up via sync_animation
        // (called from handle_event on every event) or via Event::Update.
        self.animation_progress = if checked { 1.0 } else { 0.0 };
        if was_checked != checked {
            if let Some(callback) = &mut self.on_change {
                callback(checked);
            }
        }
    }

    /// Pull the visual state in line with the group's selection. Cheap
    /// (one borrow + one compare). Called from handle_event on every
    /// event so a click on radio A also animates radio B's deselection
    /// without requiring the host to pump Event::Update.
    fn sync_animation_from_group(&mut self) {
        let target: f32 = if self.is_checked() { 1.0 } else { 0.0 };
        if (self.animation_progress - target).abs() > 0.001 {
            self.animation_progress = target;
        }
    }

    fn update_animation(&mut self, delta_time: f32) {
        let target = if self.is_checked() { 1.0 } else { 0.0 };
        let speed = 5.0;

        if self.animation_progress < target {
            self.animation_progress = (self.animation_progress + speed * delta_time).min(target);
        } else if self.animation_progress > target {
            self.animation_progress = (self.animation_progress - speed * delta_time).max(target);
        }
    }
}

impl Widget for RadioButton {
    fn id(&self) -> WidgetId {
        self.state.id
    }

    fn measure(&self, _constraints: &LayoutConstraints, theme: &Theme) -> Size {
        let text_size = if !self.text.is_empty() {
            // Approximate text size
            Size::new(
                self.text.len() as i32 * theme.typography.font_size_base * 3 / 5,
                theme.typography.font_size_base,
            )
        } else {
            Size::ZERO
        };

        let spacing = theme.spacing.gap_small;

        match self.label_position {
            LabelPosition::Left | LabelPosition::Right => Size::new(
                self.button_size + spacing + text_size.width,
                self.button_size.max(text_size.height),
            ),
            LabelPosition::Top | LabelPosition::Bottom => Size::new(
                self.button_size.max(text_size.width),
                self.button_size + spacing + text_size.height,
            ),
        }
    }

    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }

    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        if !self.state.visible {
            return;
        }

        let spacing = theme.spacing.gap_small;

        // Calculate button position based on label position
        let (button_x, button_y) = match self.label_position {
            LabelPosition::Left => (
                self.state.bounds.right() - self.button_size,
                self.state.bounds.y() + (self.state.bounds.height() - self.button_size) / 2,
            ),
            LabelPosition::Right => (
                self.state.bounds.x(),
                self.state.bounds.y() + (self.state.bounds.height() - self.button_size) / 2,
            ),
            LabelPosition::Top => (
                self.state.bounds.x() + (self.state.bounds.width() - self.button_size) / 2,
                self.state.bounds.bottom() - self.button_size,
            ),
            LabelPosition::Bottom => (
                self.state.bounds.x() + (self.state.bounds.width() - self.button_size) / 2,
                self.state.bounds.y(),
            ),
        };

        let button_rect = Rect::new(button_x, button_y, self.button_size, self.button_size);
        let center = button_rect.center();
        let outer_radius = self.button_size / 2;
        let inner_radius = (outer_radius as f32 * 0.4 * self.animation_progress) as i32;

        // Draw outer circle
        let outer_color = if !self.state.enabled {
            theme.colors.text_disabled
        } else if self.pressed {
            theme.colors.primary_active
        } else if self.hover {
            theme.colors.primary_hover
        } else {
            theme.colors.border
        };

        context.set_color(outer_color);
        context.draw_circle(center, outer_radius, 16);

        // Draw inner filled circle when checked
        if self.animation_progress > 0.0 {
            let inner_color = if !self.state.enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.primary
            };

            context.set_color(inner_color);
            context.fill_circle(center, inner_radius, 12);
        }

        // Draw label
        if !self.text.is_empty() {
            let text_color = if !self.state.enabled {
                theme.colors.text_disabled
            } else {
                theme.colors.text
            };

            context.set_color(text_color);

            let (text_x, text_y) = match self.label_position {
                LabelPosition::Left => (
                    self.state.bounds.x(),
                    button_rect.center().y - theme.typography.font_size_base / 2,
                ),
                LabelPosition::Right => (
                    button_rect.right() + spacing,
                    button_rect.center().y - theme.typography.font_size_base / 2,
                ),
                LabelPosition::Top => (
                    self.state.bounds.x()
                        + (self.state.bounds.width()
                            - self.text.len() as i32 * theme.typography.font_size_base * 3 / 5)
                            / 2,
                    self.state.bounds.y() + theme.typography.font_size_base,
                ),
                LabelPosition::Bottom => (
                    self.state.bounds.x()
                        + (self.state.bounds.width()
                            - self.text.len() as i32 * theme.typography.font_size_base * 3 / 5)
                            / 2,
                    button_rect.bottom() + spacing + theme.typography.font_size_base,
                ),
            };

            context.draw_text(
                &self.text,
                Point::new(text_x, text_y),
                theme.typography.font_size_base,
            );
        }
    }

    fn handle_event(&mut self, event: &Event, _theme: &Theme) -> EventResult {
        if !self.state.enabled || !self.state.visible {
            return EventResult::Ignored;
        }

        // Sync animation state to the group's view of selection on every
        // event. When radio A's click selects A, B/C/D see the same event
        // (or any subsequent event) and update their own visual state
        // accordingly without needing Event::Update to be explicitly
        // pumped by the host.
        self.sync_animation_from_group();

        match event {
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position,
                pressed,
                ..
            }) => {
                if self.state.bounds.contains(*position) {
                    if *pressed {
                        self.pressed = true;
                    } else if self.pressed {
                        self.pressed = false;
                        if !self.is_checked() {
                            // Click selects this radio. The group
                            // (if any) propagates the deselection of
                            // the previously-selected button — every
                            // other RadioButton sharing this group
                            // will read `is_selected(self.id())` as
                            // false on its next event/draw and
                            // animate off.
                            self.set_checked(true);
                        }
                    }
                    return EventResult::Consumed;
                } else {
                    self.pressed = false;
                }
            }
            Event::MouseMove(move_event) => {
                let was_hover = self.hover;
                self.hover = self.state.bounds.contains(move_event.position);
                if was_hover != self.hover {
                    return EventResult::Consumed;
                }
            }
            Event::Update => {
                // Animation only — no group polling. Selection state
                // is owned by the host-owned RadioGroup and read via
                // `is_checked()` directly.
                self.update_animation(0.016); // Assume 60 FPS
                if self.animation_progress > 0.0 && self.animation_progress < 1.0 {
                    return EventResult::Consumed;
                }
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn bounds(&self) -> Rect {
        self.state.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.state.bounds = bounds;
    }

    fn is_visible(&self) -> bool {
        self.state.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.visible = visible;
    }

    fn is_enabled(&self) -> bool {
        self.state.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
    }

    fn is_focused(&self) -> bool {
        self.state.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.state.focused = focused;
    }

    fn can_focus(&self) -> bool {
        self.state.enabled && self.state.visible
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erigui_core::{Modifiers, MouseButton, MouseButtonEvent, MouseMoveEvent, Theme};

    fn click_at(p: Point) -> [Event; 2] {
        [
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position: p,
                pressed: true,
                modifiers: Modifiers::empty(),
            }),
            Event::MouseButton(MouseButtonEvent {
                button: MouseButton::Left,
                position: p,
                pressed: false,
                modifiers: Modifiers::empty(),
            }),
        ]
    }

    fn move_to(p: Point) -> Event {
        Event::MouseMove(MouseMoveEvent {
            position: p,
            delta: Point::ZERO,
            modifiers: Modifiers::empty(),
        })
    }

    fn dummy_id() -> WidgetId {
        // Single-radio tests don't care about identity — use the default
        // key. Multi-radio tests in a shared group must use `id_pool()`
        // because the group keys on `WidgetId` and two
        // `WidgetId::default()` would collide.
        WidgetId::default()
    }

    /// Allocate distinct `WidgetId`s for tests that put multiple radios
    /// in the same group. Returns a closure; each call yields a fresh
    /// key from a private SlotMap whose values we don't care about.
    fn id_pool() -> impl FnMut() -> WidgetId {
        let mut sm: slotmap::SlotMap<WidgetId, ()> = slotmap::SlotMap::new();
        move || sm.insert(())
    }

    #[test]
    fn radio_group_starts_unselected() {
        let g = RadioGroup::new();
        assert_eq!(g.selected(), None);
        assert!(!g.is_selected(dummy_id()));
    }

    #[test]
    fn radio_group_select_and_clear() {
        let mut g = RadioGroup::new();
        let id = dummy_id();
        g.select(id);
        assert_eq!(g.selected(), Some(id));
        assert!(g.is_selected(id));
        g.clear();
        assert_eq!(g.selected(), None);
    }

    #[test]
    fn radio_button_new_does_not_touch_global() {
        // The whole point of the rewrite: constructors are inert.
        // If `new` mutated any global, two independent groups would
        // see each other. We construct buttons with no group at all
        // and confirm their default state is "unchecked, no group."
        let r1 = RadioButton::new(dummy_id(), "a");
        let r2 = RadioButton::new(dummy_id(), "b");
        assert!(!r1.is_checked());
        assert!(!r2.is_checked());
        assert!(r1.group.is_none());
        assert!(r2.group.is_none());
    }

    #[test]
    fn two_radios_in_same_group_mutually_exclude() {
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let theme = Theme::dark();
        let bounds_a = Rect::new(0, 0, 50, 20);
        let bounds_b = Rect::new(0, 30, 50, 20);
        let mut next_id = id_pool();

        let mut a = RadioButton::new(next_id(), "A").with_group(group.clone());
        let mut b = RadioButton::new(next_id(), "B").with_group(group.clone());
        a.layout(bounds_a, &theme);
        b.layout(bounds_b, &theme);

        // Click A — only A becomes selected.
        for ev in &click_at(Point::new(10, 10)) {
            a.handle_event(ev, &theme);
        }
        assert!(a.is_checked());
        assert!(!b.is_checked());
        assert_eq!(group.borrow().selected(), Some(a.id()));

        // Click B — A flips off via the shared group, B becomes
        // selected. A reads its new state on the next is_checked()
        // call without anyone touching it directly.
        for ev in &click_at(Point::new(10, 40)) {
            b.handle_event(ev, &theme);
        }
        assert!(!a.is_checked());
        assert!(b.is_checked());
        assert_eq!(group.borrow().selected(), Some(b.id()));
    }

    #[test]
    fn two_groups_do_not_cross_talk() {
        let theme = Theme::dark();
        let group_x = Rc::new(RefCell::new(RadioGroup::new()));
        let group_y = Rc::new(RefCell::new(RadioGroup::new()));
        let mut next_id = id_pool();

        let mut x1 = RadioButton::new(next_id(), "X1").with_group(group_x.clone());
        let mut x2 = RadioButton::new(next_id(), "X2").with_group(group_x.clone());
        let mut y1 = RadioButton::new(next_id(), "Y1").with_group(group_y.clone());
        let mut y2 = RadioButton::new(next_id(), "Y2").with_group(group_y.clone());

        x1.layout(Rect::new(0, 0, 50, 20), &theme);
        x2.layout(Rect::new(0, 30, 50, 20), &theme);
        y1.layout(Rect::new(100, 0, 50, 20), &theme);
        y2.layout(Rect::new(100, 30, 50, 20), &theme);

        // Pick x2 in group X.
        for ev in &click_at(Point::new(10, 40)) {
            x2.handle_event(ev, &theme);
        }
        // Pick y1 in group Y.
        for ev in &click_at(Point::new(110, 10)) {
            y1.handle_event(ev, &theme);
        }

        // Each group remembers its own choice independently.
        assert!(x2.is_checked());
        assert!(!x1.is_checked());
        assert!(y1.is_checked());
        assert!(!y2.is_checked());
        assert_eq!(group_x.borrow().selected(), Some(x2.id()));
        assert_eq!(group_y.borrow().selected(), Some(y1.id()));
    }

    #[test]
    fn with_checked_seeds_group_selection() {
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let mut next_id = id_pool();
        let a = RadioButton::new(next_id(), "A")
            .with_group(group.clone())
            .with_checked(true);
        let b = RadioButton::new(next_id(), "B").with_group(group.clone());

        assert!(a.is_checked());
        assert!(!b.is_checked());
        assert_eq!(group.borrow().selected(), Some(a.id()));
    }

    #[test]
    fn with_checked_before_with_group_still_seeds() {
        // Builder ordering shouldn't matter: with_checked then
        // with_group is the same outcome as the reverse.
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let a = RadioButton::new(dummy_id(), "A")
            .with_checked(true)
            .with_group(group.clone());
        assert!(a.is_checked());
        assert_eq!(group.borrow().selected(), Some(a.id()));
    }

    #[test]
    fn set_checked_propagates_to_group_and_fires_callback() {
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let fired = Rc::new(RefCell::new(0));
        let f2 = fired.clone();
        let mut a = RadioButton::new(dummy_id(), "A")
            .with_group(group.clone())
            .with_on_change(move |checked| {
                if checked {
                    *f2.borrow_mut() += 1;
                }
            });

        a.set_checked(true);
        assert!(a.is_checked());
        assert_eq!(group.borrow().selected(), Some(a.id()));
        assert_eq!(*fired.borrow(), 1);

        // No-op when state is already true — callback shouldn't fire
        // again or the host gets duplicate signals on every redraw.
        a.set_checked(true);
        assert_eq!(*fired.borrow(), 1);
    }

    #[test]
    fn groupless_radio_acts_as_local_toggle() {
        // No group attached: with_checked / set_checked / is_checked
        // operate on the local fallback. Useful for a single-radio
        // case (degenerate but should not panic or touch globals).
        let mut a = RadioButton::new(dummy_id(), "alone").with_checked(true);
        assert!(a.is_checked());
        a.set_checked(false);
        assert!(!a.is_checked());
    }

    #[test]
    fn click_outside_bounds_does_not_select() {
        let theme = Theme::dark();
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let mut a = RadioButton::new(dummy_id(), "A").with_group(group.clone());
        a.layout(Rect::new(0, 0, 50, 20), &theme);

        // Click far away from the button — no selection should happen.
        for ev in &click_at(Point::new(500, 500)) {
            a.handle_event(ev, &theme);
        }
        assert!(!a.is_checked());
        assert_eq!(group.borrow().selected(), None);
    }

    #[test]
    fn hover_state_updates_on_mouse_move() {
        let theme = Theme::dark();
        let mut a = RadioButton::new(dummy_id(), "A");
        a.layout(Rect::new(0, 0, 50, 20), &theme);

        // Move into bounds: hover flips true.
        a.handle_event(&move_to(Point::new(10, 10)), &theme);
        assert!(a.hover);

        // Move out: hover flips false.
        a.handle_event(&move_to(Point::new(500, 500)), &theme);
        assert!(!a.hover);
    }

    #[test]
    fn disabled_radio_ignores_clicks() {
        let theme = Theme::dark();
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let mut a = RadioButton::new(dummy_id(), "A").with_group(group.clone());
        a.layout(Rect::new(0, 0, 50, 20), &theme);
        a.set_enabled(false);

        for ev in &click_at(Point::new(10, 10)) {
            a.handle_event(ev, &theme);
        }
        assert!(!a.is_checked());
        assert_eq!(group.borrow().selected(), None);
    }

    #[test]
    fn update_event_no_longer_polls_group() {
        // Regression guard for the per-frame Mutex poll that used to
        // live in Event::Update. With the singleton gone, Update is
        // pure animation: it must not change `is_checked` in either
        // direction by itself. (The animation_progress will track
        // toward the group's view, but the *checked* state derives
        // directly from the group, not from the update tick.)
        let theme = Theme::dark();
        let group = Rc::new(RefCell::new(RadioGroup::new()));
        let mut a = RadioButton::new(dummy_id(), "A").with_group(group.clone());

        for _ in 0..10 {
            a.handle_event(&Event::Update, &theme);
        }
        assert!(!a.is_checked());

        // Drive selection externally via the group, then tick Update
        // again — is_checked tracks the group directly.
        group.borrow_mut().select(a.id());
        a.handle_event(&Event::Update, &theme);
        assert!(a.is_checked());
    }
}
