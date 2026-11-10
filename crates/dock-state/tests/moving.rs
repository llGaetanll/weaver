//! Taking a window somewhere else: picking it up by its tab, being told
//! where it would land, and putting it down.

mod fixture;

use dock_state::Event;
use dock_state::EventHost::Opened;
use dock_state::EventPointer::Down;
use dock_state::EventPointer::Moved;
use dock_state::EventPointer::Up;
use dock_state::Point;
use dock_state::Rect;
use dock_state::Share;
use dock_state::Side;
use dock_state::Slot;
use eval::eval;
use fixture::A;
use fixture::B;
use fixture::C;
use fixture::near;
use fixture::rect;
use fixture::three;

/// The whole of it: a window taken out of the row and put against the left
/// of another, which is the move every other test here is a corner of.
#[test]
fn a_window_is_carried_to_a_band_and_put_down() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Up { at: Point::new(420.0, 100.0) }),
    );

    assert!(rect(&dock, A).x < rect(&dock, B).x, "A went to B's left");
    near(rect(&dock, A).h, rect(&dock, B).h, "and shares its height");
    near(rect(&dock, A).y, rect(&dock, B).y, "and its top");
}

/// A press that goes nowhere has brought a window forward and done nothing
/// else, so a tab can be picked without the screen rearranging itself.
#[test]
fn a_press_on_a_tab_is_not_a_move() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(12.0, 11.0) }),
        < Event::FromPointer(Up { at: Point::new(12.0, 11.0) }),
    );

    assert!(dock.frame().carried.is_none(), "two pixels is a twitch");
    near(rect(&dock, A).w, 399.5, "and nothing moved");
}

/// Once a press has travelled, the window hangs under its tab and goes
/// where the tab goes, the size it was.
#[test]
fn a_press_that_travels_lifts_the_window() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
    );

    let frame = dock.frame();
    let carried = frame.carried.expect("now it is a move");
    let window = frame
        .windows
        .iter()
        .find(|drawn| drawn.window == A)
        .expect("A is drawn");

    assert_eq!(carried.rect, Rect::new(190.0, 290.0, 160.0, 27.0), "the tab is where it was held");
    assert_eq!(
        window.rect,
        Rect::new(190.0, 316.0, 399.5, 573.0),
        "and the window hangs under it, its top row under the tab's bottom one",
    );
    assert!(window.showing && window.lifted, "drawn, and in the air");
}

/// The slot a lifted window leaves is kept, empty, so nothing moves out from
/// under the cursor while it is aimed.
#[test]
fn the_slot_a_lifted_window_leaves_is_empty() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
    );

    let frame = dock.frame();

    assert_eq!(frame.panes.len(), 3, "the slot is still there");
    assert!(
        frame
            .panes
            .iter()
            .any(|pane| pane.rect.x == 0.0 && pane.tabs.is_empty()),
        "with nothing in it",
    );
}

/// The region drawn and the drop carried out are one decision: `targets`
/// hands back both, and the frame and the drop each take their half. If a
/// band ever highlights one side and the window lands on another, this is
/// the test that goes red.
#[test]
fn the_band_under_the_pointer_is_the_one_offered() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
    );

    assert_eq!(dock.frame().regions.len(), 5, "four bands and the middle");

    let zone = fixture::chosen(&dock).expect("in B's left band");
    let against = zone.points.iter().filter(|point| point.x < 403.0).count();

    assert_eq!(
        against, 2,
        "a trapezium with its long side on B's left edge"
    );
}

/// A gap is not a target: beside one, the pointer is in a window, and what is
/// offered is that window's regions.
#[test]
fn beside_a_gap_the_window_is_what_is_offered() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(410.0, 310.0) }),
        < Event::FromPointer(Moved { at: Point::new(395.0, 300.0) }),
    );

    assert_eq!(
        dock.frame().regions.len(),
        5,
        "A's five regions, not a strip along the gap"
    );
}

/// Dropping beside a window whose group already runs that way folds into the
/// group rather than nesting, so the arrival takes an equal share of it.
#[test]
fn a_drop_into_a_row_takes_a_third_of_the_row() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(410.0, 310.0) }),
        < Event::FromPointer(Moved { at: Point::new(370.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(370.0, 300.0) }),
    );

    let frame = dock.frame();

    assert_eq!(frame.panes.len(), 3);
    assert_eq!(frame.gaps.len(), 2, "one row, two gaps, no column left");

    near(rect(&dock, C).w, 266.0, "a third of the row");
    near(rect(&dock, C).x, 267.0, "between A and B");
    near(rect(&dock, C).h, 600.0, "and the whole height");
}

#[test]
fn a_pointer_outside_the_dock_proposes_nothing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Moved { at: Point::new(-50.0, 300.0) }),
    );

    assert!(dock.frame().carried.is_some(), "still being carried");
    assert!(dock.frame().regions.is_empty(), "with nowhere to land");
}

#[test]
fn a_drop_outside_the_dock_changes_nothing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(420.0, 100.0) }),
        < Event::FromPointer(Moved { at: Point::new(-50.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(-50.0, 300.0) }),
    );

    near(rect(&dock, A).x, 0.0, "back where it was");
    near(rect(&dock, A).w, 399.5, "back where it was");
}

/// A window alone in its slot has nothing to join and no side of itself to
/// take, so carrying it over its own slot proposes nothing at all.
#[test]
fn a_window_over_its_own_slot_proposes_nothing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
    );

    assert!(dock.frame().carried.is_some(), "still being carried");
    assert!(
        dock.frame().regions.is_empty(),
        "its own slot is not a target"
    );
}

/// Once a lone window has left its slot, the slot is empty, and an empty
/// strip holds nothing to stick to: brought back over it, the tab follows the
/// pointer and the window stays in the air.
#[test]
fn a_window_brought_back_over_its_empty_slot_stays_in_the_air() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(50.0, 20.0) }),
    );

    let frame = dock.frame();
    let carried = frame.carried.expect("still being carried");
    let window = frame
        .windows
        .iter()
        .find(|drawn| drawn.window == A)
        .expect("A is drawn");

    assert_eq!(
        carried.rect,
        Rect::new(40.0, 7.0, 160.0, 27.0),
        "the tab is under the pointer, not stuck to the strip",
    );
    assert!(window.lifted, "and the window is still in the air");
    assert!(frame.regions.is_empty(), "with nowhere to land");
}

/// Let go over the slot it left, a window is back where it was.
#[test]
fn a_window_dropped_on_its_empty_slot_is_back_where_it_was() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(10.0, 13.0) }),
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(50.0, 20.0) }),
        < Event::FromPointer(Up { at: Point::new(50.0, 20.0) }),
    );

    assert!(dock.frame().carried.is_none(), "nothing is carried");
    assert_eq!(rect(&dock, A), Rect::new(0.0, 0.0, 399.5, 600.0), "A is back");
    assert_eq!(dock.frame().panes.len(), 3, "and nothing else changed");
}

/// A fixed window is fixed wherever it is put: the number was somebody's
/// decision and a move is not a reason to forget it.
#[test]
fn a_fixed_window_keeps_its_size_when_it_is_moved() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Fixed(300.0),
        }),
        < Event::FromPointer(Down { at: Point::new(505.0, 10.0) }),
        < Event::FromPointer(Moved { at: Point::new(30.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(30.0, 300.0) }),
    );

    near(rect(&dock, B).x, 0.0, "now on the left");
    near(rect(&dock, B).w, 300.0, "and still 300");
}
