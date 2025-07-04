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
        let width = size.width
            .max(self.min_width.unwrap_or(0))
            .min(self.max_width.unwrap_or(i32::MAX));
            
        let height = size.height
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