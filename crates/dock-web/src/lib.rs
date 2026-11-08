//! The dock in a browser: a frame loop, one pointer listener, and elements
//! moved to the rectangles the machine names.
//!
//! Nothing here decides anything. See `dock-state`.

#![forbid(unsafe_code)]

mod runtime_web;
mod view_dock;

pub use runtime_web::Dock;
pub use view_dock::DockView;
