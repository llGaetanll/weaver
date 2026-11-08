//! What to draw. Derived on the way out, every time, from the tree and where
//! the pointer is.

use crate::geometry::Axis;
use crate::geometry::Rect;
use crate::tree::WindowId;

#[derive(Clone, Debug)]
pub struct WindowFrame {
    pub window: WindowId,
    pub title: String,
    pub rect: Rect,
    /// Being moved. It stays where it is, dimmed: a dock that closed up the
    /// moment you picked something up would move the target out from under
    /// the cursor.
    pub lifted: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct GapFrame {
    pub rect: Rect,
    pub axis: Axis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Default,
    ColResize,
    RowResize,
    Grabbing,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub windows: Vec<WindowFrame>,
    pub gaps: Vec<GapFrame>,
    /// The window under the cursor, drawn over everything. The only thing in
    /// the library ever drawn over a window.
    pub ghost: Option<Rect>,
    /// Where it would land. The rectangle is the one the window will actually
    /// have, because it is solved from the tree the drop would produce.
    pub preview: Option<Rect>,
    pub cursor: Cursor,
}
