//! Windows arriving and leaving, and how the surface is divided between
//! whatever is there.

mod fixture;

use dock_state::Event;
use dock_state::EventHost::Closed;
use dock_state::EventHost::Opened;
use dock_state::Share;
use dock_state::Side;
use dock_state::Slot;
use fixture::A;
use fixture::B;
use fixture::C;
use fixture::near;
use fixture::rect;
use fixture::three;
use eval::eval;

#[test]
fn opening_windows_divides_the_surface() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Flex(1.0),
        }),
        < Event::FromHost(Opened {
            window: C,
            title: String::from("C"),
            at: Slot::Beside { window: B, side: Side::Down },
            share: Share::Flex(1.0),
        }),
    );

    let frame = dock.frame();

    assert_eq!(frame.panes.len(), 3);
    assert_eq!(frame.gaps.len(), 2, "one between the columns, one within");

    near(rect(&dock, A).w, 399.5, "a half, less half a gap");
    near(rect(&dock, B).h, 299.5, "a half of the column");
    near(rect(&dock, C).y, 300.5, "under B and its gap");
}

/// A group with one child is not a group, so the slot that is left takes the
/// whole of what the two of them had.
#[test]
fn closing_a_window_dissolves_its_group() {
    let mut dock = three();

    eval!(&mut dock, < Event::FromHost(Closed { window: C }));

    let frame = dock.frame();

    assert_eq!(frame.panes.len(), 2);
    assert_eq!(frame.gaps.len(), 1, "the column is gone, not left holding one");

    near(rect(&dock, B).h, 600.0, "B has the whole height back");
}

/// A window the host asks for arrives where it says and takes the space it
/// says, and a number of pixels stays that number of pixels.
#[test]
fn a_window_can_claim_a_width_rather_than_a_share() {
    let mut dock = fixture::one();

    eval!(&mut dock,
        < Event::FromHost(Opened {
            window: B,
            title: String::from("B"),
            at: Slot::Beside { window: A, side: Side::Right },
            share: Share::Fixed(300.0),
        }),
    );

    near(rect(&dock, B).w, 300.0, "as asked");
    near(rect(&dock, A).w, 499.0, "and A takes the rest");
}
