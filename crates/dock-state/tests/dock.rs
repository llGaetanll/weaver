//! The conversations a dock can have.
//!
//! Every turn is its own `eval!`: the dock has no actions yet, so a turn with
//! no `>` line is the assertion that it accepted the events and deliberately
//! did nothing.

use dock_state::Event;
use dock_state::EventHost;
use dock_state::EventPointer;
use dock_state::Point;
use dock_state::Rect;
use dock_state::Share;
use dock_state::Side;
use dock_state::Size;
use dock_state::Slot;
use dock_state::StateDock;
use dock_state::WindowId;
use dock_state::Zone;
use util_eval::eval;

const A: WindowId = WindowId(1);
const B: WindowId = WindowId(2);
const C: WindowId = WindowId(3);

/// 800 by 600, and the metrics the library ships with: a 5px gap, so two
/// windows side by side are 397.5 each.
fn dock() -> StateDock {
    let mut dock = StateDock::default();

    eval!(&mut dock,
        < resized(800.0, 600.0),
        < opened(A, Slot::Edge { side: Side::Right }, Share::Flex(1.0)),
    );

    dock
}

/// `A | B over C`: a row of two, the second of which is a column.
fn three() -> StateDock {
    let mut dock = dock();

    eval!(&mut dock,
        < opened(B, beside(A, Side::Right), Share::Flex(1.0)),
        < opened(C, beside(B, Side::Down), Share::Flex(1.0)),
    );

    dock
}

fn resized(w: f32, h: f32) -> Event {
    Event::FromHost(EventHost::Resized {
        size: Size::new(w, h),
    })
}

fn opened(window: WindowId, at: Slot, share: Share) -> Event {
    Event::FromHost(EventHost::Opened {
        window,
        title: format!("window {}", window.0),
        at,
        share,
    })
}

fn closed(window: WindowId) -> Event {
    Event::FromHost(EventHost::Closed { window })
}

fn beside(window: WindowId, side: Side) -> Slot {
    Slot::Beside { window, side }
}

fn down(x: f32, y: f32) -> Event {
    Event::FromPointer(EventPointer::Down {
        at: Point::new(x, y),
    })
}

fn moved(x: f32, y: f32) -> Event {
    Event::FromPointer(EventPointer::Moved {
        at: Point::new(x, y),
    })
}

fn up(x: f32, y: f32) -> Event {
    Event::FromPointer(EventPointer::Up {
        at: Point::new(x, y),
    })
}

fn cancelled() -> Event {
    Event::FromPointer(EventPointer::Cancelled)
}

/// The slot a window is in, tabs and all. What the window itself gets is
/// this less the tabs, which is not what a test about layout is asking.
fn rect(dock: &StateDock, window: WindowId) -> Rect {
    dock.frame()
        .panes
        .iter()
        .find(|pane| pane.tabs.iter().any(|tab| tab.window == window))
        .map(|pane| pane.rect)
        .expect("the window is in the dock")
}

/// The tabs of the slot a window is in, in the order they are drawn.
fn tabs(dock: &StateDock, window: WindowId) -> Vec<WindowId> {
    dock.frame()
        .panes
        .iter()
        .find(|pane| pane.tabs.iter().any(|tab| tab.window == window))
        .map(|pane| pane.tabs.iter().map(|tab| tab.window).collect())
        .unwrap_or_default()
}

fn showing(dock: &StateDock, window: WindowId) -> bool {
    dock.frame()
        .windows
        .iter()
        .any(|drawn| drawn.window == window && drawn.showing)
}

fn chosen(dock: &StateDock) -> Option<Zone> {
    dock.frame()
        .regions
        .into_iter()
        .find(|region| region.chosen)
}

#[track_caller]
fn near(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 0.5, "{what}: {a} is not {b}");
}

#[test]
fn opening_windows_divides_the_surface() {
    let dock = three();
    let frame = dock.frame();

    assert_eq!(frame.windows.len(), 3);
    assert_eq!(
        frame.gaps.len(),
        2,
        "one between the columns, one in the second"
    );

    near(rect(&dock, A).w, 397.5, "a half, less half a gap");
    near(rect(&dock, B).h, 297.5, "a half of the column");
    near(rect(&dock, C).y, 302.5, "under B and its gap");
}

#[test]
fn a_press_on_a_tab_is_not_a_move() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < up(10.0, 10.0));

    assert!(dock.frame().ghost.is_none());
    near(rect(&dock, A).w, 397.5, "nothing moved");
}

