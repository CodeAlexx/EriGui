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
