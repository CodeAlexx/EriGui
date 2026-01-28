//! Tests for core types: Point, Size, Rect, Color, Margins, LayoutConstraints

use erigui_core::{Color, LayoutConstraints, Margins, Point, Rect, Size};

// ============================================================================
// Point Tests
// ============================================================================

#[test]
fn test_point_creation() {
    let p = Point::new(10, 20);
    assert_eq!(p.x, 10);
    assert_eq!(p.y, 20);
}

#[test]
fn test_point_zero() {
    assert_eq!(Point::ZERO, Point::new(0, 0));
}

#[test]
fn test_point_from_tuple() {
    let p: Point = (5, 15).into();
    assert_eq!(p.x, 5);
    assert_eq!(p.y, 15);
}

#[test]
fn test_point_addition() {
    let p1 = Point::new(10, 20);
    let p2 = Point::new(5, 15);
    let result = p1 + p2;
    assert_eq!(result.x, 15);
    assert_eq!(result.y, 35);
}

#[test]
fn test_point_subtraction() {
    let p1 = Point::new(10, 20);
    let p2 = Point::new(5, 15);
    let result = p1 - p2;
    assert_eq!(result.x, 5);
    assert_eq!(result.y, 5);
}

#[test]
fn test_point_to_vec2() {
    let p = Point::new(10, 20);
    let v = p.to_vec2();
    assert_eq!(v.x, 10.0);
    assert_eq!(v.y, 20.0);
}

// ============================================================================
// Size Tests
// ============================================================================

#[test]
fn test_size_creation() {
    let s = Size::new(100, 200);
    assert_eq!(s.width, 100);
    assert_eq!(s.height, 200);
}

#[test]
fn test_size_zero() {
    assert_eq!(Size::ZERO, Size::new(0, 0));
}

#[test]
fn test_size_area() {
    let s = Size::new(10, 20);
    assert_eq!(s.area(), 200);
}

#[test]
fn test_size_from_tuple() {
    let s: Size = (50, 100).into();
    assert_eq!(s.width, 50);
    assert_eq!(s.height, 100);
}

// ============================================================================
// Rect Tests
// ============================================================================

#[test]
fn test_rect_creation() {
    let r = Rect::new(10, 20, 100, 200);
    assert_eq!(r.x(), 10);
    assert_eq!(r.y(), 20);
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 200);
}

#[test]
fn test_rect_from_origin_size() {
    let origin = Point::new(10, 20);
    let size = Size::new(100, 200);
    let r = Rect::from_origin_size(origin, size);
    assert_eq!(r.x(), 10);
    assert_eq!(r.y(), 20);
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 200);
}

#[test]
fn test_rect_bounds() {
    let r = Rect::new(10, 20, 100, 200);
    assert_eq!(r.right(), 110);
    assert_eq!(r.bottom(), 220);
}

#[test]
fn test_rect_center() {
    let r = Rect::new(0, 0, 100, 200);
    let center = r.center();
    assert_eq!(center.x, 50);
    assert_eq!(center.y, 100);
}

#[test]
fn test_rect_contains_inside() {
    let r = Rect::new(0, 0, 100, 100);
    assert!(r.contains(Point::new(50, 50)));
}

#[test]
fn test_rect_contains_on_edge() {
    let r = Rect::new(0, 0, 100, 100);
    assert!(r.contains(Point::new(0, 0))); // top-left is inside
    assert!(!r.contains(Point::new(100, 100))); // bottom-right is outside (exclusive)
}

#[test]
fn test_rect_contains_outside() {
    let r = Rect::new(0, 0, 100, 100);
    assert!(!r.contains(Point::new(-1, 50)));
    assert!(!r.contains(Point::new(101, 50)));
    assert!(!r.contains(Point::new(50, -1)));
    assert!(!r.contains(Point::new(50, 101)));
}

#[test]
fn test_rect_intersects() {
    let r1 = Rect::new(0, 0, 100, 100);
    let r2 = Rect::new(50, 50, 100, 100);
    assert!(r1.intersects(&r2));
}

#[test]
fn test_rect_no_intersection() {
    let r1 = Rect::new(0, 0, 50, 50);
    let r2 = Rect::new(100, 100, 50, 50);
    assert!(!r1.intersects(&r2));
}

#[test]
fn test_rect_intersection() {
    let r1 = Rect::new(0, 0, 100, 100);
    let r2 = Rect::new(50, 50, 100, 100);
    let intersection = r1.intersection(&r2).unwrap();
    assert_eq!(intersection.x(), 50);
    assert_eq!(intersection.y(), 50);
    assert_eq!(intersection.width(), 50);
    assert_eq!(intersection.height(), 50);
}

#[test]
fn test_rect_union() {
    let r1 = Rect::new(0, 0, 50, 50);
    let r2 = Rect::new(25, 25, 50, 50);
    let union = r1.union(&r2);
    assert_eq!(union.x(), 0);
    assert_eq!(union.y(), 0);
    assert_eq!(union.width(), 75);
    assert_eq!(union.height(), 75);
}

#[test]
fn test_rect_inset() {
    let r = Rect::new(0, 0, 100, 100);
    let inset = r.inset(10);
    assert_eq!(inset.x(), 10);
    assert_eq!(inset.y(), 10);
    assert_eq!(inset.width(), 80);
    assert_eq!(inset.height(), 80);
}

#[test]
fn test_rect_inset_clamps_to_zero() {
    let r = Rect::new(0, 0, 20, 20);
    let inset = r.inset(15); // 15*2 = 30 > 20
    assert_eq!(inset.width(), 0);
    assert_eq!(inset.height(), 0);
}

