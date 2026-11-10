//! Windows sharing a slot: joining one, leafing through it, changing the
//! order, and leaving.

mod fixture;

use dock_state::Event;
use dock_state::EventPointer::Down;
use dock_state::EventPointer::Moved;
use dock_state::EventPointer::Up;
use dock_state::Point;
use dock_state::Rect;
use eval::eval;
use fixture::A;
use fixture::B;
use fixture::C;
use fixture::showing;
use fixture::tabs;
use fixture::three;
use fixture::window;

/// The middle of a window is its stack: a window dropped there shares the
/// slot rather than taking a side of it.
#[test]
fn the_middle_of_a_window_is_its_stack() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
    );

    assert_eq!(dock.frame().panes.len(), 2, "three windows in two slots");
    assert_eq!(tabs(&dock, A), vec![B, A], "A joined B, behind it in order");
    assert!(
        showing(&dock, A),
        "and it is the one showing, having arrived"
    );
    assert!(!showing(&dock, B), "which means B is not");
}

#[test]
fn the_middle_of_a_window_is_offered_as_a_region() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
    );

    assert_eq!(dock.frame().regions.len(), 5, "four bands and the middle");
    assert!(
        fixture::chosen(&dock).is_some(),
        "the middle is an answer too"
    );
}

/// A tab is picked by pressing it, and a window behind one keeps its place.
#[test]
fn pressing_a_tab_brings_its_window_forward() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(80.0, 13.0) }),
    );

    assert!(showing(&dock, B), "B's own tab is the first of the two");
    assert_eq!(
        tabs(&dock, A),
        vec![B, A],
        "with neither of them having moved"
    );
}

/// Leaving a stack is an ordinary move: the slot stays, holding what is
/// left. The pointer goes past the tabs, which are where the order is
/// decided rather than where a window lands.
#[test]
fn a_window_can_be_taken_back_out_of_a_stack() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(168.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(400.0, 340.0) }),
        < Event::FromPointer(Up { at: Point::new(400.0, 340.0) }),
    );

    assert_eq!(dock.frame().panes.len(), 3, "three slots again");
    assert_eq!(tabs(&dock, B), vec![B], "and B is alone in its own");
    assert!(showing(&dock, A), "A is showing wherever it landed");
}

/// A tab carried over its own strip changes the order rather than landing in
/// the window under it. It is the only thing a stack can be rearranged with,
/// and it is why the strip is not a drop target for anything else.
#[test]
fn a_tab_dragged_along_its_strip_changes_the_order() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(168.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(40.0, 13.0) }),
        < Event::FromPointer(Up { at: Point::new(40.0, 13.0) }),
    );

    assert_eq!(tabs(&dock, A), vec![A, B], "now in front of B");
    assert_eq!(dock.frame().panes.len(), 2, "without having left the slot");
}

/// A tab pushed to the end of a strip reaches over the gap there, so the
/// line down its side is the gap's line rather than a second one beside it.
#[test]
fn a_tab_pushed_to_the_end_of_a_strip_covers_the_gap() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(420.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(398.0, 13.0) }),
    );

    let carried = dock.frame().carried.expect("a tab is being carried");
    assert_eq!(
        carried.rect,
        Rect::new(240.5, 0.0, 160.0, 27.0),
        "its right edge is past the gap"
    );
}

/// The same at the start of a strip, and along its top.
#[test]
fn a_tab_pushed_to_the_start_of_a_strip_covers_the_gap() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(401.0, 310.0) }),
    );

    let carried = dock.frame().carried.expect("a tab is being carried");
    assert_eq!(
        carried.rect,
        Rect::new(399.5, 299.5, 160.0, 27.0),
        "it starts on the gap"
    );
}

/// The dock's own edge has no gap to cover, so a tab stops at it.
#[test]
fn a_tab_pushed_to_the_dock_edge_stops_there() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(420.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(2.0, 13.0) }),
    );

    let carried = dock.frame().carried.expect("a tab is being carried");
    assert_eq!(
        carried.rect,
        Rect::new(0.0, 0.0, 160.0, 27.0),
        "inside the dock"
    );
}

/// A window lifted out of a stack leaves the rest behind, and the tab that
/// slides into its place shows — the one that will show once it lands.
#[test]
fn a_window_lifted_out_of_a_stack_leaves_the_next_one_showing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(200.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(100.0, 400.0) }),
    );

    assert_eq!(tabs(&dock, B), vec![B], "A's tab has left the strip");
    assert!(showing(&dock, B), "B shows in its place");
    assert_eq!(
        window(&dock, A).rect,
        Rect::new(60.0, 413.0, 800.0, 272.5),
        "and A hangs under its tab",
    );
}

/// Putting tabs in order is not moving a window, so the window stays in its
/// slot while its tab is carried along its own strip.
#[test]
fn a_window_stays_while_its_tab_is_put_in_order() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(200.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(20.0, 13.0) }),
    );

    assert_eq!(
        window(&dock, A).rect,
        Rect::new(0.0, 27.0, 800.0, 272.5),
        "A is where it was",
    );
    assert!(showing(&dock, A), "and still the one showing");
    assert!(!showing(&dock, B), "with B behind it");
    assert!(!window(&dock, A).lifted, "it is not in the air");
}

/// A tab whose window is in the air has a line along its top, so over
/// another slot's strip it reaches over the gap above, the way it does at
/// the ends, and the two lines are one.
#[test]
fn a_tab_carried_over_another_strip_covers_the_gap_above() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 310.0) }),
    );

    let carried = dock.frame().carried.expect("a tab is being carried");
    assert_eq!(
        carried.rect,
        Rect::new(590.0, 299.5, 160.0, 27.0),
        "its top is on the gap"
    );
    assert_eq!(
        window(&dock, A).rect,
        Rect::new(590.0, 325.5, 399.5, 573.0),
        "and its window follows",
    );
}

/// A title bar has no order to be put in, so it does not slide along its
/// own strip: it stays where it is until the pointer leaves the strip.
#[test]
fn a_title_bar_does_not_slide_along_its_own_strip() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 13.0) }),
    );

    let carried = dock.frame().carried.expect("a tab is being carried");
    assert_eq!(carried.rect, Rect::new(0.0, 0.0, 160.0, 27.0), "it has not moved");
    assert!(!window(&dock, A).lifted, "and neither has its window");
}

/// The tabs move apart as the pointer goes, so the hole they leave is where
/// the drop would land. Nothing else is drawn: the hole is the answer.
#[test]
fn the_tabs_move_apart_to_leave_the_hole_a_drop_would_fill() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Down { at: Point::new(168.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(40.0, 13.0) }),
    );

    assert!(dock.frame().regions.is_empty(), "no wash: the hole says it");
    assert_eq!(
        tabs(&dock, B),
        vec![A, B],
        "and the tabs have already moved"
    );
}

/// Dropping a tab on the title bar of a window that is alone joins it: one
/// tab is a title bar rather than an order, and the place along the strip
/// still says which side of it the arrival lands on.
#[test]
fn a_lone_window_takes_a_tab_dropped_on_its_title_bar() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 320.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 310.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 310.0) }),
    );

    assert_eq!(tabs(&dock, C), vec![C, A], "A joined C, after it in order");
    assert_eq!(dock.frame().panes.len(), 2);
}