#[test]
fn a_press_that_travels_lifts_the_window() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));

    assert!(dock.frame().ghost.is_none(), "not yet past the threshold");

    eval!(&mut dock, < moved(12.0, 11.0));

    assert!(dock.frame().ghost.is_none(), "two pixels is a twitch");

    eval!(&mut dock, < moved(200.0, 300.0));

    let frame = dock.frame();

    assert!(frame.ghost.is_some(), "now it is a move");
    assert!(
        frame
            .windows
            .iter()
            .any(|drawn| drawn.window == A && drawn.lifted),
        "the window stays where it is, dimmed",
    );
}

/// The region drawn and the drop carried out are one decision: `slot` hands
/// back both, and the frame and the drop each take their half. If a band ever
/// highlights one side and the window lands on another, this is the test that
/// goes red.
#[test]
fn a_drop_lands_on_the_side_its_band_was_on() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(420.0, 100.0));

    let frame = dock.frame();

    assert_eq!(
        frame.regions.len(),
        5,
        "four bands and the middle, all drawn"
    );
    assert_eq!(
        frame.regions.iter().filter(|region| region.chosen).count(),
        1,
        "and exactly one of them under the pointer",
    );

    let zone = chosen(&dock).expect("in B's left band");
    let against = zone.points.iter().filter(|point| point.x < 403.0).count();

    assert_eq!(
        against, 2,
        "a trapezium with its long side on B's left edge"
    );

    eval!(&mut dock, < up(420.0, 100.0));

    assert!(rect(&dock, A).x < rect(&dock, B).x, "A went to B's left");
    near(rect(&dock, A).h, rect(&dock, B).h, "and shares its height");
    near(rect(&dock, A).y, rect(&dock, B).y, "and its top");
}

/// The middle of a window is its stack: a window dropped there shares the
/// slot rather than taking a side of it.
#[test]
fn the_middle_of_a_window_is_its_stack() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(600.0, 150.0));

    assert_eq!(dock.frame().regions.len(), 5, "four bands and the middle");
    assert!(
        chosen(&dock).is_some(),
        "and the middle is one of the answers"
    );

    eval!(&mut dock, < up(600.0, 150.0));

    assert_eq!(dock.frame().panes.len(), 2, "three windows in two slots");
    assert_eq!(
        tabs(&dock, A),
        vec![B, A],
        "A joined B, behind it in the order"
    );
    assert!(
        showing(&dock, A),
        "and it is the one showing, since it was just put there"
    );
    assert!(!showing(&dock, B), "which means B is not");
}

/// A window alone in its slot has nothing to join and no side of itself to
/// take, so carrying it over its own slot proposes nothing at all.
#[test]
fn a_window_over_its_own_slot_proposes_nothing() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(200.0, 300.0));

    assert!(dock.frame().ghost.is_some(), "still being carried");
    assert!(
        dock.frame().regions.is_empty(),
        "but its own slot is not a target"
    );
}

/// A tab is picked by pressing it, and a window behind one keeps its place.
#[test]
fn pressing_a_tab_brings_its_window_forward() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(600.0, 150.0));
    eval!(&mut dock, < up(600.0, 150.0));

    assert!(showing(&dock, A), "A is in front");

    // B's own tab is the first of the two.
    eval!(&mut dock, < down(80.0, 13.0));

    assert!(showing(&dock, B), "and now B is");
    assert_eq!(
        tabs(&dock, A),
        vec![B, A],
        "with neither of them having moved"
    );
}

/// Leaving a stack is an ordinary move: the slot stays, holding what is left.
#[test]
fn a_window_can_be_taken_back_out_of_a_stack() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(600.0, 150.0));
    eval!(&mut dock, < up(600.0, 150.0));

    assert_eq!(dock.frame().panes.len(), 2);

    // A's tab is the second of the two; drag it onto the top band of the slot
    // below.
    eval!(&mut dock, < down(168.0, 13.0));
    eval!(&mut dock, < moved(400.0, 320.0));
    eval!(&mut dock, < up(400.0, 320.0));

    assert_eq!(dock.frame().panes.len(), 3, "three slots again");
    assert_eq!(tabs(&dock, B), vec![B], "and B is alone in its own");
    assert!(showing(&dock, A), "A is showing wherever it landed");
}

/// Dropping beside a window whose group already runs that way folds into the
/// group rather than nesting, so the arrival takes an equal share of it.
#[test]
fn a_drop_into_a_row_takes_a_third_of_the_row() {
    let mut dock = three();

    eval!(&mut dock, < down(410.0, 310.0));
    eval!(&mut dock, < moved(370.0, 300.0));
    eval!(&mut dock, < up(370.0, 300.0));

    let frame = dock.frame();

    assert_eq!(frame.windows.len(), 3);
    assert_eq!(frame.gaps.len(), 2, "one row, two gaps, no column left");

    near(rect(&dock, C).w, 263.3, "a third of the row");
    near(rect(&dock, C).x, 268.3, "between A and B");
    near(rect(&dock, C).h, 600.0, "the whole height");
}

