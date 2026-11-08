//! What to draw. Derived on the way out, every time, from the tree and where
//! the pointer is.

use crate::geometry::Axis;
use crate::geometry::Point;
use crate::geometry::Rect;
use crate::tree::WindowId;

#[derive(Clone, Debug, PartialEq)]
pub struct WindowFrame {
    pub window: WindowId,
    pub title: String,
    pub rect: Rect,
    /// Being moved. It stays where it is, dimmed: a dock that closed up the
    /// moment you picked something up would move the target out from under
    /// the cursor.
    pub lifted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapFrame {
    pub rect: Rect,
    pub axis: Axis,
}

/// A region of the surface, as the four corners of a quadrilateral.
///
/// A band in a window is a trapezium — the wedge between two of its diagonals
/// with the middle taken out — and everything else here is a rectangle, so
/// four points covers all of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    pub points: [Point; 4],
}

impl From<Rect> for Zone {
    fn from(rect: Rect) -> Self {
        Self {
            points: [
                Point::new(rect.x, rect.y),
                Point::new(rect.right(), rect.y),
                Point::new(rect.right(), rect.bottom()),
                Point::new(rect.x, rect.bottom()),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    ColResize,
    RowResize,
    Grabbing,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub windows: Vec<WindowFrame>,
    pub gaps: Vec<GapFrame>,
    /// The window under the cursor, drawn over everything. The only thing in
    /// the library ever drawn over a window.
    pub ghost: Option<Rect>,
    /// The region the pointer is in, drawn as the shape you have to be inside
    /// rather than the slot the window would get. It is shown exactly when
    /// letting go would do something, so an empty middle is the window saying
    /// there is no drop here.
    pub preview: Option<Zone>,
    pub cursor: Cursor,
    /// The windows may move to these rectangles rather than appear at them.
    /// False while something is happening under the pointer, where anything
    /// but keeping up reads as lag.
    pub settling: bool,
}
