use erigui_core::{Color, DrawContext, Point, Rect};

pub struct Icon;

impl Icon {
    pub fn draw_back_arrow(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.6;

        // Draw arrow pointing left
        let points = vec![
            Point::new((center.x as f32 - size * 0.3) as i32, center.y),
            Point::new(
                (center.x as f32 + size * 0.2) as i32,
                (center.y as f32 - size * 0.4) as i32,
            ),
            Point::new(
                (center.x as f32 + size * 0.2) as i32,
                (center.y as f32 + size * 0.4) as i32,
            ),
        ];

        context.set_color(color);
        context.fill_polygon(&points);
    }

    pub fn draw_forward_arrow(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.6;

        // Draw arrow pointing right
        let points = vec![
            Point::new((center.x as f32 + size * 0.3) as i32, center.y),
            Point::new(
                (center.x as f32 - size * 0.2) as i32,
                (center.y as f32 - size * 0.4) as i32,
            ),
            Point::new(
                (center.x as f32 - size * 0.2) as i32,
                (center.y as f32 + size * 0.4) as i32,
            ),
        ];

        context.set_color(color);
        context.fill_polygon(&points);
    }

    pub fn draw_up_arrow(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.6;

        // Draw arrow pointing up
        let points = vec![
            Point::new(center.x, (center.y as f32 - size * 0.3) as i32),
            Point::new(
                (center.x as f32 - size * 0.4) as i32,
                (center.y as f32 + size * 0.2) as i32,
            ),
            Point::new(
                (center.x as f32 + size * 0.4) as i32,
                (center.y as f32 + size * 0.2) as i32,
            ),
        ];

        context.set_color(color);
        context.fill_polygon(&points);
    }

    pub fn draw_home(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.7;

        // Draw house shape
        let roof_points = vec![
            Point::new(center.x, (center.y as f32 - size * 0.4) as i32),
            Point::new((center.x as f32 - size * 0.5) as i32, center.y),
            Point::new((center.x as f32 + size * 0.5) as i32, center.y),
        ];

        context.set_color(color);
        context.fill_polygon(&roof_points);

        // Draw house body
        let body_rect = Rect::new(
            (center.x as f32 - size * 0.4) as i32,
            center.y,
            (size * 0.8) as i32,
            (size * 0.5) as i32,
        );
        context.fill_rect(body_rect);
    }

    pub fn draw_refresh(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let radius = bounds.width().min(bounds.height()) as f32 * 0.35;

        context.set_color(color);
        context.set_line_width(2);

        // Draw circular arrow
        let segments = 16;
        let start_angle = 0.0;
        let end_angle = std::f32::consts::PI * 1.5;

        for i in 0..segments {
            let angle1 = start_angle + (end_angle - start_angle) * (i as f32) / (segments as f32);
            let angle2 =
                start_angle + (end_angle - start_angle) * ((i + 1) as f32) / (segments as f32);

            let x1 = center.x + (radius * angle1.cos()) as i32;
            let y1 = center.y + (radius * angle1.sin()) as i32;
            let x2 = center.x + (radius * angle2.cos()) as i32;
            let y2 = center.y + (radius * angle2.sin()) as i32;

            context.draw_line(Point::new(x1, y1), Point::new(x2, y2), 2);
        }

        // Draw arrow head
        let arrow_size = radius * 0.3;
        let arrow_angle = end_angle;
        let arrow_x = center.x + (radius * arrow_angle.cos()) as i32;
        let arrow_y = center.y + (radius * arrow_angle.sin()) as i32;

        let arrow_points = vec![
            Point::new(arrow_x, arrow_y),
            Point::new(
                arrow_x + (arrow_size * (arrow_angle - 2.5).cos()) as i32,
                arrow_y + (arrow_size * (arrow_angle - 2.5).sin()) as i32,
            ),
            Point::new(
                arrow_x + (arrow_size * (arrow_angle + 0.5).cos()) as i32,
                arrow_y + (arrow_size * (arrow_angle + 0.5).sin()) as i32,
            ),
        ];

        context.fill_polygon(&arrow_points);
    }

    pub fn draw_folder(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let width = bounds.width() as f32 * 0.8;
        let height = bounds.height() as f32 * 0.6;

        let x = center.x as f32 - width * 0.5;
        let y = center.y as f32 - height * 0.5;

        context.set_color(color);

        // Draw folder tab
        let tab_width = width * 0.4;
        let tab_height = height * 0.2;
        let tab_rect = Rect::new(x as i32, y as i32, tab_width as i32, tab_height as i32);
        context.fill_rounded_rect(tab_rect, 3);

        // Draw folder body
        let body_rect = Rect::new(
            x as i32,
            (y + tab_height * 0.7) as i32,
            width as i32,
            (height - tab_height * 0.7) as i32,
        );
        context.fill_rounded_rect(body_rect, 3);
    }

