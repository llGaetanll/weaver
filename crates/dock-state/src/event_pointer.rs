//! What the pointer did, in the dock's own coordinates.
//!
//! Raw positions and nothing else: which grip or gap a press is on is a
//! question about the rectangles, and the rectangles are the machine's. A
//! runtime that answered it would be deciding.

use crate::geometry::Point;

#[derive(Clone, Copy, Debug)]
pub enum EventPointer {
    Down { at: Point },
    Moved { at: Point },
    Up { at: Point },
    /// Escape, or the pointer taken away mid-drag. The gesture ends having
    /// decided nothing.
    Cancelled,
}
