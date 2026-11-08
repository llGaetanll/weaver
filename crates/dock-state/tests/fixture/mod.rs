//! What a test starts from, and the questions it asks afterwards.
//!
//! Nothing here builds an event: an event belongs on the `<` line that sends
//! it, where it can be read. What a test may take from here is the dock it
//! begins with and a way to ask the finished frame a question.

#![allow(dead_code)]

use dock_state::Event;
use dock_state::EventHost;
use dock_state::Rect;
use dock_state::Share;
use dock_state::Side;
use dock_state::Size;
use dock_state::Slot;
use dock_state::StateDock;
use dock_state::WindowId;
use dock_state::Zone;

pub const A: WindowId = WindowId(1);
pub const B: WindowId = WindowId(2);
pub const C: WindowId = WindowId(3);

/// 800 by 600 with one window in it, and the metrics the library ships with:
/// a 1px gap, so two windows side by side are 399.5 each.
pub fn one() -> StateDock {
    let mut dock = StateDock::default();

    dock.handle_events(&[
        Event::FromHost(EventHost::Resized {
            size: Size::new(800.0, 600.0),
        }),
        Event::FromHost(EventHost::Opened {
            window: A,
            title: String::from("A"),
            at: Slot::Edge { side: Side::Right },
            share: Share::Flex(1.0),
        }),
    ]);

    dock
}

/// `A | B over C`: a row of two, the second of which is a column.
pub fn three() -> StateDock {
    let mut dock = one();

    dock.handle_events(&[
        Event::FromHost(EventHost::Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside {
                window: A,
                side: Side::Right,
            },
            share: Share::Flex(1.0),
        }),
        Event::FromHost(EventHost::Opened {
            window: C,
            title: String::from("C"),
            at: Slot::Beside {
                window: B,
                side: Side::Down,
            },
            share: Share::Flex(1.0),
        }),
    ]);

    dock
}

/// The slot a window is in, tabs and all. What the window itself gets is
/// this less the tabs, which is not what a test about layout is asking.
pub fn rect(dock: &StateDock, window: WindowId) -> Rect {
    dock.frame()
        .panes
        .iter()
        .find(|pane| pane.tabs.iter().any(|tab| tab.window == window))
        .map(|pane| pane.rect)
        .expect("the window is in the dock")
}

/// The tabs of the slot a window is in, in the order they are drawn.
pub fn tabs(dock: &StateDock, window: WindowId) -> Vec<WindowId> {
    dock.frame()
        .panes
        .iter()
        .find(|pane| pane.tabs.iter().any(|tab| tab.window == window))
        .map(|pane| pane.tabs.iter().map(|tab| tab.window).collect())
        .unwrap_or_default()
}

pub fn showing(dock: &StateDock, window: WindowId) -> bool {
    dock.frame()
        .windows
        .iter()
        .any(|drawn| drawn.window == window && drawn.showing)
}

/// The region the pointer is in, of the ones on offer.
pub fn chosen(dock: &StateDock) -> Option<Zone> {
    dock.frame()
        .regions
        .into_iter()
        .find(|region| region.chosen)
}

#[track_caller]
pub fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 0.5, "{what}: {a} is not {b}");
}
