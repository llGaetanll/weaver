//! The numbers the dock is drawn with. They live here rather than in a
//! stylesheet because the machine hit tests against them: a title bar that is
//! 27px in CSS and 30px here is a grip you cannot grab along its top edge.

use crate::geometry::Size;

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// A window's title bar, which is also the height of a tab.
    pub tab: f32,
    /// The widest a tab is drawn. Past it a tab is wider than the name in it
    /// needs, so the rest of the strip is left empty rather than one tab
    /// being stretched across a whole slot.
    pub tab_width: f32,
    /// The gap between two windows, which is the resize control.
    pub gap: f32,
    /// How far from the left of a title bar the grip reaches.
    pub grip: f32,
    /// The strip along the dock's edge that drops at the root.
    pub edge: f32,
    /// How much of a window's width and height each drop band takes, so a
    /// tenth leaves four fifths of it in the middle. What is left in the
    /// middle proposes nothing — it is the room tabs will need,
    /// and until they arrive a drop there is a drop that does not happen.
    /// The same fraction on both axes is what makes the four bands meet the
    /// middle exactly along the window's diagonals.
    pub band: f32,
    /// How far a press has to travel before it is a move rather than a click.
    pub threshold: f32,
    /// How small a window may be made.
    pub min: Size,
    /// The largest the thing under the cursor is drawn. A ghost the size of
    /// the window would cover what you are aiming at.
    pub ghost: Size,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            tab: 27.0,
            tab_width: 160.0,
            gap: 5.0,
            grip: 24.0,
            edge: 16.0,
            band: 0.1,
            threshold: 4.0,
            min: Size::new(120.0, 80.0),
            ghost: Size::new(320.0, 180.0),
        }
    }
}