#[test]
fn a_drop_outside_the_dock_changes_nothing() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(420.0, 100.0));
    eval!(&mut dock, < moved(-50.0, 300.0));

    assert!(dock.frame().regions.is_empty(), "nowhere to land");

    eval!(&mut dock, < up(-50.0, 300.0));

    near(rect(&dock, A).w, 397.5, "back where it was");
    near(rect(&dock, A).x, 0.0, "back where it was");
}

#[test]
fn dragging_a_gap_moves_weight_between_two_windows() {
    let mut dock = dock();

    eval!(&mut dock, < opened(B, beside(A, Side::Right), Share::Flex(1.0)));
    eval!(&mut dock, < down(400.0, 300.0));
    eval!(&mut dock, < moved(500.0, 300.0));
    eval!(&mut dock, < up(500.0, 300.0));

    near(rect(&dock, A).w, 497.5, "a hundred pixels wider");
    near(rect(&dock, B).w, 297.5, "and the other a hundred narrower");
}

#[test]
fn a_resize_stops_at_the_minimum() {
    let mut dock = dock();

    eval!(&mut dock, < opened(B, beside(A, Side::Right), Share::Flex(1.0)));
    eval!(&mut dock, < down(400.0, 300.0));
    eval!(&mut dock, < moved(2000.0, 300.0));

    near(rect(&dock, B).w, 120.0, "the minimum, not nothing");
}

#[test]
fn escape_puts_a_resize_back() {
    let mut dock = dock();

    eval!(&mut dock, < opened(B, beside(A, Side::Right), Share::Flex(1.0)));
    eval!(&mut dock, < down(400.0, 300.0));
    eval!(&mut dock, < moved(500.0, 300.0));
    eval!(&mut dock, < cancelled());

    near(rect(&dock, A).w, 397.5, "half again");
}

#[test]
fn closing_a_window_dissolves_its_group() {
    let mut dock = three();

    eval!(&mut dock, < closed(C));

    let frame = dock.frame();

    assert_eq!(frame.windows.len(), 2);
    assert_eq!(
        frame.gaps.len(),
        1,
        "the column is gone, not left holding one"
    );

    near(rect(&dock, B).h, 600.0, "B has the whole height back");
}

/// A fixed window is fixed wherever it is put: the number was somebody's
/// decision and a move is not a reason to forget it.
#[test]
fn a_fixed_window_keeps_its_size_when_it_is_moved() {
    let mut dock = dock();

    eval!(&mut dock, < opened(B, beside(A, Side::Right), Share::Fixed(300.0)));

    near(rect(&dock, B).w, 300.0, "as asked");
    near(rect(&dock, A).w, 495.0, "and A takes the rest");

    eval!(&mut dock, < down(505.0, 10.0));
    eval!(&mut dock, < moved(30.0, 300.0));
    eval!(&mut dock, < up(30.0, 300.0));

    near(rect(&dock, B).x, 0.0, "now on the left");
    near(rect(&dock, B).w, 300.0, "and still 300");
}

/// Whether the windows should move to where they now belong or simply be
/// there. A drop settles; a gap being dragged and the browser being resized
/// are continuous, and a window that eased into place behind either would
/// read as lag.
#[test]
fn a_drop_settles_and_a_drag_does_not() {
    let mut dock = dock();

    eval!(&mut dock, < opened(B, beside(A, Side::Right), Share::Flex(1.0)));
    eval!(&mut dock, < down(400.0, 300.0));
    eval!(&mut dock, < moved(500.0, 300.0));

    assert!(!dock.frame().settling, "the gap is under the pointer");

    eval!(&mut dock, < up(500.0, 300.0));

    assert!(!dock.frame().settling, "and letting go of it moved nothing");

    eval!(&mut dock, < resized(900.0, 600.0));

    assert!(
        !dock.frame().settling,
        "the browser's edge is under one too"
    );
}

/// Only what changes the layout says how the layout came about. A pointer
/// moving across a dock that has just been dropped into must not cut the
/// settling short — which is exactly what happens if every event answers the
/// question.
#[test]
fn a_pointer_moving_afterwards_does_not_cut_a_settle_short() {
    let mut dock = three();

    eval!(&mut dock, < down(10.0, 10.0));
    eval!(&mut dock, < moved(420.0, 100.0));
    eval!(&mut dock, < up(420.0, 100.0));

    assert!(dock.frame().settling, "dropped");

    eval!(&mut dock, < moved(600.0, 400.0));

    assert!(dock.frame().settling, "and still settling into it");
}
