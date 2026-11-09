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
/// One pointer listener, on the surface, because which tab or gap a press
/// landed on is the machine's question to answer — it owns the rectangles.
#[component]
pub fn DockView(
    dock: Dock,
    /// What goes inside a window. Called once per window, not once per frame.
    content: Callback<WindowId, AnyView>,
    /// What goes in a window's tab, before its name. The library has no
    /// opinion about what a window is, so it draws whatever the host hands
    /// back and nothing when there is none.
    #[prop(optional)]
    icon: Option<Callback<WindowId, AnyView>>,
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

    // Every list is keyed by what the thing *is* rather than where it is: an
    // element has to follow its own slot across a rearrangement, or it takes
    // on another one's rectangle and eases between two places that have
    // nothing to do with each other.
    //
    // And every list is kept in one order, which is nobody's idea of the
    // order things are in on screen. Everything here is placed by its own
    // rectangle, so the order the elements sit in makes no difference to the
    // picture — while moving one to keep a list tidy takes it out of the page
    // and puts it back, which ends whatever it was in the middle of doing.
    let windows = Memo::new(move |_| sorted(dock.frame().windows.iter().map(|w| w.window)));
    let panes = Memo::new(move |_| sorted(dock.frame().panes.iter().map(|pane| pane.key)));
    let gaps = Memo::new(move |_| sorted(dock.frame().gaps.iter().map(|gap| gap.after)));

    let settling = move || {
        if dock.frame().settling {
            " settles"
        } else {
            ""
        }
    };

    // A region fades while a window is being carried and is simply gone once
    // it has been put down. The ghost is what says which of those is true.
    let moving = move || {
        if dock.frame().carried.is_some() {
            " is-moving"
        } else {
            ""
        }
    };

    let cursor = move || match dock.frame().cursor {
        Cursor::Default => "",
        Cursor::ColResize => "is-col-resize",
        Cursor::RowResize => "is-row-resize",
        Cursor::BothResize => "is-both-resize",
        Cursor::Grabbing => "is-grabbing",
    };

    view! {
        <div
            node_ref=node
            class=move || format!("dock {}{}{}", cursor(), settling(), moving())
            style=format!(
                "--tab: {}px; --gap: {}px; --grab: {}px",
                metrics.tab, metrics.gap, metrics.grab,
            )
            on:pointerdown=move |event: ev::PointerEvent| {
                let at = dock.point(node, event.client_x() as f32, event.client_y() as f32);
                dock.send(Event::FromPointer(EventPointer::Down { at }));
            }
        >
            <For each=move || panes.get() key=|pane| *pane let:pane>
                <PaneView dock=dock pane=pane icon=icon />
            </For>

            <For each=move || windows.get() key=|window| *window let:window>
                <Window dock=dock window=window content=content />
            </For>

            <For each=move || gaps.get() key=|gap| *gap let:gap>
                <Gap dock=dock after=gap />
            </For>

            <Regions dock=dock />
            <Ghost dock=dock icon=icon />
        </div>
    }
}

