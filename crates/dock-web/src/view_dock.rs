//! The elements. A window is a box the frame puts somewhere; what is in one
//! comes from the host.

use dock_state::Cursor;
use dock_state::Event;
use dock_state::EventPointer;
use dock_state::Point;
use dock_state::Rect;
use dock_state::WindowId;
use dock_state::Zone;
use leptos::ev;
use leptos::html::Div;
use leptos::prelude::*;

use crate::runtime_web::Dock;

/// The surface every window lives on.
///
/// One pointer listener, on the surface, because which grip or gap a press
/// landed on is the machine's question to answer — it owns the rectangles.
#[component]
pub fn DockView(
    dock: Dock,
    /// What goes inside a window. Called once per window, not once per frame.
    content: Callback<WindowId, AnyView>,
) -> impl IntoView {
    let node: NodeRef<Div> = NodeRef::new();
    let metrics = dock.metrics();

    Effect::new(move |_| dock.run(node));

    let _ = window_event_listener(ev::pointermove, move |event| {
        let at = dock.point(node, event.client_x() as f32, event.client_y() as f32);

        dock.send(Event::FromPointer(EventPointer::Moved { at }));
    });

    let _ = window_event_listener(ev::pointerup, move |event| {
        let at = dock.point(node, event.client_x() as f32, event.client_y() as f32);

        dock.send(Event::FromPointer(EventPointer::Up { at }));
    });

    let _ = window_event_listener(ev::keydown, move |event| {
        if event.key() == "Escape" {
            dock.send(Event::FromPointer(EventPointer::Cancelled));
        }
    });

    // The list of windows, which changes when one opens or closes — not every
    // time one moves. Keying on it is what keeps a window's contents alive
    // across a drag.
    let windows = Memo::new(move |_| {
        dock.frame()
            .windows
            .iter()
            .map(|window| window.window)
            .collect::<Vec<_>>()
    });

    let gaps = Memo::new(move |_| dock.frame().gaps.len());

    let settling = move || if dock.frame().settling { " settles" } else { "" };

    let cursor = move || match dock.frame().cursor {
        Cursor::Default => "",
        Cursor::ColResize => "is-col-resize",
        Cursor::RowResize => "is-row-resize",
        Cursor::Grabbing => "is-grabbing",
    };

    view! {
        <div
            node_ref=node
            class=move || format!("dock {}{}", cursor(), settling())
            style=format!("--tab: {}px; --gap: {}px", metrics.tab, metrics.gap)
            on:pointerdown=move |event: ev::PointerEvent| {
                let at = dock.point(node, event.client_x() as f32, event.client_y() as f32);
                dock.send(Event::FromPointer(EventPointer::Down { at }));
            }
        >
            <For each=move || windows.get() key=|window| *window let:window>
                <Window dock=dock window=window content=content />
            </For>

            <For each=move || 0..gaps.get() key=|index| *index let:index>
                <Gap dock=dock index=index />
            </For>

            <Regions dock=dock />
            <Ghost dock=dock />
        </div>
    }
}

#[component]
fn Window(dock: Dock, window: WindowId, content: Callback<WindowId, AnyView>) -> impl IntoView {
    let drawn = move || {
        dock.frame()
            .windows
            .iter()
            .find(|drawn| drawn.window == window)
            .cloned()
    };

    let title = move || drawn().map(|drawn| drawn.title).unwrap_or_default();
    let lifted = move || drawn().is_some_and(|drawn| drawn.lifted);
    let style = move || drawn().map(|drawn| place(drawn.rect)).unwrap_or_default();

    view! {
        <div class=move || if lifted() { "win lifted" } else { "win" } style=style>
            <div class="win-tab">
                <span class="grip" title="Drag to move this window">
                    <svg viewBox="0 0 10 16" width="10" height="16" aria-hidden="true">
                        <circle cx="3" cy="4" r="1.1" /><circle cx="7" cy="4" r="1.1" />
                        <circle cx="3" cy="8" r="1.1" /><circle cx="7" cy="8" r="1.1" />
                        <circle cx="3" cy="12" r="1.1" /><circle cx="7" cy="12" r="1.1" />
                    </svg>
                </span>
                <h3>{title}</h3>
            </div>
            <div class="win-body">{content.run(window)}</div>
        </div>
    }
}

