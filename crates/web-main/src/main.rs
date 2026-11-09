//! A page with a few windows in it, to drag around.
//!
//! It is the smallest thing that uses the library the way a program would:
//! it opens windows, says where each one goes and how it takes its space,
//! and draws whatever it likes inside them. Nothing in here is the library's
//! — the file tree, the code, the test run and the terminal are all this
//! page's own, and the dock has never heard of any of them.

use dock_state::Event;
use dock_state::EventHost;
use dock_state::Share;
use dock_state::Side;
use dock_state::Slot;
use dock_state::WindowId;
use dock_web::Dock;
use dock_web::DockView;
use leptos::prelude::*;

const EDITOR: WindowId = WindowId(1);
const TESTS: WindowId = WindowId(2);
const TERMINAL: WindowId = WindowId(3);
const FILES: WindowId = WindowId(4);
const OUTLINE: WindowId = WindowId(5);
const NOTES: WindowId = WindowId(6);

fn main() {
    let dock = Dock::new();

    // The first window is the whole dock, and every one after it says where
    // it goes relative to one already open.
    open(
        dock,
        EDITOR,
        "editor",
        Slot::Edge { side: Side::Right },
        Share::Flex(1.0),
    );
    open(dock, TESTS, "tests", with(EDITOR), Share::Flex(1.0));
    open(
        dock,
        TERMINAL,
        "terminal",
        beside(EDITOR, Side::Down),
        Share::Fixed(168.0),
    );
    open(
        dock,
        FILES,
        "files",
        Slot::Edge { side: Side::Left },
        Share::Fixed(224.0),
    );
    open(
        dock,
        OUTLINE,
        "outline",
        Slot::Edge { side: Side::Right },
        Share::Fixed(268.0),
    );
    open(
        dock,
        NOTES,
        "notes",
        beside(OUTLINE, Side::Down),
        Share::Flex(1.0),
    );

    // Arriving in a slot means arriving in front, so the last one in would be
    // the one showing. This page would rather you saw the code.
    dock.send(Event::FromHost(EventHost::Show { window: EDITOR }));

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
                <span>"crates/dock-state/src/state_dock.rs"</span>
            </div>

            <DockView dock=dock content=content icon=icon />
        </div>
    }
}

/// What each window wears in its tab. The library draws whatever comes back
/// and nothing at all when a host has none to give.
fn mark(window: WindowId) -> AnyView {
    let path = match window {
        FILES => "M1.5 3.5 H5 L6 5 H10.5 V9.5 H1.5 Z",
        TERMINAL => "M2 3.5 L5 6 L2 8.5 M6.5 9 H10",
        OUTLINE => "M2 3 H10 M4 6 H10 M6 9 H10",
        NOTES => "M3 2 H9 L9 10 L6 8 L3 10 Z",
        TESTS => "M2.5 6.5 L5 9 L9.5 3.5",
        _ => "M4.5 3.5 L2 6 L4.5 8.5 M7.5 3.5 L10 6 L7.5 8.5",
    };

    view! {
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
            <path
                d=path
                fill="none"
                stroke="currentColor"
                stroke-width="1.1"
                stroke-linecap="round"
                stroke-linejoin="round"
            />
        </svg>
    }
    .into_any()
}

/// Stand-ins. A program would put something it cares about here.
fn filler(window: WindowId) -> AnyView {
    match window {
        FILES => files().into_any(),
        EDITOR => editor().into_any(),
        TESTS => tests().into_any(),
        TERMINAL => terminal().into_any(),
        OUTLINE => outline().into_any(),
        _ => notes().into_any(),
    }
}

