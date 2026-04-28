use thiserror::Error;

#[derive(Error, Debug)]
pub enum EriGuiError {
    #[error("OpenGL initialization failed: {0}")]
    OpenGLInit(String),

    #[error("Window creation failed: {0}")]
    WindowCreation(String),

    #[error("Font loading failed: {0}")]
    FontLoading(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    #[error("Invalid widget ID")]
    InvalidWidgetId,

    #[error("Widget not found: {0:?}")]
    WidgetNotFound(crate::WidgetId),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Rendering error: {0}")]
    Rendering(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EriGuiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opengl_init_display_includes_message() {
        let err = EriGuiError::OpenGLInit("egl missing".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("OpenGL initialization failed"));
        assert!(msg.contains("egl missing"));
    }

    #[test]
    fn window_creation_display_includes_message() {
        let err = EriGuiError::WindowCreation("no display".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Window creation failed"));
        assert!(msg.contains("no display"));
    }

    #[test]
    fn font_loading_display_includes_message() {
        let err = EriGuiError::FontLoading("bad ttf".into());
        assert!(format!("{}", err).contains("bad ttf"));
    }

    #[test]
    fn shader_compilation_display_includes_message() {
        let err = EriGuiError::ShaderCompilation("syntax error".into());
        assert!(format!("{}", err).contains("Shader compilation failed"));
    }

    #[test]
    fn invalid_widget_id_has_static_message() {
        let err = EriGuiError::InvalidWidgetId;
        assert_eq!(format!("{}", err), "Invalid widget ID");
    }

    #[test]
    fn layout_display_includes_message() {
        let err = EriGuiError::Layout("constraints unsatisfiable".into());
        assert!(format!("{}", err).contains("constraints unsatisfiable"));
    }

    #[test]
    fn rendering_display_includes_message() {
        let err = EriGuiError::Rendering("draw failed".into());
        assert!(format!("{}", err).contains("draw failed"));
    }

    #[test]
    fn io_error_converts_via_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: EriGuiError = io_err.into();
        match err {
            EriGuiError::Io(_) => {}
            _ => panic!("expected Io variant"),
        }
    }

    #[test]
    fn result_type_alias_is_usable() {
        fn ok_fn() -> Result<u32> {
            Ok(42)
        }
        fn err_fn() -> Result<u32> {
            Err(EriGuiError::InvalidWidgetId)
        }
        assert_eq!(ok_fn().unwrap(), 42);
        assert!(err_fn().is_err());
    }
}
