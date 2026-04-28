use crate::{Color, Margins};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    pub typography: Typography,
    pub spacing: Spacing,
    pub borders: Borders,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Background colors
    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,

    // Text colors
    pub text: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,

    // Primary colors
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_active: Color,
    pub primary_disabled: Color,

    // Secondary colors
    pub secondary: Color,
    pub secondary_hover: Color,
    pub secondary_active: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Border colors
    pub border: Color,
    pub border_hover: Color,
    pub border_focus: Color,

    // Special colors
    pub selection: Color,
    pub shadow: Color,
    pub overlay: Color,
}

#[derive(Debug, Clone)]
pub struct Typography {
    pub font_family: String,
    pub font_size_base: i32,
    pub font_size_small: i32,
    pub font_size_large: i32,
    pub font_size_xlarge: i32,
    pub line_height: f32,
    pub letter_spacing: f32,
}

#[derive(Debug, Clone)]
pub struct Spacing {
    pub base: i32,
    pub margins: Margins,
    pub padding: Margins,
    pub gap_small: i32,
    pub gap_medium: i32,
    pub gap_large: i32,
}

#[derive(Debug, Clone)]
pub struct Borders {
    pub width: i32,
    pub radius: i32,
    pub radius_small: i32,
    pub radius_large: i32,
}

impl Theme {
    pub fn alex_jammin() -> Self {
        Self {
            name: "alexJammin".to_string(),
            colors: ThemeColors {
                background: Color::from_hex(0x0D0F1B),
                surface: Color::from_hex(0x15192B),
                surface_variant: Color::from_hex(0x1E2237),
                text: Color::from_hex(0xE8EBFF),
                text_secondary: Color::from_hex(0xA5AECD),
                text_disabled: Color::from_hex(0x5C6076),
                primary: Color::from_hex(0x7C5DFA),
                primary_hover: Color::from_hex(0x8E72FF),
                primary_active: Color::from_hex(0x5A3BD9),
                primary_disabled: Color::from_hex(0x2C234B),
                secondary: Color::from_hex(0x5F6B99),
                secondary_hover: Color::from_hex(0x7080B5),
                secondary_active: Color::from_hex(0x4A547C),
                success: Color::from_hex(0x7CE8C4),
                warning: Color::from_hex(0xF2C94C),
                error: Color::from_hex(0xEF6B73),
                info: Color::from_hex(0x5FD4FF),
                border: Color::from_hex(0x24293D),
                border_hover: Color::from_hex(0x343B55),
                border_focus: Color::from_hex(0x7C5DFA),
                selection: Color::rgba(124, 93, 250, 60),
                shadow: Color::rgba(0, 0, 0, 140),
                overlay: Color::rgba(3, 4, 10, 180),
            },
            typography: Typography {
                font_family: "Space Grotesk".to_string(),
                font_size_base: 14,
                font_size_small: 12,
                font_size_large: 16,
                font_size_xlarge: 22,
                line_height: 1.4,
                letter_spacing: 0.0,
            },
            spacing: Spacing {
                base: 8,
                margins: Margins::all(10),
                padding: Margins::all(10),
                gap_small: 6,
                gap_medium: 10,
                gap_large: 18,
            },
            borders: Borders {
                width: 1,
                radius: 8,
                radius_small: 6,
                radius_large: 12,
            },
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            colors: ThemeColors {
                // Backgrounds
                background: Color::from_hex(0xFFFFFF),
                surface: Color::from_hex(0xF5F5F5),
                surface_variant: Color::from_hex(0xEEEEEE),

                // Text
                text: Color::from_hex(0x212121),
                text_secondary: Color::from_hex(0x757575),
                text_disabled: Color::from_hex(0xBDBDBD),

                // Primary
                primary: Color::from_hex(0x1976D2),
                primary_hover: Color::from_hex(0x1565C0),
                primary_active: Color::from_hex(0x0D47A1),
                primary_disabled: Color::from_hex(0x90CAF9),

                // Secondary
                secondary: Color::from_hex(0x424242),
                secondary_hover: Color::from_hex(0x333333),
                secondary_active: Color::from_hex(0x212121),

                // Semantic
                success: Color::from_hex(0x4CAF50),
                warning: Color::from_hex(0xFF9800),
                error: Color::from_hex(0xF44336),
                info: Color::from_hex(0x2196F3),

                // Borders
                border: Color::from_hex(0xE0E0E0),
                border_hover: Color::from_hex(0xBDBDBD),
                border_focus: Color::from_hex(0x1976D2),

                // Special
                selection: Color::rgba(25, 118, 210, 51), // 20% alpha
                shadow: Color::rgba(0, 0, 0, 38),         // 15% alpha
                overlay: Color::rgba(0, 0, 0, 128),       // 50% alpha
            },
            typography: Typography {
                font_family: "JetBrains Mono".to_string(),
                font_size_base: 14,
                font_size_small: 12,
                font_size_large: 16,
                font_size_xlarge: 20,
                line_height: 1.5,
                letter_spacing: 0.0,
            },
            spacing: Spacing {
                base: 8,
                margins: Margins::all(8),
                padding: Margins::all(8),
                gap_small: 4,
                gap_medium: 8,
                gap_large: 16,
            },
            borders: Borders {
                width: 1,
                radius: 4,
                radius_small: 2,
                radius_large: 8,
            },
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            colors: ThemeColors {
                // Backgrounds
                background: Color::from_hex(0x121212),
                surface: Color::from_hex(0x1E1E1E),
                surface_variant: Color::from_hex(0x2C2C2C),

                // Text
                text: Color::from_hex(0xE0E0E0),
                text_secondary: Color::from_hex(0x9E9E9E),
                text_disabled: Color::from_hex(0x616161),

                // Primary
                primary: Color::from_hex(0x90CAF9),
                primary_hover: Color::from_hex(0x64B5F6),
                primary_active: Color::from_hex(0x42A5F5),
                primary_disabled: Color::from_hex(0x424242),

                // Secondary
                secondary: Color::from_hex(0xBDBDBD),
                secondary_hover: Color::from_hex(0xE0E0E0),
                secondary_active: Color::from_hex(0xF5F5F5),

                // Semantic
                success: Color::from_hex(0x66BB6A),
                warning: Color::from_hex(0xFFA726),
                error: Color::from_hex(0xEF5350),
                info: Color::from_hex(0x42A5F5),

                // Borders
                border: Color::from_hex(0x424242),
                border_hover: Color::from_hex(0x616161),
                border_focus: Color::from_hex(0x90CAF9),

                // Special
                selection: Color::rgba(144, 202, 249, 51), // 20% alpha
                shadow: Color::rgba(0, 0, 0, 128),         // 50% alpha
                overlay: Color::rgba(0, 0, 0, 179),        // 70% alpha
            },
            ..Self::light()
        }
    }

