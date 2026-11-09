//! Taking hold of the line between two windows.

mod fixture;

use dock_state::Cursor;
use dock_state::Event;
use dock_state::EventHost::Opened;
use dock_state::EventPointer::Cancelled;
use dock_state::EventPointer::Down;
use dock_state::EventPointer::Moved;
use dock_state::EventPointer::Up;
use dock_state::Point;
use dock_state::Share;
use dock_state::Side;
use dock_state::Slot;
use eval::eval;
use fixture::A;
use fixture::B;
use fixture::near;
use fixture::rect;
use fixture::three;

/// Between two flexible windows a drag moves weight from one to the other.
/// Their total is untouched, so nothing outside the pair moves.
#[test]
fn dragging_a_gap_moves_weight_between_two_windows() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Flex(1.0),
        }),
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(500.0, 300.0) }),
        < Event::FromPointer(Up { at: Point::new(500.0, 300.0) }),
    );

    near(rect(&dock, A).w, 499.5, "a hundred pixels wider");
    near(rect(&dock, B).w, 299.5, "and the other a hundred narrower");
}

/// A drag stops when it reaches a minimum rather than pushing the next
/// window along — which is why a dock can end up larger than the surface
/// instead of crushing a window to a title bar.
#[test]
fn a_resize_stops_at_the_minimum() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Flex(1.0),
        }),
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(2000.0, 300.0) }),
    );

    near(rect(&dock, B).w, 120.0, "the minimum, not nothing");
}

#[test]
fn escape_puts_a_resize_back() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Flex(1.0),
        }),
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(500.0, 300.0) }),
        < Event::FromPointer(Cancelled),
    );

    near(rect(&dock, A).w, 399.5, "half again");
}

/// Where two gaps meet, both are taken hold of: the corner four windows
/// share moves as one thing rather than as two drags done in turn. The press
/// is dead on the crossing, where the column's gap ends and the row's gap
/// runs past — one of them has to be reached from its end.
#[test]
fn the_corner_where_two_gaps_meet_moves_both() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(450.0, 350.0) }),
    );

    near(rect(&dock, A).w, 449.5, "the row's gap went with it");
    near(rect(&dock, B).h, 349.5, "and so did the column's");
}

#[test]
fn escape_puts_both_sides_of_a_corner_back() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Down { at: Point::new(400.0, 300.0) }),
        < Event::FromPointer(Moved { at: Point::new(450.0, 350.0) }),
        < Event::FromPointer(Cancelled),
    );

    near(rect(&dock, A).w, 399.5, "the row's gap is back");
    near(rect(&dock, B).h, 299.5, "and so is the column's");
}

/// The cursor is the machine's answer while the pointer is only hovering,
/// too: two gaps overlap where they cross, and an element that knows it is a
/// vertical line cannot know the horizontal one is there as well.
#[test]
fn hovering_a_gap_says_which_way_it_moves() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(400.0, 100.0) }),
    );

    assert_eq!(dock.frame().cursor, Cursor::ColResize, "a vertical line");
}

#[test]
fn hovering_a_crossing_says_it_moves_both_ways() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(400.0, 300.0) }),
    );

    assert_eq!(dock.frame().cursor, Cursor::BothResize, "both at once");
}

#[test]
fn hovering_a_window_says_nothing() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
    );

    assert_eq!(dock.frame().cursor, Cursor::Default, "nothing to take hold of");
}

/// Near the crossing is near enough: the corner four windows share is a
/// thing somebody aims at, and a square as wide as the lines are grabbable
/// is a small thing to aim at.
#[test]
fn hovering_near_a_crossing_still_says_both() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(400.0, 293.0) }),
    );

    assert_eq!(dock.frame().cursor, Cursor::BothResize, "seven above it");
}

#[test]
fn a_line_far_from_a_crossing_is_only_itself() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(400.0, 200.0) }),
    );

    assert_eq!(dock.frame().cursor, Cursor::ColResize, "a hundred above it");
}

/// Both lines of a crossing are held together, so both of them can be drawn
/// as taken hold of.
#[test]
fn a_crossing_holds_both_of_its_lines() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(400.0, 300.0) }),
    );

    let held = dock.frame().gaps.iter().filter(|gap| gap.held).count();

    assert_eq!(held, 2, "the row's line and the column's");
}

#[test]
fn a_pointer_on_no_line_holds_none_of_them() {
    let mut dock = three();

    eval!(&mut dock,
        < Event::FromPointer(Moved { at: Point::new(200.0, 300.0) }),
    );

    assert!(dock.frame().gaps.iter().all(|gap| !gap.held));
}
