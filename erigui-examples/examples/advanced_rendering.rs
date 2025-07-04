use erigui_core::{
    Color, DrawContext, Event, EventResult, LayoutConstraints,
    Point, Rect, Size, Theme, Widget, WidgetId, WidgetState,
};
use erigui_rendering::Renderer;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use std::any::Any;

struct AdvancedRenderingDemo {
    state: WidgetState,
    frame: i32,
}

impl AdvancedRenderingDemo {
    fn new(id: WidgetId) -> Self {
        Self {
            state: WidgetState::new(id),
            frame: 0,
        }
    }
}

impl Widget for AdvancedRenderingDemo {
    fn id(&self) -> WidgetId {
        self.state.id
    }
    
    fn measure(&self, _constraints: &LayoutConstraints, _theme: &Theme) -> Size {
        Size::new(800, 600)
    }
    
    fn layout(&mut self, rect: Rect, _theme: &Theme) {
        self.state.bounds = rect;
    }
    
    fn draw(&self, context: &mut dyn DrawContext, theme: &Theme) {
        // Clear background
        context.set_color(theme.colors.background);
        context.fill_rect(self.state.bounds);
        
        // Draw title
        context.set_color(theme.colors.text);
        context.draw_text("Advanced Rendering Features Demo", Point::new(20, 30), 20);
        
        // 1. Rounded rectangles
        context.draw_text("Rounded Rectangles:", Point::new(20, 70), 14);
        
        // Filled rounded rect
        context.set_color(theme.colors.primary);
        context.fill_rounded_rect(Rect::new(20, 90, 100, 60), 10);
        
        // Outlined rounded rect
        context.set_color(theme.colors.error);
        context.draw_rounded_rect(Rect::new(140, 90, 100, 60), 20);
        
        // 2. Gradients
        context.set_color(theme.colors.text);
        context.draw_text("Gradients:", Point::new(20, 180), 14);
        
        // Vertical gradient
        context.draw_gradient_rect(
            Rect::new(20, 200, 100, 60),
            theme.colors.primary,
            theme.colors.info,
            false
        );
        
        // Horizontal gradient
        context.draw_gradient_rect(
            Rect::new(140, 200, 100, 60),
            theme.colors.success,
            theme.colors.warning,
            true
        );
        
        // 3. Shadows
        context.set_color(theme.colors.text);
        context.draw_text("Shadows:", Point::new(20, 290), 14);
        
        // Draw shadow first
        context.draw_shadow(
            Rect::new(20, 310, 100, 60),
            Color::rgba(0, 0, 0, 128),
            10,
            Point::new(5, 5)
        );
        // Then draw the rect
        context.set_color(theme.colors.surface_variant);
        context.fill_rect(Rect::new(20, 310, 100, 60));
        
        // Colored shadow
        context.draw_shadow(
            Rect::new(140, 310, 100, 60),
            theme.colors.primary.with_alpha(100),
            15,
            Point::new(8, 8)
        );
        context.set_color(theme.colors.surface_variant);
        context.fill_rounded_rect(Rect::new(140, 310, 100, 60), 8);
        
        // 4. Circles and Ellipses
        context.set_color(theme.colors.text);
        context.draw_text("Circles & Ellipses:", Point::new(300, 70), 14);
        
        // Filled circle
        context.set_color(theme.colors.primary);
        context.fill_circle(Point::new(350, 120), 30, 32);
        
        // Outlined circle
        context.set_color(theme.colors.error);
        context.draw_circle(Point::new(450, 120), 30, 32);
        
        // Filled ellipse
        context.set_color(theme.colors.success);
        context.fill_ellipse(Point::new(350, 230), 40, 25, 32);
        
        // Outlined ellipse
        context.set_color(theme.colors.info);
        context.draw_ellipse(Point::new(450, 230), 25, 40, 32);
        
        // 5. Combined effects
        context.set_color(theme.colors.text);
        context.draw_text("Combined Effects:", Point::new(300, 290), 14);
        
        // Shadow + gradient + rounded rect
        let combined_rect = Rect::new(300, 310, 200, 60);
        
        // Draw shadow
        context.draw_shadow(
            combined_rect,
            Color::rgba(0, 0, 0, 100),
            12,
            Point::new(6, 6)
        );
        
        // Draw gradient with rounded corners
        // First clip to rounded rect shape
        context.push_clip_rect(combined_rect);
        context.draw_gradient_rect(
            combined_rect,
            theme.colors.primary,
            theme.colors.primary_active,
            false
        );
        context.pop_clip_rect();
        
        // Draw border
        context.set_color(theme.colors.border);
        context.draw_rounded_rect(combined_rect, 15);
        
        // 6. Animated element
        let anim_x = 520 + ((self.frame as f32 * 0.05).sin() * 30.0) as i32;
        let anim_radius = 20 + ((self.frame as f32 * 0.03).cos() * 10.0) as i32;
        
        context.set_color(theme.colors.text);
        context.draw_text("Animated:", Point::new(520, 180), 14);
        
        // Animated shadow
        context.draw_shadow(
            Rect::new(anim_x - anim_radius, 210 - anim_radius, anim_radius * 2, anim_radius * 2),
            theme.colors.primary.with_alpha(80),
            8,
            Point::new(4, 4)
        );
        
        // Animated circle
        context.set_color(theme.colors.primary);
        context.fill_circle(Point::new(anim_x, 210), anim_radius, 32);
    }
    
    fn handle_event(&mut self, _event: &Event, _theme: &Theme) -> EventResult {
        self.frame += 1;
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
        false
    }
    
    fn set_focused(&mut self, _focused: bool) {}
    
    fn can_focus(&self) -> bool {
        false
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() -> erigui_core::Result<()> {
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop, 800, 600, "Advanced Rendering Demo")?;
    
    let theme = Theme::light();
    let mut demo = AdvancedRenderingDemo::new(WidgetId::default());
    demo.layout(Rect::new(0, 0, 800, 600), &theme);
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        
        match event {
            WinitEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size.width, physical_size.height);
                    let size = Size::new(physical_size.width as i32, physical_size.height as i32);
                    demo.layout(Rect::new(0, 0, size.width, size.height), &theme);
                }
                _ => {}
            },
            WinitEvent::MainEventsCleared => {
                renderer.window().request_redraw();
            }
            WinitEvent::RedrawRequested(_) => {
                renderer.begin_frame(theme.colors.background);
                demo.draw(&mut renderer, &theme);
                renderer.end_frame();
                
                // Trigger animation update
                demo.handle_event(&Event::Update, &theme);
            }
            _ => {}
        }
    });
}