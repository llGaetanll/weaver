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

    open(dock, CANVAS, "Canvas", Slot::Edge { side: Side::Right }, Share::Flex(1.0));
    open(dock, TIMELINE, "Timeline", beside(CANVAS, Side::Down), Share::Fixed(140.0));
    open(dock, INSPECTOR, "Inspector", Slot::Edge { side: Side::Right }, Share::Fixed(300.0));
    open(dock, NOTES, "Notes", beside(INSPECTOR, Side::Down), Share::Flex(1.0));

    mount_to_body(move || view! { <Page dock=dock /> });
}

#[component]
fn Page(dock: Dock) -> impl IntoView {
    let content = Callback::new(move |window: WindowId| filler(window));

    view! {
        <div class="page">
            <div class="hostbar">
                <b>ui-lib</b>
                <span>"drag a grip to move a window, a gap to resize one"</span>
            </div>

            <DockView dock=dock content=content />
        </div>
    }
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
                <p>"Pick it up by the grip and the rest of the screen stays where it is."</p>
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