    pub fn from_system() -> Self {
        // TODO: Detect system theme
        Self::light()
    }

    /// Scale all pixel-valued fields by `scale` (typography font sizes,
    /// spacing, borders). For HiDPI: pass the monitor's `scale_factor`.
    /// Leaves color/font_family/line_height/letter_spacing untouched.
    /// `scale <= 0.0` is treated as `1.0`.
    pub fn with_scale(mut self, scale: f32) -> Self {
        let s = if scale > 0.0 { scale } else { 1.0 };
        if (s - 1.0).abs() < f32::EPSILON {
            return self;
        }
        let scale_i = |v: i32| -> i32 { ((v as f32) * s).round() as i32 };
        let scale_m = |m: Margins| -> Margins {
            Margins::new(scale_i(m.top), scale_i(m.right), scale_i(m.bottom), scale_i(m.left))
        };

        self.typography.font_size_base = scale_i(self.typography.font_size_base);
        self.typography.font_size_small = scale_i(self.typography.font_size_small);
        self.typography.font_size_large = scale_i(self.typography.font_size_large);
        self.typography.font_size_xlarge = scale_i(self.typography.font_size_xlarge);

        self.spacing.base = scale_i(self.spacing.base);
        self.spacing.gap_small = scale_i(self.spacing.gap_small);
        self.spacing.gap_medium = scale_i(self.spacing.gap_medium);
        self.spacing.gap_large = scale_i(self.spacing.gap_large);
        self.spacing.margins = scale_m(self.spacing.margins);
        self.spacing.padding = scale_m(self.spacing.padding);

        self.borders.width = scale_i(self.borders.width).max(1);
        self.borders.radius = scale_i(self.borders.radius);
        self.borders.radius_small = scale_i(self.borders.radius_small);
        self.borders.radius_large = scale_i(self.borders.radius_large);

        self
    }
}

