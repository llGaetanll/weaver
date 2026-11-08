//! A page with four windows in it, to drag around.
//!
//! It is the smallest thing that uses the library the way a program would:
//! it opens windows, says where each one goes and how it takes its space,
//! and draws whatever it likes inside them.

use dock_state::Event;
use dock_state::EventHost;
use dock_state::Share;
use dock_state::Side;
use dock_state::Slot;
use dock_state::WindowId;
use dock_web::Dock;
use dock_web::DockView;
use leptos::prelude::*;

const CANVAS: WindowId = WindowId(1);
const TIMELINE: WindowId = WindowId(2);
const INSPECTOR: WindowId = WindowId(3);
const NOTES: WindowId = WindowId(4);

fn main() {
    let dock = Dock::new();

    open(
        dock,
        CANVAS,
        "Canvas",
        Slot::Edge { side: Side::Right },
        Share::Flex(1.0),
    );
    open(
        dock,
        TIMELINE,
        "Timeline",
        beside(CANVAS, Side::Down),
        Share::Fixed(140.0),
    );
    open(
        dock,
        INSPECTOR,
        "Inspector",
        Slot::Edge { side: Side::Right },
        Share::Fixed(300.0),
    );
    open(
        dock,
        NOTES,
        "Notes",
        Slot::With { window: INSPECTOR },
        Share::Flex(1.0),
    );

    mount_to_body(move || view! { <Page dock=dock /> });
}

#[component]
fn Page(dock: Dock) -> impl IntoView {
    let content = Callback::new(move |window: WindowId| filler(window));
    let icon = Callback::new(move |window: WindowId| mark(window));

    view! {
        <div class="page">
            <div class="hostbar">
                <b>weaver</b>
                <span>"drag a tab to move a window, a gap to resize one, press a tab to bring it forward"</span>
            </div>

            <DockView dock=dock content=content icon=icon />
        </div>
    }
}

/// What each window wears in its tab. The library draws whatever comes back
/// and nothing at all when a host has none to give.
fn mark(window: WindowId) -> AnyView {
    let path = match window {
        TIMELINE => "M1 6 H11 M3 3 V9 M8 3 V9",
        INSPECTOR => "M2 3 H10 M2 6 H10 M2 9 H7",
        NOTES => "M3 2 H9 L9 10 L6 8 L3 10 Z",
        _ => "M2 2 H10 V10 H2 Z",
    };

    view! {
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
            <path d=path fill="none" stroke="currentColor" stroke-width="1.2" />
        </svg>
    }
    .into_any()
}

/// Stand-ins. A program would put something it cares about here.
fn filler(window: WindowId) -> AnyView {
    match window {
        INSPECTOR => view! {
            <div class="filler">
                <dl>
                    <div><dt>"windows"</dt><dd>"4"</dd></div>
                    <div><dt>"gaps"</dt><dd>"3"</dd></div>
                    <div><dt>"depth"</dt><dd>"2"</dd></div>
                </dl>
            </div>
        }
        .into_any(),

        NOTES => view! {
            <div class="filler">
                <p>"Nothing in this window knows it is in a dock."</p>
                <p>"Pick it up by its tab and the rest of the screen stays where it is."</p>
            </div>
        }
        .into_any(),

        TIMELINE => view! { <div class="lines"></div> }.into_any(),

        _ => view! {
            <div class="filler">
                <p>"The library decides where this rectangle is. What is drawn in it is the program's."</p>
            </div>
        }
        .into_any(),
    }
}

fn open(dock: Dock, window: WindowId, title: &str, at: Slot, share: Share) {
    dock.send(Event::FromHost(EventHost::Opened {
        window,
        title: title.to_string(),
        at,
        share,
    }));
}

fn beside(window: WindowId, side: Side) -> Slot {
    Slot::Beside { window, side }
}
