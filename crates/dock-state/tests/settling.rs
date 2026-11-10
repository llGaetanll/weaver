//! Whether the windows should be moved to where they now belong or simply
//! be there.
//!
//! A drop settles; a gap being dragged and the browser being resized are
//! continuous, and a window that eased into place behind either of those
//! would read as lag rather than as motion.

mod fixture;

use dock_state::Event;
use dock_state::EventHost::Resized;
use dock_state::EventPointer::Cancelled;
use dock_state::EventPointer::Down;
use dock_state::EventPointer::Moved;
use dock_state::EventPointer::Up;
use dock_state::Point;
use dock_state::Rect;
use dock_state::Size;
use eval::eval;
use fixture::A;
use fixture::B;
use fixture::slot;
use fixture::three;
use fixture::window;

#[test]
fn a_gap_under_the_pointer_does_not_settle() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(450.0, 300.0) }),
    );

    assert!(!dock.frame().settling, "the gap is under the pointer");
}

#[test]
fn letting_go_of_a_gap_settles_nothing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(450.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(450.0, 300.0) }),
    );

    assert!(!dock.frame().settling, "letting go of it moved nothing");
}

#[test]
fn the_browser_being_resized_does_not_settle() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromHost(Resized { size: Size::new(900.0, 600.0) }),
    );

    assert!(!dock.frame().settling, "that edge is under a pointer too");
}

#[test]
fn a_drop_settles() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    assert!(dock.frame().settling, "the layout changed on its own");
}

/// Only what changes the layout says how the layout came about. A pointer
/// moving across a dock that has just been dropped into must not cut the
/// settling short — which is exactly what happens if every event answers
/// the question.
#[test]
fn a_pointer_moving_afterwards_does_not_cut_a_settle_short() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 400.0) }),
    );

    assert!(dock.frame().settling, "and still settling into it");
}

/// The window put down is the one the dock settles around. It is named,
/// because how it gets to its slot is not how the others get to theirs.
#[test]
fn the_window_put_down_is_arriving() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    assert!(window(&dock, A).arriving, "A was put down");
    assert!(!window(&dock, B).arriving, "B made room for it");
}

/// Escape puts a window back where it came from, which is arriving too.
#[test]
fn a_window_put_back_is_arriving() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Cancelled),
    );

    assert!(window(&dock, A).arriving, "A went back");
}

/// Whatever changes the layout next is a settle of its own, around nothing
/// in particular.
#[test]
fn the_next_change_forgets_what_arrived() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
        < Event::FromHost(Resized { size: Size::new(900.0, 600.0) }),
    );

    assert!(!window(&dock, A).arriving, "the resize is about everyone");
}

/// What a slot is called is what lets it ease from where it was, so a slot
/// keeps its name when a window joins it.
#[test]
fn a_slot_keeps_its_name_when_a_window_joins_it() {
    let mut dock = three();
    let before = slot(&dock, B);

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(600.0, 150.0) }),
        < Event::FromPointer(Up { at: Point::new(600.0, 150.0) }),
    );

    assert_eq!(slot(&dock, B), before, "B's slot is still B's slot");
    assert_eq!(slot(&dock, A), before, "and A is in it");
}

/// And a window put down beside another is in a slot nobody has seen
/// before, rather than the one it left, which is somewhere else entirely.
#[test]
fn a_window_put_beside_another_has_a_new_slot() {
    let mut dock = three();
    let before = slot(&dock, A);

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    assert_ne!(slot(&dock, A), before, "a new slot");
}

/// A window put down eases from where it was in the air: its tab from where
/// the pointer let go of it, and the slot made for it from around the tab and
/// the window hanging under it.
#[test]
fn a_window_put_down_comes_from_where_it_was_in_the_air() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    let frame = dock.frame();
    let pane = frame
        .panes
        .iter()
        .find(|pane| pane.tabs.iter().any(|tab| tab.window == A))
        .expect("A has a slot");

    assert_eq!(
        pane.tabs[0].from,
        Some(Rect::new(410.0, 90.0, 160.0, 27.0)),
        "its tab, where it was let go of",
    );
    assert_eq!(
        pane.from,
        Some(Rect::new(410.0, 90.0, 399.5, 599.0)),
        "its slot, around the tab and the window under it",
    );
}

/// Nothing else comes from anywhere: every other slot has a rectangle in the
/// old layout to ease from.
#[test]
fn the_slots_making_room_come_from_nowhere_in_particular() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    let frame = dock.frame();
    let others = frame
        .panes
        .iter()
        .filter(|pane| pane.tabs.iter().all(|tab| tab.window != A));

    for pane in others {
        assert_eq!(pane.from, None, "{:?} was already there", pane.key);
        assert!(pane.tabs.iter().all(|tab| tab.from.is_none()), "and so were its tabs");
    }
}

/// Let go of nowhere, a window in the air goes back to its slot rather than
/// appearing there, so that is a settle too.
#[test]
fn a_window_let_go_of_nowhere_settles_back() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromHost(Resized { size: Size::new(800.0, 600.0) }),
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Moved { at: Point::new(-50.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(-50.0, 300.0) }),
    );

    assert!(dock.frame().settling, "it eases back");
}