    pub fn draw_folder_new(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        // Draw folder first
        Self::draw_folder(context, bounds, color);

        // Draw plus sign
        let center = bounds.center();
        let plus_size = bounds.width().min(bounds.height()) as f32 * 0.3;
        let x_offset = bounds.width() as f32 * 0.25;
        let y_offset = bounds.height() as f32 * 0.15;

        context.set_color(color.with_alpha(200));

        // Horizontal line
        context.fill_rect(Rect::new(
            (center.x as f32 + x_offset - plus_size * 0.5) as i32,
            (center.y as f32 + y_offset) as i32,
            plus_size as i32,
            2,
        ));

        // Vertical line
        context.fill_rect(Rect::new(
            (center.x as f32 + x_offset) as i32,
            (center.y as f32 + y_offset - plus_size * 0.5) as i32,
            2,
            plus_size as i32,
        ));
    }

    pub fn draw_delete(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.6;

        context.set_color(color);

        // Draw trash can body
        let body_width = size * 0.6;
        let body_height = size * 0.7;
        let body_rect = Rect::new(
            (center.x as f32 - body_width * 0.5) as i32,
            (center.y as f32 - body_height * 0.3) as i32,
            body_width as i32,
            body_height as i32,
        );
        context.fill_rounded_rect(body_rect, 2);

        // Draw lid
        let lid_width = size * 0.8;
        let lid_rect = Rect::new(
            (center.x as f32 - lid_width * 0.5) as i32,
            (center.y as f32 - body_height * 0.4) as i32,
            lid_width as i32,
            (size * 0.1) as i32,
        );
        context.fill_rect(lid_rect);

        // Draw handle
        let handle_width = size * 0.3;
        let handle_rect = Rect::new(
            (center.x as f32 - handle_width * 0.5) as i32,
            (center.y as f32 - body_height * 0.5) as i32,
            handle_width as i32,
            (size * 0.15) as i32,
        );
        context.fill_rounded_rect(handle_rect, 2);
    }

    pub fn draw_file(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let width = bounds.width() as f32 * 0.6;
        let height = bounds.height() as f32 * 0.8;

        let x = center.x as f32 - width * 0.5;
        let y = center.y as f32 - height * 0.5;

        context.set_color(color);

        // Draw page with folded corner
        let corner_size = width * 0.2;

        // Main body
        let points = vec![
            Point::new(x as i32, y as i32),
            Point::new((x + width - corner_size) as i32, y as i32),
            Point::new((x + width) as i32, (y + corner_size) as i32),
            Point::new((x + width) as i32, (y + height) as i32),
            Point::new(x as i32, (y + height) as i32),
        ];

        context.fill_polygon(&points);

        // Draw corner fold
        context.set_color(color.with_alpha(150));
        let fold_points = vec![
            Point::new((x + width - corner_size) as i32, y as i32),
            Point::new((x + width) as i32, (y + corner_size) as i32),
            Point::new((x + width - corner_size) as i32, (y + corner_size) as i32),
        ];
        context.fill_polygon(&fold_points);
    }

    pub fn draw_search(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.7;

        context.set_color(color);

        // Draw magnifying glass circle
        let glass_radius = size * 0.35;
        let glass_center = Point::new(
            (center.x as f32 - size * 0.1) as i32,
            (center.y as f32 - size * 0.1) as i32,
        );

        context.set_line_width(3);
        context.draw_ellipse(glass_center, glass_radius as i32, glass_radius as i32, 32);

        // Draw handle
        let handle_start = Point::new(
            (glass_center.x as f32 + glass_radius * 0.7) as i32,
            (glass_center.y as f32 + glass_radius * 0.7) as i32,
        );
        let handle_end = Point::new(
            (center.x as f32 + size * 0.4) as i32,
            (center.y as f32 + size * 0.4) as i32,
        );

        context.draw_line(handle_start, handle_end, 3);
    }

    pub fn draw_chevron_right(context: &mut dyn DrawContext, bounds: Rect, color: Color) {
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) as f32 * 0.5;

        // Draw chevron pointing right
        let points = [Point::new(
                (center.x as f32 - size * 0.3) as i32,
                (center.y as f32 - size * 0.5) as i32,
            ),
            Point::new((center.x as f32 + size * 0.3) as i32, center.y),
            Point::new(
                (center.x as f32 - size * 0.3) as i32,
                (center.y as f32 + size * 0.5) as i32,
            )];

        context.set_color(color);
        context.set_line_width(2);

