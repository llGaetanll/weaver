//! What to draw. Derived on the way out, every time, from the tree and where
//! the pointer is.

use crate::geometry::Axis;
use crate::geometry::Point;
use crate::geometry::Rect;
use crate::tree::WindowId;

/// One window's contents: where to draw them, and whether they are the ones
/// showing in their slot.
///
/// There is one of these per window rather than per slot so that a window
/// behind a tab keeps whatever the host has built in it. A hidden window is
/// still here, with the rectangle it will have when its tab is picked.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowFrame {
    pub window: WindowId,
    pub rect: Rect,
    pub showing: bool,
    /// Being moved. Once its tab leaves its own strip it hangs under the tab
    /// and its slot is left empty; the slot is not closed up, which would
    /// move the target out from under the cursor.
    pub lifted: bool,
}

/// One tab: the name of a window sharing a slot, and whatever the host draws
/// beside it. The whole of it is the handle the window is moved by, which is
/// also the whole of what picks it — one target, so there is nothing to aim
/// at inside a tab.
#[derive(Clone, Debug, PartialEq)]
pub struct TabFrame {
    pub window: WindowId,
    pub title: String,
    pub rect: Rect,
    pub showing: bool,
    pub lifted: bool,
}

/// One slot: the box a group of windows shares, and the tabs along the top of
/// it. A slot with one window in it has one tab, which is the title bar.
#[derive(Clone, Debug, PartialEq)]
pub struct PaneFrame {
    /// What to call this slot from one frame to the next. It is not the
    /// window showing in it and not its place on screen — both of those
    /// change while the slot stays the same thing.
    pub key: WindowId,
    pub rect: Rect,
    pub tabs: Vec<TabFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapFrame {
    pub rect: Rect,
    pub axis: Axis,
    /// The slot it comes after. A gap has no name of its own, and whoever
    /// draws one needs to know it is the same gap it drew last time.
    pub after: WindowId,
    /// The pointer is on it, or holding it. Both lines of a crossing are
    /// held together, which an element on its own could never work out.
    pub held: bool,
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
    /// Both at once: the intersection where two gaps meet.
    BothResize,
    Grabbing,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub panes: Vec<PaneFrame>,
    pub windows: Vec<WindowFrame>,
    pub gaps: Vec<GapFrame>,
    /// The tab under the cursor, drawn over everything — the only thing in
    /// the library ever drawn over a window. It is given whole rather than as
    /// a rectangle because what is being carried has a name, and while it is
    /// in the air it is in no slot to read one from.
    pub carried: Option<TabFrame>,
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