/// The gap is the resize control, so the only thing it draws is the cursor
/// that says so.
#[component]
fn Gap(dock: Dock, index: usize) -> impl IntoView {
    let drawn = move || dock.frame().gaps.get(index).copied();

    let class = move || match drawn().map(|gap| gap.axis) {
        Some(dock_state::Axis::Row) => "gap gap-v",
        Some(dock_state::Axis::Col) => "gap gap-h",
        None => "gap",
    };

    view! { <div class=class style=move || drawn().map(|gap| place(gap.rect)).unwrap_or_default()></div> }
}

/// Every region on offer, one element each.
///
/// They are drawn as clipped boxes rather than as an SVG because they have to
/// *change* — the one under the pointer darkens as the pointer moves between
/// them, and a shape rebuilt from markup has no previous value to ease from.
#[component]
fn Regions(dock: Dock) -> impl IntoView {
    view! {
        <div class="regions">
            <For each=move || 0..dock_state::REGIONS key=|index| *index let:index>
                <Region dock=dock index=index />
            </For>
        </div>
    }
}

#[component]
fn Region(dock: Dock, index: usize) -> impl IntoView {
    let zone = move || dock.frame().regions.get(index).copied();

    // The shape a region had is kept while it fades out. Dropping the clip
    // with the region would leave the whole surface washed for as long as the
    // fade lasts.
    let shape = RwSignal::new(String::new());
    let fill = RwSignal::new("var(--drop)");

    Effect::new(move |_| {
        if let Some(zone) = zone() {
            shape.set(clip(zone));
            fill.set(if zone.chosen {
                "var(--drop-chosen)"
            } else {
                "var(--drop)"
            });
        }
    });

    let style = move || {
        format!(
            "opacity: {}; background: {}; clip-path: polygon({})",
            u8::from(zone().is_some()),
            fill.get(),
            shape.get(),
        )
    };

    view! { <div class="region" style=style></div> }
}

/// A region, pulled in from its own edges so the regions do not touch. The
/// seam is what makes five of them read as five rather than as one wash —
/// the same answer the dock gives for two windows.
fn clip(zone: Zone) -> String {
    const SEAM: f32 = 1.0;

    let middle = Point::new(
        zone.points.iter().map(|point| point.x).sum::<f32>() / 4.0,
        zone.points.iter().map(|point| point.y).sum::<f32>() / 4.0,
    );

    zone.points
        .iter()
        .map(|point| {
            let away = point.distance(middle).max(1.0);
            let pull = (away - SEAM) / away;

            format!(
                "{}px {}px",
                middle.x + (point.x - middle.x) * pull,
                middle.y + (point.y - middle.y) * pull,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The window under the cursor. Its title and nothing else: the host's
/// contents are already on screen, in the window that stayed behind.
#[component]
fn Ghost(dock: Dock) -> impl IntoView {
    let rect = move || dock.frame().ghost;

    let title = move || {
        dock.frame()
            .windows
            .iter()
            .find(|window| window.lifted)
            .map(|window| window.title.clone())
            .unwrap_or_default()
    };

    view! {
        <Show when=move || rect().is_some()>
            <div class="ghost" style=move || rect().map(place).unwrap_or_default()>
                <div class="win-tab">
                    <span class="grip">
                        <svg viewBox="0 0 10 16" width="10" height="16" aria-hidden="true">
                            <circle cx="3" cy="4" r="1.1" /><circle cx="7" cy="4" r="1.1" />
                            <circle cx="3" cy="8" r="1.1" /><circle cx="7" cy="8" r="1.1" />
                            <circle cx="3" cy="12" r="1.1" /><circle cx="7" cy="12" r="1.1" />
                        </svg>
                    </span>
                    <h3>{title}</h3>
                </div>
                <div class="win-body"></div>
            </div>
        </Show>
    }
}

fn place(rect: Rect) -> String {
    format!(
        "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px",
        rect.x, rect.y, rect.w, rect.h,
    )
}
