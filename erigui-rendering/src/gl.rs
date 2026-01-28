// Re-export the generated OpenGL bindings
pub use self::bindings::*;

mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