// ============================================================================
// Color Tests
// ============================================================================

#[test]
fn test_color_rgb() {
    let c = Color::rgb(100, 150, 200);
    assert_eq!(c.r, 100);
    assert_eq!(c.g, 150);
    assert_eq!(c.b, 200);
    assert_eq!(c.a, 255);
}

#[test]
fn test_color_rgba() {
    let c = Color::rgba(100, 150, 200, 128);
    assert_eq!(c.r, 100);
    assert_eq!(c.g, 150);
    assert_eq!(c.b, 200);
    assert_eq!(c.a, 128);
}

#[test]
fn test_color_from_hex() {
    let c = Color::from_hex(0xFF8040);
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 128);
    assert_eq!(c.b, 64);
    assert_eq!(c.a, 255);
}

#[test]
fn test_color_with_alpha() {
    let c = Color::RED.with_alpha(128);
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 128);
}

#[test]
fn test_color_constants() {
    assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
    assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
    assert_eq!(Color::RED, Color::rgb(255, 0, 0));
    assert_eq!(Color::GREEN, Color::rgb(0, 255, 0));
    assert_eq!(Color::BLUE, Color::rgb(0, 0, 255));
    assert_eq!(Color::TRANSPARENT, Color::rgba(0, 0, 0, 0));
}

#[test]
fn test_color_to_gl() {
    let c = Color::rgba(255, 128, 64, 255);
    let gl = c.to_gl_color();
    assert!((gl[0] - 1.0).abs() < 0.01);
    assert!((gl[1] - 0.502).abs() < 0.01);
    assert!((gl[2] - 0.251).abs() < 0.01);
    assert!((gl[3] - 1.0).abs() < 0.01);
}

// ============================================================================
// Margins Tests
// ============================================================================

#[test]
fn test_margins_all() {
    let m = Margins::all(10);
    assert_eq!(m.top, 10);
    assert_eq!(m.right, 10);
    assert_eq!(m.bottom, 10);
    assert_eq!(m.left, 10);
}

#[test]
fn test_margins_symmetric() {
    let m = Margins::symmetric(10, 20);
    assert_eq!(m.top, 10);
    assert_eq!(m.bottom, 10);
    assert_eq!(m.left, 20);
    assert_eq!(m.right, 20);
}

#[test]
fn test_margins_new() {
    let m = Margins::new(1, 2, 3, 4);
    assert_eq!(m.top, 1);
    assert_eq!(m.right, 2);
    assert_eq!(m.bottom, 3);
    assert_eq!(m.left, 4);
}

#[test]
fn test_margins_horizontal() {
    let m = Margins::new(0, 10, 0, 20);
    assert_eq!(m.horizontal(), 30);
}

#[test]
fn test_margins_vertical() {
    let m = Margins::new(10, 0, 20, 0);
    assert_eq!(m.vertical(), 30);
}

#[test]
fn test_margins_zero() {
    assert_eq!(Margins::ZERO, Margins::all(0));
}

// ============================================================================
// LayoutConstraints Tests
// ============================================================================

#[test]
fn test_constraints_unbounded() {
    let c = LayoutConstraints::UNBOUNDED;
    assert_eq!(c.min_width, None);
    assert_eq!(c.max_width, None);
    assert_eq!(c.min_height, None);
    assert_eq!(c.max_height, None);
}

#[test]
fn test_constraints_bounded() {
    let c = LayoutConstraints::bounded(100, 200);
    assert_eq!(c.min_width, Some(100));
    assert_eq!(c.max_width, Some(100));
    assert_eq!(c.min_height, Some(200));
    assert_eq!(c.max_height, Some(200));
}

#[test]
fn test_constraints_constrain() {
    let c = LayoutConstraints::UNBOUNDED
        .with_min_width(50)
        .with_max_width(200)
        .with_min_height(50)
        .with_max_height(200);

    // Test clamping within bounds
    let size = c.constrain(Size::new(100, 150));
    assert_eq!(size.width, 100);
    assert_eq!(size.height, 150);

    // Test clamping below min
    let size = c.constrain(Size::new(10, 10));
    assert_eq!(size.width, 50);
    assert_eq!(size.height, 50);

    // Test clamping above max
    let size = c.constrain(Size::new(300, 300));
    assert_eq!(size.width, 200);
    assert_eq!(size.height, 200);
}

#[test]
fn test_constraints_loosen() {
    let c = LayoutConstraints::bounded(100, 200).loosen();
    assert_eq!(c.min_width, None);
    assert_eq!(c.min_height, None);
    assert_eq!(c.max_width, Some(100));
    assert_eq!(c.max_height, Some(200));
}

#[test]
fn test_constraints_tighten() {
    let c = LayoutConstraints::UNBOUNDED.tighten(Size::new(100, 200));
    assert_eq!(c.min_width, Some(100));
    assert_eq!(c.max_width, Some(100));
    assert_eq!(c.min_height, Some(200));
    assert_eq!(c.max_height, Some(200));
}

#[test]
fn test_constraints_deflate() {
    let c = LayoutConstraints::bounded(100, 200);
    let margins = Margins::all(10);
    let deflated = c.deflate(margins);
    assert_eq!(deflated.min_width, Some(80));
    assert_eq!(deflated.max_width, Some(80));
    assert_eq!(deflated.min_height, Some(180));
    assert_eq!(deflated.max_height, Some(180));
}
