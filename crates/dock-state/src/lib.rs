//! A tiled dock: windows on a surface, moved by their grips and resized by
//! the gaps between them.
//!
//! The library decides where rectangles go and draws a window's title bar,
//! the gaps, and the two things that exist only while a window is being
//! moved. What is inside a window belongs to whoever is using it.
//!
//! It is a state machine. Events go in with [`StateDock::handle_events`],
//! actions come back from [`StateDock::emit_actions`] once the whole batch has
//! landed, and everything drawn is derived on the way out by
//! [`StateDock::frame`]. Nothing here reads a clock, and nothing here knows
//! what a browser is. See `CLAUDE.md`.

#![forbid(unsafe_code)]

mod action;
mod event;
mod event_host;
mod event_pointer;
mod frame;
mod geometry;
mod metrics;
mod solve;
mod state_dock;
mod tree;

pub use action::Action;
pub use event::Event;
pub use event_host::EventHost;
pub use event_pointer::EventPointer;
pub use frame::Cursor;
pub use frame::Frame;
pub use frame::GapFrame;
pub use frame::Zone;
pub use state_dock::REGIONS;
pub use frame::WindowFrame;
pub use geometry::Axis;
pub use geometry::Point;
pub use geometry::Rect;
pub use geometry::Side;
pub use geometry::Size;
pub use metrics::Metrics;
pub use state_dock::StateDock;
pub use tree::Share;
pub use tree::Slot;
pub use tree::WindowId;
