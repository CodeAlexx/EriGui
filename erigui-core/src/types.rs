use glam::Vec2;
use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0, y: 0 };
    
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Self {
        Self { x, y }
    }
}

impl Add for Point {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const ZERO: Self = Self { width: 0, height: 0 };
    
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
    
    pub fn area(self) -> i32 {
        self.width * self.height
    }
}

impl From<(i32, i32)> for Size {
    fn from((width, height): (i32, i32)) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }
    
    pub fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }
    
    pub fn x(&self) -> i32 {
        self.origin.x
    }
    
    pub fn y(&self) -> i32 {
        self.origin.y
    }
    
    pub fn width(&self) -> i32 {
        self.size.width
    }
    
    pub fn height(&self) -> i32 {
        self.size.height
    }
    
    pub fn right(&self) -> i32 {
        self.origin.x + self.size.width
    }
    
    pub fn bottom(&self) -> i32 {
        self.origin.y + self.size.height
    }
    
    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width / 2,
            self.origin.y + self.size.height / 2,
        )
    }
    
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x < self.right()
            && point.y >= self.origin.y
            && point.y < self.bottom()
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        self.origin.x < other.right()
            && self.right() > other.origin.x
            && self.origin.y < other.bottom()
            && self.bottom() > other.origin.y
    }
    
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        
        let x = self.origin.x.max(other.origin.x);
        let y = self.origin.y.max(other.origin.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        
        Some(Rect::new(x, y, right - x, bottom - y))
    }
    
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.origin.x.min(other.origin.x);
        let y = self.origin.y.min(other.origin.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        
        Rect::new(x, y, right - x, bottom - y)
    }
    
    pub fn inset(&self, amount: i32) -> Rect {
        Rect::new(
            self.origin.x + amount,
            self.origin.y + amount,
            (self.size.width - 2 * amount).max(0),
            (self.size.height - 2 * amount).max(0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    pub fn from_hex(hex: u32) -> Self {
        Self::rgb(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }
    
    pub fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }
    
    pub fn to_gl_color(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Margins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Margins {
    pub const ZERO: Self = Self::all(0);
    
    pub const fn all(value: i32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
    
    pub const fn symmetric(vertical: i32, horizontal: i32) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }
    
    pub const fn new(top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self { top, right, bottom, left }
    }
    
    pub fn horizontal(&self) -> i32 {
        self.left + self.right
    }
    
    pub fn vertical(&self) -> i32 {
        self.top + self.bottom
    }
}

pub type WidgetId = slotmap::DefaultKey;