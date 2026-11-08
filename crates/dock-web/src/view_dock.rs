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

    // A region fades while a window is being carried and is simply gone once
    // it has been put down. The ghost is what says which of those is true.
    let moving = move || if dock.frame().ghost.is_some() { " is-moving" } else { "" };

    let cursor = move || match dock.frame().cursor {
        Cursor::Default => "",
        Cursor::ColResize => "is-col-resize",
        Cursor::RowResize => "is-row-resize",
        Cursor::Grabbing => "is-grabbing",
    };

    view! {
        <div
            node_ref=node
            class=move || format!("dock {}{}{}", cursor(), settling(), moving())
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
            "opacity: {}; background: {}; clip-path: path(\"{}\")",
            u8::from(zone().is_some()),
            fill.get(),
            shape.get(),
        )
    };

    view! { <div class="region" style=style></div> }
}

/// A region, pulled in from its own edges so that neighbours do not touch,
/// with its corners rounded.
///
/// The gutter is what makes five regions read as five rather than as one
/// wash, now that they have no outline — the same answer the dock gives for
/// two windows. Pulling every edge in by the same amount is the whole trick:
/// moving each corner towards the middle instead leaves a wide gutter where
/// two edges meet shallowly and a narrow one where they meet squarely, which
/// is the uneven spacing that was there before.
fn clip(zone: Zone) -> String {
    const SEAM: f32 = 4.0;
    const ROUND: f32 = 4.0;

    let points = inset(zone.points, SEAM);
    let mut path = String::new();

    for i in 0..4 {
        let previous = points[(i + 3) % 4];
        let corner = points[i];
        let next = points[(i + 1) % 4];

        let back = corner.distance(previous);
        let on = corner.distance(next);
        let round = ROUND.min(back / 2.0).min(on / 2.0);

        let before = towards(corner, previous, round);
        let after = towards(corner, next, round);

        if i == 0 {
            path.push_str(&format!("M {} {} ", before.x, before.y));
        } else {
            path.push_str(&format!("L {} {} ", before.x, before.y));
        }

        path.push_str(&format!(
            "Q {} {} {} {} ",
            corner.x, corner.y, after.x, after.y
        ));
    }

    path.push('Z');

    path
}

/// Every edge of a convex quadrilateral moved inwards by the same distance,
/// and the corners put back where the moved edges meet.
fn inset(points: [Point; 4], by: f32) -> [Point; 4] {
    let mut moved = points;

    for i in 0..4 {
        let before = edge(points[(i + 3) % 4], points[i], by);
        let after = edge(points[i], points[(i + 1) % 4], by);

        moved[i] = meet(before, after).unwrap_or(points[i]);
    }

    moved
}

/// One edge, moved inwards, as a point on it and the way it runs. The points
/// of a zone always wind the same way, so the inward side is always the same
/// turn from the edge.
fn edge(from: Point, to: Point, by: f32) -> (Point, Point) {
    let run = Point::new(to.x - from.x, to.y - from.y);
    let length = (run.x * run.x + run.y * run.y).sqrt().max(0.001);
    let inward = Point::new(-run.y / length * by, run.x / length * by);

    (Point::new(from.x + inward.x, from.y + inward.y), run)
}

/// Where two lines cross, each given as a point and a direction.
fn meet(a: (Point, Point), b: (Point, Point)) -> Option<Point> {
    let (at, run) = a;
    let (bt, other) = b;

    let cross = run.x * other.y - run.y * other.x;

    if cross.abs() < 0.001 {
        return None;
    }

    let along = ((bt.x - at.x) * other.y - (bt.y - at.y) * other.x) / cross;

    Some(Point::new(at.x + run.x * along, at.y + run.y * along))
}

fn towards(from: Point, to: Point, by: f32) -> Point {
    let length = from.distance(to).max(0.001);

    Point::new(
        from.x + (to.x - from.x) / length * by,
        from.y + (to.y - from.y) / length * by,
    )
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
