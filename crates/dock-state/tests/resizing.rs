//! Taking hold of the line between two windows.

mod fixture;

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
