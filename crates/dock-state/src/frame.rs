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

/// A region a drop can be aimed at, as the four corners of a quadrilateral.
///
/// A band in a window is a trapezium — the wedge between two of its diagonals
/// with the middle taken out — and everything else here is a rectangle, so
/// four points covers all of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    pub points: [Point; 4],
    /// The one the pointer is in. Every region on offer is handed over, so
    /// what the window is divided into can be seen rather than guessed, and
    /// this is the one that would happen.
    pub chosen: bool,
}

impl Zone {
    pub fn rect(rect: Rect, chosen: bool) -> Self {
        Self {
            points: [
                Point::new(rect.x, rect.y),
                Point::new(rect.right(), rect.y),
                Point::new(rect.right(), rect.bottom()),
                Point::new(rect.x, rect.bottom()),
            ],
            chosen,
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
    /// Every region on offer where the pointer is: the five a window is
    /// divided into, or the single one a gap or the dock's edge makes. They
    /// are the shapes you have to be inside rather than the slots a window
    /// would get, and at most one of them is `chosen`. None of them chosen
    /// means letting go here would do nothing.
    pub regions: Vec<Zone>,
    pub cursor: Cursor,
    /// The windows may move to these rectangles rather than appear at them.
    /// False while something is happening under the pointer, where anything
    /// but keeping up reads as lag.
    pub settling: bool,
}
