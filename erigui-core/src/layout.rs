use crate::{Margins, Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub min_width: Option<i32>,
    pub max_width: Option<i32>,
    pub min_height: Option<i32>,
    pub max_height: Option<i32>,
}

impl LayoutConstraints {
    pub const UNBOUNDED: Self = Self {
        min_width: None,
        max_width: None,
        min_height: None,
        max_height: None,
    };

    pub fn bounded(width: i32, height: i32) -> Self {
        Self {
            min_width: Some(width),
            max_width: Some(width),
            min_height: Some(height),
            max_height: Some(height),
        }
    }

    pub fn with_max_width(self, width: i32) -> Self {
        Self {
            max_width: Some(width),
            ..self
        }
    }

    pub fn with_max_height(self, height: i32) -> Self {
        Self {
            max_height: Some(height),
            ..self
        }
    }

    pub fn with_min_width(self, width: i32) -> Self {
        Self {
            min_width: Some(width),
            ..self
        }
    }

    pub fn with_min_height(self, height: i32) -> Self {
        Self {
            min_height: Some(height),
            ..self
        }
    }

    pub fn constrain(&self, size: Size) -> Size {
        let width = size
            .width
            .max(self.min_width.unwrap_or(0))
            .min(self.max_width.unwrap_or(i32::MAX));

        let height = size
            .height
            .max(self.min_height.unwrap_or(0))
            .min(self.max_height.unwrap_or(i32::MAX));

        Size::new(width, height)
    }

    pub fn loosen(&self) -> Self {
        Self {
            min_width: None,
            min_height: None,
            ..*self
        }
    }

    pub fn tighten(&self, size: Size) -> Self {
        Self {
            min_width: Some(size.width),
            max_width: Some(size.width),
            min_height: Some(size.height),
            max_height: Some(size.height),
        }
    }

    pub fn deflate(&self, margins: Margins) -> Self {
        let horizontal = margins.horizontal();
        let vertical = margins.vertical();

        Self {
            min_width: self.min_width.map(|w| (w - horizontal).max(0)),
            max_width: self.max_width.map(|w| (w - horizontal).max(0)),
            min_height: self.min_height.map(|h| (h - vertical).max(0)),
            max_height: self.max_height.map(|h| (h - vertical).max(0)),
        }
    }
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self::UNBOUNDED
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    None,
    Vertical,
    Horizontal,
    Grid { columns: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossAlignment {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexChild {
    pub flex: f32,
    pub min_size: Option<i32>,
    pub max_size: Option<i32>,
}

impl FlexChild {
    pub fn new(flex: f32) -> Self {
        Self {
            flex,
            min_size: None,
            max_size: None,
        }
    }

    pub fn fixed() -> Self {
        Self {
            flex: 0.0,
            min_size: None,
            max_size: None,
        }
    }
}

impl Default for FlexChild {
    fn default() -> Self {
        Self::fixed()
    }
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub mode: LayoutMode,
    pub alignment: Alignment,
    pub cross_alignment: CrossAlignment,
    pub spacing: i32,
    pub padding: Margins,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            mode: LayoutMode::None,
            alignment: Alignment::Start,
            cross_alignment: CrossAlignment::Start,
            spacing: 0,
            padding: Margins::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- LayoutConstraints edge cases -----

    #[test]
    fn constraints_default_is_unbounded() {
        let c = LayoutConstraints::default();
        assert_eq!(c, LayoutConstraints::UNBOUNDED);
    }

    #[test]
    fn unbounded_constrain_returns_input_when_non_negative() {
        let c = LayoutConstraints::UNBOUNDED;
        let s = c.constrain(Size::new(123, 456));
        assert_eq!(s, Size::new(123, 456));
    }

    #[test]
    fn unbounded_constrain_clamps_negative_to_zero() {
        // No min set ⇒ default min is 0 in constrain(), so negatives clamp up.
        let c = LayoutConstraints::UNBOUNDED;
        let s = c.constrain(Size::new(-10, -20));
        assert_eq!(s, Size::new(0, 0));
    }

    #[test]
    fn deflate_clamps_to_zero_when_margins_exceed_size() {
        let c = LayoutConstraints::bounded(20, 30);
        let m = Margins::all(50);
        let d = c.deflate(m);
        assert_eq!(d.min_width, Some(0));
        assert_eq!(d.max_width, Some(0));
        assert_eq!(d.min_height, Some(0));
        assert_eq!(d.max_height, Some(0));
    }

    #[test]
    fn deflate_preserves_none_bounds() {
        let c = LayoutConstraints::UNBOUNDED;
        let d = c.deflate(Margins::all(10));
        assert_eq!(d.min_width, None);
        assert_eq!(d.max_width, None);
        assert_eq!(d.min_height, None);
        assert_eq!(d.max_height, None);
    }

    #[test]
    fn with_min_width_then_with_max_width_chain() {
        let c = LayoutConstraints::UNBOUNDED
            .with_min_width(10)
            .with_max_width(100);
        assert_eq!(c.min_width, Some(10));
        assert_eq!(c.max_width, Some(100));
        assert_eq!(c.min_height, None);
        assert_eq!(c.max_height, None);
    }

    #[test]
    fn with_min_height_and_max_height_chain() {
        let c = LayoutConstraints::UNBOUNDED
            .with_min_height(5)
            .with_max_height(50);
        assert_eq!(c.min_height, Some(5));
        assert_eq!(c.max_height, Some(50));
    }

    // ----- LayoutMode -----

    #[test]
    fn layout_mode_grid_carries_columns() {
        let m = LayoutMode::Grid { columns: 4 };
        match m {
            LayoutMode::Grid { columns } => assert_eq!(columns, 4),
            _ => panic!("expected Grid"),
        }
    }

    #[test]
    fn layout_mode_variants_distinct() {
        assert_ne!(LayoutMode::None, LayoutMode::Vertical);
        assert_ne!(LayoutMode::Vertical, LayoutMode::Horizontal);
        assert_ne!(
            LayoutMode::Grid { columns: 1 },
            LayoutMode::Grid { columns: 2 }
        );
    }

    // ----- Alignment / CrossAlignment -----

    #[test]
    fn alignment_variants_distinct() {
        let all = [
            Alignment::Start,
            Alignment::Center,
            Alignment::End,
            Alignment::Stretch,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn cross_alignment_stretch_distinct_from_center() {
        assert_ne!(CrossAlignment::Stretch, CrossAlignment::Center);
    }

    // ----- FlexChild -----

    #[test]
    fn flex_child_new_sets_flex_value() {
        let f = FlexChild::new(2.5);
        assert_eq!(f.flex, 2.5);
        assert!(f.min_size.is_none());
        assert!(f.max_size.is_none());
    }

    #[test]
    fn flex_child_fixed_has_zero_flex() {
        let f = FlexChild::fixed();
        assert_eq!(f.flex, 0.0);
    }

    #[test]
    fn flex_child_default_is_fixed() {
        assert_eq!(FlexChild::default(), FlexChild::fixed());
    }

    // ----- LayoutConfig -----

    #[test]
    fn layout_config_default_values() {
        let c = LayoutConfig::default();
        assert_eq!(c.mode, LayoutMode::None);
        assert_eq!(c.alignment, Alignment::Start);
        assert_eq!(c.cross_alignment, CrossAlignment::Start);
        assert_eq!(c.spacing, 0);
        assert_eq!(c.padding, Margins::ZERO);
    }
}