        // Draw as lines instead of filled polygon for chevron style
        context.draw_line(points[0], points[1], 2);
        context.draw_line(points[1], points[2], 2);
    }

    pub fn draw_gear(
        context: &mut dyn DrawContext,
        bounds: Rect,
        color: Color,
        bg_color: Color,
    ) {
        let center = bounds.center();
        let outer_radius = bounds.width().min(bounds.height()) as f32 * 0.4;
        let inner_radius = outer_radius * 0.6;
        let teeth = 8;

        context.set_color(color);

        let mut points = Vec::new();
        for i in 0..teeth * 2 {
            let angle = std::f32::consts::PI * 2.0 * (i as f32) / ((teeth * 2) as f32);
            let radius = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };

            points.push(Point::new(
                (center.x as f32 + radius * angle.cos()) as i32,
                (center.y as f32 + radius * angle.sin()) as i32,
            ));
        }

        context.fill_polygon(&points);

        // Draw center hole. Re-fill with the surface/background color
        // rather than `color.with_alpha(0)` -- alpha-zero is a no-op
        // pixel paint, so the hole would never appear over the gear
        // body. Caller passes the underlying surface color.
        context.set_color(bg_color);
        context.fill_ellipse(
            center,
            (inner_radius * 0.5) as i32,
            (inner_radius * 0.5) as i32,
            16,
        );
    }

    pub fn draw_palette(
        context: &mut dyn DrawContext,
        bounds: Rect,
        color: Color,
        bg_color: Color,
    ) {
        // Draw artist's palette icon
        let center = bounds.center();
        let width = bounds.width() as f32 * 0.8;
        let height = bounds.height() as f32 * 0.8;

        context.set_color(color);

        // Draw palette shape (oval with thumb hole)
        let palette_center = Point::new(center.x, center.y);
        context.fill_ellipse(
            palette_center,
            (width * 0.5) as i32,
            (height * 0.4) as i32,
            32,
        );

        // Draw thumb hole. Same fix as draw_gear: alpha-zero never
        // erases pixels, so paint the hole in the surface color.
        context.set_color(bg_color);
        let hole_x = (center.x as f32 - width * 0.2) as i32;
        let hole_y = (center.y as f32 - height * 0.1) as i32;
        context.fill_ellipse(
            Point::new(hole_x, hole_y),
            (width * 0.15) as i32,
            (height * 0.15) as i32,
            16,
        );
    }

    pub fn draw_info(context: &mut dyn DrawContext, position: Point, size: i32) {
        let center_x = position.x + size / 2;
        let center_y = position.y + size / 2;

        // Draw circle
        context.draw_circle(Point::new(center_x, center_y), size / 2 - 1, 32);

        // Draw 'i'
        let dot_size = 2;
        context.fill_circle(Point::new(center_x, center_y - size / 4), dot_size, 16);

        context.draw_line(
            Point::new(center_x, center_y - size / 8),
            Point::new(center_x, center_y + size / 4),
            2,
        );
    }

    pub fn draw_check(context: &mut dyn DrawContext, position: Point, size: i32) {
        let center_x = position.x + size / 2;
        let center_y = position.y + size / 2;

        // Draw checkmark
        context.draw_line(
            Point::new(center_x - size / 3, center_y),
            Point::new(center_x - size / 8, center_y + size / 4),
            2,
        );
        context.draw_line(
            Point::new(center_x - size / 8, center_y + size / 4),
            Point::new(center_x + size / 3, center_y - size / 3),
            2,
        );
    }

    pub fn draw_warning(context: &mut dyn DrawContext, position: Point, size: i32) {
        let center_x = position.x + size / 2;
        let center_y = position.y + size / 2;

        // Draw triangle
        let top = Point::new(center_x, position.y + 2);
        let left = Point::new(position.x + 2, position.y + size - 2);
        let right = Point::new(position.x + size - 2, position.y + size - 2);

        context.draw_line(top, left, 2);
        context.draw_line(left, right, 2);
        context.draw_line(right, top, 2);

        // Draw exclamation mark
        context.draw_line(
            Point::new(center_x, center_y - size / 6),
            Point::new(center_x, center_y + size / 8),
            2,
        );

        context.fill_circle(Point::new(center_x, center_y + size / 4), 2, 16);
    }

    pub fn draw_error(context: &mut dyn DrawContext, position: Point, size: i32) {
        let center_x = position.x + size / 2;
        let center_y = position.y + size / 2;

        // Draw circle
        context.draw_circle(Point::new(center_x, center_y), size / 2 - 1, 32);

        // Draw X
        let offset = size / 3;
        context.draw_line(
            Point::new(center_x - offset, center_y - offset),
            Point::new(center_x + offset, center_y + offset),
            2,
        );
        context.draw_line(
            Point::new(center_x + offset, center_y - offset),
            Point::new(center_x - offset, center_y + offset),
            2,
        );
    }

    pub fn draw_close(context: &mut dyn DrawContext, position: Point, size: i32) {
        // Draw X
        let half = size / 2;
        context.draw_line(
            Point::new(position.x - half, position.y - half),
            Point::new(position.x + half, position.y + half),
            2,
        );
        context.draw_line(
            Point::new(position.x + half, position.y - half),
            Point::new(position.x - half, position.y + half),
            2,
        );
    }
}