fn files() -> impl IntoView {
    let rows = [
        (0, "crates", true, false),
        (1, "dock-state", true, false),
        (2, "event_host.rs", false, false),
        (2, "event_pointer.rs", false, false),
        (2, "frame.rs", false, false),
        (2, "geometry.rs", false, false),
        (2, "metrics.rs", false, false),
        (2, "solve.rs", false, false),
        (2, "state_dock.rs", false, true),
        (2, "tree.rs", false, false),
        (1, "dock-web", true, false),
        (2, "runtime_web.rs", false, false),
        (2, "view_dock.rs", false, false),
        (2, "dock.css", false, false),
        (1, "web-main", true, false),
        (0, "sketches", true, false),
        (1, "design", true, false),
        (1, "dock", true, false),
        (0, "tools", true, false),
        (1, "trace.sh", false, false),
        (1, "read-trace.py", false, false),
        (0, "CLAUDE.md", false, false),
        (0, "Cargo.toml", false, false),
    ];

    view! {
        <div class="tree">
            {rows
                .into_iter()
                .map(|(depth, name, folder, here)| {
                    view! {
                        <div
                            class=if here { "row is-here" } else { "row" }
                            style=format!("padding-left: {}px", 10 + depth * 13)
                        >
                            <span class="dot">{if folder { "▸" } else { "" }}</span>
                            <span>{name}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

fn editor() -> impl IntoView {
    let lines = [
        ("/// Where a window is, as a path from the root.", true),
        (
            "pub fn find(&self, window: WindowId) -> Option<Path> {",
            false,
        ),
        ("    let mut path = Path::new();", false),
        ("", false),
        (
            "    self.find_into(window, &mut path).then_some(path)",
            false,
        ),
        ("}", false),
        ("", false),
        (
            "/// The two rules that keep a tree canonical, applied bottom up.",
            true,
        ),
        ("pub fn normalize(node: &mut Node) {", false),
        (
            "    let Node::Group { axis, children } = node else {",
            false,
        ),
        ("        return;", false),
        ("    };", false),
        ("", false),
        ("    for child in children.iter_mut() {", false),
        ("        normalize(&mut child.node);", false),
        ("    }", false),
        ("", false),
        ("    let axis = *axis;", false),
        (
            "    let mut folded: Vec<Child> = Vec::with_capacity(children.len());",
            false,
        ),
        ("", false),
        ("    for child in children.drain(..) {", false),
        ("        match child.node {", false),
        ("            Node::Group {", false),
        ("                axis: inner,", false),
        ("                children: grandchildren,", false),
        (
            "            } if inner == axis => folded.extend(rescale(grandchildren, child.share)),",
            false,
        ),
        ("            node => folded.push(Child {", false),
        ("                share: child.share,", false),
        ("                node,", false),
        ("            }),", false),
        ("        }", false),
        ("    }", false),
        ("", false),
        ("    *children = folded;", false),
        ("", false),
        ("    if children.len() == 1 {", false),
        ("        let Some(only) = children.pop() else {", false),
        (
            "            unreachable!(\"just checked the length\");",
            false,
        ),
        ("        };", false),
        ("", false),
        (
            "        // The share belongs to the slot, and the slot is the parent's.",
            true,
        ),
        ("        *node = only.node;", false),
        ("    }", false),
        ("}", false),
    ];

    view! {
        <div class="code">
            {lines
                .into_iter()
                .enumerate()
                .map(|(at, (text, note))| {
                    view! {
                        <div class="line">
                            <span class="number">{at + 1}</span>
                            <span class=if note { "note" } else { "" }>{text}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

fn tests() -> impl IntoView {
    let rows = [
        (true, "opening_windows_divides_the_surface"),
        (true, "a_drop_lands_on_the_side_its_band_was_on"),
        (true, "the_middle_of_a_window_is_its_stack"),
        (false, "a_tab_dragged_along_its_strip_changes_the_order"),
        (true, "the_corner_where_two_gaps_meet_moves_both"),
    ];

    view! {
        <div class="runs">
            {rows
                .into_iter()
                .map(|(passed, name)| {
                    view! {
                        <div class="run">
                            <span class=if passed { "ok" } else { "no" }>
                                {if passed { "ok" } else { "FAILED" }}
                            </span>
                            <span class="what">{name}</span>
                        </div>
                    }
                })
                .collect_view()}
            <div class="summary">"4 passed, 1 failed in 0.21s"</div>
        </div>
    }
}

fn terminal() -> impl IntoView {
    view! {
        <div class="shell">
            <div><span class="prompt">"$"</span>" cargo test --workspace"</div>
            <div class="dim">"   Compiling dock-state v0.1.0"</div>
            <div class="dim">"   Compiling dock-web v0.1.0"</div>
            <div class="dim">"    Finished `test` profile in 2.41s"</div>
            <div class="dim">"     Running tests/dock.rs"</div>
            <div class="dim">"test result: ok. 19 passed; 0 failed"</div>
            <div><span class="prompt">"$"</span>" tools/trace.sh \"down:1120,13|up:40,200\""</div>
            <div class="dim">"157 frames over 2616ms, 8-18ms apart (median 17)"</div>
            <div class="dim">"paneCanvas  x: 0 -> 450, moving 434-2333ms"</div>
            <div><span class="prompt">"$"</span><span class="caret"></span></div>
        </div>
    }
}

fn outline() -> impl IntoView {
    let rows = [
        (0, "WindowId", "struct"),
        (0, "Share", "enum"),
        (1, "Flex", ""),
        (1, "Fixed", ""),
        (0, "Stack", "struct"),
        (1, "windows", "fn"),
        (1, "showing", "fn"),
        (1, "holds", "fn"),
        (1, "other", "fn"),
        (1, "show", "fn"),
        (1, "take", "fn"),
        (1, "drop", "fn"),
        (0, "Node", "enum"),
        (1, "find", "fn"),
        (1, "at_mut", "fn"),
        (1, "stack_mut", "fn"),
        (1, "last", "fn"),
        (0, "Slot", "enum"),
        (1, "Beside", ""),
        (1, "Edge", ""),
        (1, "With", ""),
        (0, "place", "fn"),
        (0, "remove", "fn"),
        (0, "normalize", "fn"),
        (0, "rescale", "fn"),
    ];

    view! {
        <div class="tree">
            {rows
                .into_iter()
                .map(|(depth, name, kind)| {
                    view! {
                        <div class="row" style=format!("padding-left: {}px", 10 + depth * 13)>
                            <span>{name}</span>
                            <span class="kind">{kind}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

fn notes() -> impl IntoView {
    view! {
        <div class="prose">
            <p>"Nothing in this window knows it is in a dock."</p>
            <p>
                "Take it by its tab and the rest of the screen stays where it is.
                 Drop it on the middle of another window to share a slot, or on
                 one of its edges to take a side of it."
            </p>
            <dl>
                <div><dt>"windows"</dt><dd>"6"</dd></div>
                <div><dt>"slots"</dt><dd>"5"</dd></div>
                <div><dt>"depth"</dt><dd>"2"</dd></div>
            </dl>
        </div>
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

fn with(window: WindowId) -> Slot {
    Slot::With {
        window,
        before: false,
    }
}
