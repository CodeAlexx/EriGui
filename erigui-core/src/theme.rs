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

    // ============================================================
    // Ported palettes (from serenity/serenity/ui/theme.py)
    //
    // Mapping rules (ImGui palette → EriGui ThemeColors):
    //   window_bg            → background
    //   child_bg             → surface
    //   popup_bg / frame_bg  → surface_variant
    //   text                 → text
    //   text_disabled        → text_disabled
    //   text_selected_bg     → selection
    //   button*              → primary*
    //   header*              → secondary*
    //   border               → border
    //   modal_dim_bg         → overlay
    //
    // Slots without an ImGui equivalent (success / warning / error /
    // info / border_focus / shadow / text_secondary / primary_disabled)
    // are assigned palette-appropriate defaults.
    // ============================================================

    /// Serenity palette — indigo + soft purple. Default for the
    /// serenity training UI. Source: `theme.py:PALETTE_SERENITY`.
    pub fn serenity() -> Self {
        Self {
            name: "Serenity".to_string(),
            colors: ThemeColors {
                background: Color::rgb(26, 26, 46),
                surface: Color::rgb(22, 33, 62),
                surface_variant: Color::rgb(30, 30, 52),
                text: Color::rgb(232, 232, 232),
                text_secondary: Color::rgb(165, 174, 205),
                text_disabled: Color::rgb(102, 102, 102),
                primary: Color::rgb(67, 97, 238),
                primary_hover: Color::rgb(90, 120, 240),
                primary_active: Color::rgb(52, 81, 222),
                primary_disabled: Color::rgb(40, 50, 90),
                secondary: Color::rgb(15, 52, 96),
                secondary_hover: Color::rgb(20, 65, 120),
                secondary_active: Color::rgb(25, 78, 140),
                success: Color::rgb(124, 232, 196),
                warning: Color::rgb(242, 201, 76),
                error: Color::rgb(239, 107, 115),
                info: Color::rgb(95, 212, 255),
                border: Color::rgb(42, 42, 74),
                border_hover: Color::rgb(70, 70, 110),
                border_focus: Color::rgb(67, 97, 238),
                selection: Color::rgba(67, 97, 238, 100),
                shadow: Color::rgba(15, 15, 30, 200),
                overlay: Color::rgba(0, 0, 0, 140),
            },
            ..Self::dark()
        }
    }

    /// Moonlight palette — neutral gray with yellow accent. Source:
    /// `theme.py:PALETTE_MOONLIGHT`.
    pub fn moonlight() -> Self {
        Self {
            name: "Moonlight".to_string(),
            colors: ThemeColors {
                background: Color::rgb(20, 22, 26),
                surface: Color::rgb(24, 26, 30),
                surface_variant: Color::rgb(29, 32, 39),
                text: Color::rgb(255, 255, 255),
                text_secondary: Color::rgb(180, 180, 200),
                text_disabled: Color::rgb(70, 81, 115),
                primary: Color::rgb(248, 255, 127),
                primary_hover: Color::rgb(255, 255, 160),
                primary_active: Color::rgb(255, 203, 127),
                primary_disabled: Color::rgb(60, 60, 60),
                secondary: Color::rgb(36, 42, 53),
                secondary_hover: Color::rgb(50, 60, 75),
                secondary_active: Color::rgb(40, 47, 64),
                success: Color::rgb(120, 220, 140),
                warning: Color::rgb(248, 255, 127),
                error: Color::rgb(220, 100, 100),
                info: Color::rgb(140, 200, 240),
                border: Color::rgb(40, 43, 49),
                border_hover: Color::rgb(60, 65, 75),
                border_focus: Color::rgb(248, 255, 127),
                selection: Color::rgba(248, 255, 127, 100),
                shadow: Color::rgba(0, 0, 0, 160),
                overlay: Color::rgba(50, 45, 139, 128),
            },
            ..Self::dark()
        }
    }

    /// Monochrome palette — cyan-on-black, hacker terminal aesthetic.
    /// Source: `theme.py:PALETTE_MONOCHROME`.
    pub fn monochrome() -> Self {
        Self {
            name: "Monochrome".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0, 0, 0),
                surface: Color::rgb(0, 16, 16),
                surface_variant: Color::rgb(0, 33, 33),
                text: Color::rgb(0, 255, 255),
                text_secondary: Color::rgb(0, 200, 200),
                text_disabled: Color::rgb(0, 102, 105),
                primary: Color::rgb(0, 255, 255),
                primary_hover: Color::rgb(80, 255, 255),
                primary_active: Color::rgb(0, 200, 200),
                primary_disabled: Color::rgb(0, 60, 60),
                secondary: Color::rgba(0, 255, 255, 84).with_alpha(255),
                secondary_hover: Color::rgb(0, 180, 180),
                secondary_active: Color::rgb(0, 230, 230),
                success: Color::rgb(0, 255, 128),
                warning: Color::rgb(255, 255, 0),
                error: Color::rgb(255, 80, 80),
                info: Color::rgb(0, 255, 255),
                border: Color::rgba(0, 255, 255, 166),
                border_hover: Color::rgb(0, 255, 255),
                border_focus: Color::rgb(0, 255, 255),
                selection: Color::rgba(0, 255, 255, 56),
                shadow: Color::rgba(0, 0, 0, 0),
                overlay: Color::rgba(10, 26, 23, 130),
            },
            ..Self::dark()
        }
    }

    /// Nord palette — cool arctic blue/gray. Source:
    /// `theme.py:PALETTE_NORD`.
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            colors: ThemeColors {
                background: Color::rgb(46, 52, 64),
                surface: Color::rgb(41, 43, 51),
                surface_variant: Color::rgb(59, 66, 82),
                text: Color::rgb(216, 222, 233),
                text_secondary: Color::rgb(180, 190, 210),
                text_disabled: Color::rgb(125, 128, 135),
                primary: Color::rgb(46, 52, 64),
                primary_hover: Color::rgb(130, 161, 194),
                primary_active: Color::rgb(94, 130, 171),
                primary_disabled: Color::rgb(60, 65, 75),
                secondary: Color::rgb(130, 161, 194),
                secondary_hover: Color::rgb(135, 191, 209),
                secondary_active: Color::rgb(94, 130, 171),
                success: Color::rgb(163, 190, 140),
                warning: Color::rgb(235, 203, 139),
                error: Color::rgb(191, 97, 106),
                info: Color::rgb(143, 188, 187),
                border: Color::rgb(36, 41, 49),
                border_hover: Color::rgb(76, 86, 106),
                border_focus: Color::rgb(143, 188, 187),
                selection: Color::rgba(94, 130, 171, 100),
                shadow: Color::rgba(0, 0, 0, 100),
                overlay: Color::rgba(26, 26, 38, 153),
            },
            ..Self::dark()
        }
    }

    /// Cinder palette — slate dark with red accent. Source:
    /// `theme.py:PALETTE_CINDER`.
    pub fn cinder() -> Self {
        Self {
            name: "Cinder".to_string(),
            colors: ThemeColors {
                background: Color::rgb(33, 36, 43),
                surface: Color::rgb(33, 36, 43),
                surface_variant: Color::rgb(51, 56, 69),
                text: Color::rgb(220, 237, 227),
                text_secondary: Color::rgb(180, 195, 188),
                text_disabled: Color::rgb(110, 120, 115),
                primary: Color::rgb(235, 46, 74),
                primary_hover: Color::rgb(255, 80, 100),
                primary_active: Color::rgb(180, 35, 60),
                primary_disabled: Color::rgb(80, 30, 40),
                secondary: Color::rgb(120, 196, 211),
                secondary_hover: Color::rgb(160, 220, 230),
                secondary_active: Color::rgb(90, 160, 180),
                success: Color::rgb(120, 200, 140),
                warning: Color::rgb(235, 180, 80),
                error: Color::rgb(235, 46, 74),
                info: Color::rgb(120, 196, 211),
                border: Color::rgb(36, 41, 49),
                border_hover: Color::rgb(70, 80, 90),
                border_focus: Color::rgb(235, 46, 74),
                selection: Color::rgba(235, 46, 74, 110),
                shadow: Color::rgba(0, 0, 0, 100),
                overlay: Color::rgba(51, 56, 69, 186),
            },
            ..Self::dark()
        }
    }

    /// Blender palette — neutral gray with blue selection accent.
    /// Mirrors the Blender 3D editor look. Source:
    /// `theme.py:PALETTE_BLENDER`.
    pub fn blender() -> Self {
        Self {
            name: "Blender".to_string(),
            colors: ThemeColors {
                background: Color::rgb(56, 56, 56),
                surface: Color::rgb(48, 48, 48),
                surface_variant: Color::rgb(84, 84, 84),
                text: Color::rgb(214, 214, 214),
                text_secondary: Color::rgb(160, 160, 160),
                text_disabled: Color::rgb(127, 127, 127),
                primary: Color::rgb(71, 114, 179),
                primary_hover: Color::rgb(90, 130, 195),
                primary_active: Color::rgb(48, 99, 176),
                primary_disabled: Color::rgb(70, 70, 70),
                secondary: Color::rgb(69, 69, 69),
                secondary_hover: Color::rgb(102, 102, 102),
                secondary_active: Color::rgb(48, 99, 176),
                success: Color::rgb(120, 180, 120),
                warning: Color::rgb(220, 180, 80),
                error: Color::rgb(220, 100, 100),
                info: Color::rgb(120, 160, 200),
                border: Color::rgb(43, 43, 43),
                border_hover: Color::rgb(70, 70, 70),
                border_focus: Color::rgb(71, 114, 179),
                selection: Color::rgba(71, 114, 179, 130),
                shadow: Color::rgba(0, 0, 0, 100),
                overlay: Color::rgba(26, 26, 26, 153),
            },
            ..Self::dark()
        }
    }

    /// Cyberpunk palette — neon cyan/magenta on near-black. Source:
    /// `theme.py:PALETTE_CYBERPUNK`.
    pub fn cyberpunk() -> Self {
        Self {
            name: "Cyberpunk".to_string(),
            colors: ThemeColors {
                background: Color::rgb(0, 10, 31),
                surface: Color::rgb(8, 10, 56),
                surface_variant: Color::rgb(31, 15, 69),
                text: Color::rgb(0, 209, 255),
                text_secondary: Color::rgb(140, 220, 255),
                text_disabled: Color::rgb(0, 92, 161),
                primary: Color::rgb(0, 250, 255),
                primary_hover: Color::rgb(240, 0, 255),
                primary_active: Color::rgb(2, 0, 255),
                primary_disabled: Color::rgb(50, 50, 90),
                secondary: Color::rgb(155, 0, 255),
                secondary_hover: Color::rgb(190, 80, 255),
                secondary_active: Color::rgb(120, 0, 200),
                success: Color::rgb(0, 255, 217),
                warning: Color::rgb(255, 230, 0),
                error: Color::rgb(255, 50, 100),
                info: Color::rgb(0, 220, 255),
                border: Color::rgb(155, 0, 255),
                border_hover: Color::rgb(0, 250, 255),
                border_focus: Color::rgb(0, 250, 255),
                selection: Color::rgba(0, 209, 255, 100),
                shadow: Color::rgba(0, 0, 0, 0),
                overlay: Color::rgba(13, 0, 51, 153),
            },
            ..Self::dark()
        }
    }

    /// All themes ported from serenity, in display order. Useful for
    /// building a theme-picker dropdown — iterate and call `.name()`
    /// on each, then dispatch back via [`Theme::by_name`].
    pub fn all_named() -> Vec<Theme> {
        vec![
            Self::dark(),
            Self::light(),
            Self::alex_jammin(),
            Self::serenity(),
            Self::moonlight(),
            Self::monochrome(),
            Self::nord(),
            Self::cinder(),
            Self::blender(),
            Self::cyberpunk(),
        ]
    }

    /// Look up a theme by display name. Case-insensitive. Returns
    /// `None` if the name doesn't match any built-in theme.
    /// Use this to wire a theme-picker dropdown back to a `Theme`
    /// without keeping closures around.
    pub fn by_name(name: &str) -> Option<Theme> {
        let n = name.trim().to_ascii_lowercase();
        Some(match n.as_str() {
            "dark" => Self::dark(),
            "light" => Self::light(),
            "alexjammin" | "alex_jammin" | "alex jammin" => Self::alex_jammin(),
            "serenity" => Self::serenity(),
            "moonlight" => Self::moonlight(),
            "monochrome" => Self::monochrome(),
            "nord" => Self::nord(),
            "cinder" => Self::cinder(),
            "blender" => Self::blender(),
            "cyberpunk" => Self::cyberpunk(),
            _ => return None,
        })
    }

    /// The display name of this theme. Always equal to the `name`
    /// field; provided as a method so call sites don't reach into
    /// the struct directly.
    pub fn name(&self) -> &str {
        &self.name
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
        themes.insert("Serenity".to_string(), Theme::serenity());
        themes.insert("Moonlight".to_string(), Theme::moonlight());
        themes.insert("Monochrome".to_string(), Theme::monochrome());
        themes.insert("Nord".to_string(), Theme::nord());
        themes.insert("Cinder".to_string(), Theme::cinder());
        themes.insert("Blender".to_string(), Theme::blender());
        themes.insert("Cyberpunk".to_string(), Theme::cyberpunk());

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
    fn manager_default_starts_with_builtin_themes() {
        let m = ThemeManager::default();
        let names = m.list_themes();
        // Three originals + seven serenity-port palettes.
        assert_eq!(names.len(), 10);
        for required in [
            "Light",
            "Dark",
            "alexJammin",
            "Serenity",
            "Moonlight",
            "Monochrome",
            "Nord",
            "Cinder",
            "Blender",
            "Cyberpunk",
        ] {
            assert!(
                names.contains(&required),
                "default ThemeManager should contain {required}"
            );
        }
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
        let baseline = m.list_themes().len();
        let mut t = Theme::dark();
        t.name = "Extra".to_string();
        m.add_theme(t);
        assert_eq!(m.list_themes().len(), baseline + 1);
    }

    // ----- ported palettes (serenity) -----

    #[test]
    fn ported_palettes_round_trip_via_by_name() {
        let canon = [
            ("Serenity", Theme::serenity()),
            ("Moonlight", Theme::moonlight()),
            ("Monochrome", Theme::monochrome()),
            ("Nord", Theme::nord()),
            ("Cinder", Theme::cinder()),
            ("Blender", Theme::blender()),
            ("Cyberpunk", Theme::cyberpunk()),
        ];
        for (name, expected) in canon {
            let by_name = Theme::by_name(name).expect("by_name must resolve");
            assert_eq!(by_name.name(), expected.name());
        }
    }

    #[test]
    fn by_name_is_case_insensitive_and_handles_unknown() {
        assert!(Theme::by_name("DARK").is_some());
        assert!(Theme::by_name(" cyberpunk ").is_some());
        assert!(Theme::by_name("does_not_exist").is_none());
    }

    #[test]
    fn all_named_returns_every_built_in_theme() {
        let all = Theme::all_named();
        // 3 originals + 7 serenity-port palettes.
        assert_eq!(all.len(), 10);
        // No empty names; every entry must round-trip via by_name.
        for theme in &all {
            assert!(!theme.name().is_empty());
            assert!(Theme::by_name(theme.name()).is_some());
        }
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
