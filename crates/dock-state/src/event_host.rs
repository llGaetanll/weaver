//! What the program using the dock did.

use crate::geometry::Size;
use crate::tree::Share;
use crate::tree::Slot;
use crate::tree::WindowId;

#[derive(Clone, Debug)]
pub enum EventHost {
    /// The surface the dock has to fill.
    Resized {
        size: Size,
    },

    /// A window to arrange, where to put it, and how much of whatever group
    /// it lands in it takes. The slot is ignored when the dock is empty,
    /// since the first window is the whole of it.
    Opened {
        window: WindowId,
        title: String,
        at: Slot,
        share: Share,
    },

    Closed {
        window: WindowId,
    },

    /// Bring a window to the front of the slot it shares. A host that has
    /// just laid out several windows has an opinion about which of them is
    /// being looked at, and pressing a tab is not something it can do.
    Show {
        window: WindowId,
    },
}