#[derive(Debug, Clone)]
pub struct ThemeManager {
    themes: HashMap<String, Theme>,
    current_theme: String,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        themes.insert("Light".to_string(), Theme::light());
        themes.insert("Dark".to_string(), Theme::dark());
        themes.insert("alexJammin".to_string(), Theme::alex_jammin());

        Self {
            themes,
            current_theme: "Light".to_string(),
        }
    }

    pub fn current(&self) -> &Theme {
        self.themes
            .get(&self.current_theme)
            .expect("Current theme should always exist")
    }

    pub fn set_theme(&mut self, name: &str) -> Result<(), String> {
        if self.themes.contains_key(name) {
            self.current_theme = name.to_string();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", name))
        }
    }

    pub fn add_theme(&mut self, theme: Theme) {
        self.themes.insert(theme.name.clone(), theme);
    }

    pub fn list_themes(&self) -> Vec<&str> {
        self.themes.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Built-in theme construction -----

    #[test]
    fn light_theme_name() {
        let t = Theme::light();
        assert_eq!(t.name, "Light");
    }

    #[test]
    fn dark_theme_name() {
        let t = Theme::dark();
        assert_eq!(t.name, "Dark");
    }

    #[test]
    fn alex_jammin_theme_name() {
        let t = Theme::alex_jammin();
        assert_eq!(t.name, "alexJammin");
    }

    #[test]
    fn from_system_does_not_panic() {
        let _ = Theme::from_system();
    }

    #[test]
    fn light_theme_background_is_white() {
        let t = Theme::light();
        assert_eq!(t.colors.background, Color::from_hex(0xFFFFFF));
    }

    #[test]
    fn dark_theme_background_is_dark() {
        let t = Theme::dark();
        assert_eq!(t.colors.background, Color::from_hex(0x121212));
    }

    #[test]
    fn dark_theme_inherits_typography_from_light() {
        // Dark::dark() uses ..Self::light() so non-color fields should match.
        let l = Theme::light();
        let d = Theme::dark();
        assert_eq!(d.typography.font_family, l.typography.font_family);
        assert_eq!(d.typography.font_size_base, l.typography.font_size_base);
        assert_eq!(d.spacing.base, l.spacing.base);
        assert_eq!(d.borders.radius, l.borders.radius);
    }

    #[test]
    fn light_theme_typography_is_jetbrains_mono() {
        let t = Theme::light();
        assert_eq!(t.typography.font_family, "JetBrains Mono");
    }

    #[test]
    fn alex_jammin_typography_is_space_grotesk() {
        let t = Theme::alex_jammin();
        assert_eq!(t.typography.font_family, "Space Grotesk");
    }

    #[test]
    fn light_theme_borders_are_sensible() {
        let t = Theme::light();
        assert!(t.borders.width > 0);
        assert!(t.borders.radius_small <= t.borders.radius);
        assert!(t.borders.radius <= t.borders.radius_large);
    }

    // ----- ThemeManager -----

    #[test]
    fn manager_default_starts_with_three_themes() {
        let m = ThemeManager::default();
        let names = m.list_themes();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"Light"));
        assert!(names.contains(&"Dark"));
        assert!(names.contains(&"alexJammin"));
    }

    #[test]
    fn manager_default_current_is_light() {
        let m = ThemeManager::new();
        assert_eq!(m.current().name, "Light");
    }

    #[test]
    fn manager_set_theme_to_existing_succeeds() {
        let mut m = ThemeManager::new();
        assert!(m.set_theme("Dark").is_ok());
        assert_eq!(m.current().name, "Dark");
    }

    #[test]
    fn manager_set_theme_to_missing_returns_err() {
        let mut m = ThemeManager::new();
        let err = m.set_theme("Solarized").unwrap_err();
        assert!(err.contains("Solarized"));
        // current theme should be unchanged.
        assert_eq!(m.current().name, "Light");
    }

    #[test]
    fn manager_add_theme_makes_it_available() {
        let mut m = ThemeManager::new();
        let mut custom = Theme::light();
        custom.name = "Custom".to_string();
        m.add_theme(custom);
        assert!(m.list_themes().contains(&"Custom"));
        assert!(m.set_theme("Custom").is_ok());
        assert_eq!(m.current().name, "Custom");
    }

    #[test]
    fn manager_add_theme_overwrites_same_name() {
        let mut m = ThemeManager::new();
        let mut replacement = Theme::light();
        replacement.name = "Light".to_string();
        // Sentinel: change a typography value to detect overwrite.
        replacement.typography.font_size_base = 999;
        m.add_theme(replacement);
        assert_eq!(m.current().typography.font_size_base, 999);
    }

    #[test]
    fn manager_list_themes_returns_all_keys() {
        let mut m = ThemeManager::new();
        let mut t = Theme::dark();
        t.name = "Extra".to_string();
        m.add_theme(t);
        assert_eq!(m.list_themes().len(), 4);
    }

    // ----- with_scale (HiDPI) -----

    #[test]
    fn with_scale_one_is_identity() {
        let t = Theme::dark();
        let s = Theme::dark().with_scale(1.0);
        assert_eq!(s.typography.font_size_base, t.typography.font_size_base);
        assert_eq!(s.spacing.base, t.spacing.base);
        assert_eq!(s.borders.width, t.borders.width);
        assert_eq!(s.borders.radius, t.borders.radius);
    }

    #[test]
    fn with_scale_two_doubles_typography() {
        let s = Theme::dark().with_scale(2.0);
        assert_eq!(s.typography.font_size_base, 28); // 14 * 2
        assert_eq!(s.typography.font_size_small, 24); // 12 * 2
        assert_eq!(s.typography.font_size_large, 32); // 16 * 2
        assert_eq!(s.typography.font_size_xlarge, 40); // 20 * 2 (light/dark have 20)
    }

    #[test]
    fn with_scale_two_doubles_spacing() {
        let s = Theme::dark().with_scale(2.0);
        assert_eq!(s.spacing.base, 16); // 8 * 2
        assert_eq!(s.spacing.gap_small, 8); // 4 * 2
        assert_eq!(s.spacing.gap_medium, 16); // 8 * 2
        assert_eq!(s.spacing.gap_large, 32); // 16 * 2
        assert_eq!(s.spacing.padding.top, 16); // 8 * 2
        assert_eq!(s.spacing.margins.left, 16); // 8 * 2
    }

    #[test]
    fn with_scale_two_doubles_borders() {
        let s = Theme::dark().with_scale(2.0);
        assert_eq!(s.borders.width, 2); // 1 * 2
        assert_eq!(s.borders.radius_small, 4); // 2 * 2
        assert_eq!(s.borders.radius, 8); // 4 * 2
        assert_eq!(s.borders.radius_large, 16); // 8 * 2
    }

    #[test]
    fn with_scale_preserves_colors_and_family() {
        let t = Theme::dark();
        let s = Theme::dark().with_scale(2.0);
        assert_eq!(s.colors.background, t.colors.background);
        assert_eq!(s.colors.text, t.colors.text);
        assert_eq!(s.typography.font_family, t.typography.font_family);
        assert_eq!(s.typography.line_height, t.typography.line_height);
        assert_eq!(s.typography.letter_spacing, t.typography.letter_spacing);
    }

    #[test]
    fn with_scale_zero_or_negative_is_treated_as_one() {
        let t = Theme::dark();
        let s_zero = Theme::dark().with_scale(0.0);
        let s_neg = Theme::dark().with_scale(-1.5);
        assert_eq!(s_zero.typography.font_size_base, t.typography.font_size_base);
        assert_eq!(s_neg.typography.font_size_base, t.typography.font_size_base);
    }

    #[test]
    fn with_scale_one_point_five_rounds_correctly() {
        let s = Theme::dark().with_scale(1.5);
        assert_eq!(s.typography.font_size_base, 21); // 14 * 1.5 = 21.0
        assert_eq!(s.typography.font_size_small, 18); // 12 * 1.5 = 18.0
        assert_eq!(s.spacing.base, 12); // 8 * 1.5 = 12.0
        // borders.width: 1 * 1.5 = 1.5 → rounds to 2
        assert_eq!(s.borders.width, 2);
    }

    #[test]
    fn with_scale_clamps_border_width_to_at_least_one() {
        // At very small scales, rounding would zero-out the border.
        // We force a minimum of 1 px so borders never silently disappear.
        let s = Theme::dark().with_scale(0.1);
        assert_eq!(s.borders.width, 1);
    }
}