/// One slot: the box a stack of windows shares, and the tabs along the top
/// of it. The tabs are drawn here rather than beside the windows because they
/// belong to the slot — a slot with one window in it has one tab, and that is
/// the title bar.
#[component]
fn PaneView(dock: Dock, pane: WindowId, icon: Option<Callback<WindowId, AnyView>>) -> impl IntoView {
    let key = pane;
    let pane = move || {
        dock.frame()
            .panes
            .iter()
            .find(|pane| pane.key == key)
            .cloned()
    };
    let tabs = Memo::new(move |_| pane().map(|pane| pane.tabs.len()).unwrap_or_default());

    // Keyed by window and kept in one order, for the reasons above: a tab
    // that changed places in the list would be taken out of the page and put
    // back, which is the one thing that stops it sliding there.
    let windows = Memo::new(move |_| {
        sorted(
            pane()
                .map(|pane| pane.tabs.iter().map(|tab| tab.window).collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter(),
        )
    });

    let strip = move || match tabs.get() {
        0 | 1 => "pane-strip",
        _ => "pane-strip is-stacked",
    };

    view! {
        <div class="pane" style=move || pane().map(|pane| place(pane.rect)).unwrap_or_default()>
            <div class=strip></div>
            <For each=move || windows.get() key=|window| *window let:window>
                <Tab dock=dock pane=key window=window icon=icon />
            </For>
        </div>
    }
}

/// A tab: whatever the host draws for a window, and its name. The whole of
/// it is the handle.
#[component]
fn Tab(
    dock: Dock,
    pane: WindowId,
    window: WindowId,
    icon: Option<Callback<WindowId, AnyView>>,
) -> impl IntoView {
    let drawn = move || {
        let frame = dock.frame();
        let pane = frame.panes.iter().find(|held| held.key == pane)?;
        let at = pane.tabs.iter().position(|tab| tab.window == window)?;

        Some((pane.rect, pane.tabs.get(at)?.clone(), at, pane.tabs.len()))
    };

    let class = move || {
        let Some((_, tab, at, count)) = drawn() else {
            return String::new();
        };

        let mut class = String::from("tab");

        if tab.showing {
            class.push_str(" is-showing");
        }

        if at == 0 {
            class.push_str(" is-first");
        }

        if tab.lifted {
            class.push_str(" is-lifted");
        }

        if count == 1 {
            class.push_str(" is-only");
        }

        class
    };

    // A tab is placed inside its own slot, so its rectangle is given from the
    // slot's corner rather than the dock's.
    let style = move || match drawn() {
        Some((pane, tab, _, _)) => place(Rect::new(
            tab.rect.x - pane.x,
            tab.rect.y - pane.y,
            tab.rect.w,
            tab.rect.h,
        )),
        None => String::new(),
    };

    let mark = move || Some(icon?.run(window));

    view! {
        <div class=class style=style title="Drag to move this window">
            <span class="tab-icon">{mark}</span>
            <h3>{move || drawn().map(|(_, tab, _, _)| tab.title).unwrap_or_default()}</h3>
        </div>
    }
}

/// What the host put in a window. A window behind a tab keeps its contents
/// and its place — it is hidden rather than taken down, so leafing through a
/// stack costs the host nothing.
#[component]
fn Window(dock: Dock, window: WindowId, content: Callback<WindowId, AnyView>) -> impl IntoView {
    let drawn = move || {
        dock.frame()
            .windows
            .iter()
            .find(|drawn| drawn.window == window)
            .cloned()
    };

    let class = move || match drawn() {
        Some(drawn) if !drawn.showing => "win is-hidden",
        Some(drawn) if drawn.lifted => "win is-lifted",
        _ => "win",
    };

    view! {
        <div class=class style=move || drawn().map(|drawn| place(drawn.rect)).unwrap_or_default()>
            {content.run(window)}
        </div>
    }
}

/// The gap is the resize control, so the only thing it draws is the cursor
/// that says so.
#[component]
fn Gap(dock: Dock, after: WindowId) -> impl IntoView {
    let drawn = move || {
        dock.frame()
            .gaps
            .iter()
            .find(|gap| gap.after == after)
            .copied()
    };

    let class = move || match drawn() {
        Some(gap) if gap.held && gap.axis == dock_state::Axis::Row => "gap gap-v is-held",
        Some(gap) if gap.held => "gap gap-h is-held",
        Some(gap) if gap.axis == dock_state::Axis::Row => "gap gap-v",
        Some(_) => "gap gap-h",
        None => "gap",
    };

    // Drawn wider than it is, by as much as the machine counts as being on
    // it, so that the pointer meets the cursor and the line at the same time.
    let grab = dock.metrics().grab;

    view! {
        <div
            class=class
            style=move || {
                drawn()
                    .map(|gap| place(gap.rect.thickened(gap.axis, grab)))
                    .unwrap_or_default()
            }
        ></div>
    }
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

/// A region, pulled in from its own edges so that neighbours do not touch.
///
/// The gutter is what makes five regions read as five rather than as one
/// wash, now that they have no outline — the same answer the dock gives for
/// two windows. Pulling every edge in by the same amount is the whole trick:
/// moving each corner towards the middle instead leaves a wide gutter where
/// two edges meet shallowly and a narrow one where they meet squarely, which
/// is uneven spacing by another name.
fn clip(zone: Zone) -> String {
    const SEAM: f32 = 4.0;

    inset(zone.points, SEAM)
        .iter()
        .map(|point| format!("{}px {}px", point.x, point.y))
        .collect::<Vec<_>>()
        .join(", ")
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

#[component]
fn Ghost(dock: Dock, icon: Option<Callback<WindowId, AnyView>>) -> impl IntoView {
    let carried = move || dock.frame().carried;
    let mark = move || Some(icon?.run(carried()?.window));

    view! {
        <Show when=move || carried().is_some()>
            <div
                class="ghost"
                style=move || carried().map(|tab| place(tab.rect)).unwrap_or_default()
            >
                <div class="tab is-showing is-only">
                    <span class="tab-icon">{mark}</span>
                    <h3>{move || carried().map(|tab| tab.title).unwrap_or_default()}</h3>
                </div>
            </div>
        </Show>
    }
}

/// The same windows in the same order every time, whatever order they happen
/// to be drawn in.
fn sorted(windows: impl Iterator<Item = WindowId>) -> Vec<WindowId> {
    let mut windows: Vec<WindowId> = windows.collect();

    windows.sort_by_key(|window| window.0);

    windows
}

fn place(rect: Rect) -> String {
    format!(
        "position: absolute; left: {}px; top: {}px; width: {}px; height: {}px",
        rect.x, rect.y, rect.w, rect.h,
    )
}
