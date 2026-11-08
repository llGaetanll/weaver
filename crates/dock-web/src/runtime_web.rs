//! The half that touches the browser.
//!
//! It decides nothing. Pointer positions go in as events, a frame comes back
//! out, and the elements are moved to the rectangles the frame names.
//!
//! Events are collected and handed over **once a frame**. A browser delivers
//! a burst of pointer moves between two paints, and a burst is one situation
//! rather than a list of them — so the dock answers it once, and lays out
//! once per frame because of how it is called rather than because something
//! debounces it.

use dock_state::Event;
use dock_state::EventHost;
use dock_state::Frame;
use dock_state::Metrics;
use dock_state::Point;
use dock_state::Size;
use dock_state::StateDock;
use leptos::prelude::*;
use leptos::html::Div;

/// A dock, as a page holds it: the machine, a queue, and a counter the view
/// watches. The machine is stored rather than signalled — it owns a tree, and
/// nothing is gained by cloning it.
#[derive(Clone, Copy)]
pub struct Dock {
    state: StoredValue<StateDock>,
    queue: StoredValue<Vec<Event>>,
    version: RwSignal<u32>,
    surface: StoredValue<Size>,
    published: StoredValue<Frame>,
}

impl Default for Dock {
    fn default() -> Self {
        Self::new()
    }
}

impl Dock {
    pub fn new() -> Self {
        Self {
            state: StoredValue::new(StateDock::default()),
            queue: StoredValue::new(Vec::new()),
            version: RwSignal::new(0),
            surface: StoredValue::new(Size::default()),
            published: StoredValue::new(Frame::default()),
        }
    }

    /// Hand the dock something that happened. It is answered on the next
    /// frame, along with everything else that happened in the same one.
    pub fn send(&self, event: Event) {
        self.queue.update_value(|queue| queue.push(event));
    }

    /// What to draw: the last frame published. Reading it watches the dock,
    /// so a view that calls it is rebuilt when the dock changes — and it is
    /// the frame the whole view shares, rather than one solve per caller.
    pub fn frame(&self) -> Frame {
        self.version.track();

        self.published.get_value()
    }

    pub fn metrics(&self) -> Metrics {
        self.state.with_value(|state| state.metrics())
    }

    /// One turn: every event that arrived since the last frame, then the
    /// actions they add up to.
    fn turn(&self) {
        let events = self.queue.try_update_value(std::mem::take).unwrap_or_default();

        if events.is_empty() {
            return;
        }

        let mut actions = Vec::new();

        self.state.update_value(|state| {
            state.handle_events(&events);
            state.emit_actions(&mut actions);
        });

        for action in actions {
            self.perform_action(&action);
        }

        // The pointer moves over the page all day and most of it changes
        // nothing. Publishing only what is different is the runtime's job:
        // the machine says what should be true, and the runtime makes it true
        // without saying it twice.
        let next = self.state.with_value(|state| state.frame());

        if self.published.with_value(|frame| *frame == next) {
            return;
        }

        self.published.set_value(next);
        self.version.update(|version| *version += 1);
    }

    fn perform_action(&self, action: &dock_state::Action) {
        match *action {}
    }

    /// The surface is measured rather than listened for: the dock is told how
    /// big it is the same way it is told anything else.
    fn measure(&self, rect: Option<web_sys::DomRect>) {
        let Some(rect) = rect else {
            return;
        };

        let size = Size::new(rect.width() as f32, rect.height() as f32);

        if self.surface.get_value() == size {
            return;
        }

        self.surface.set_value(size);
        self.send(Event::FromHost(EventHost::Resized { size }));
    }

    /// Where a browser event is, in the dock's own coordinates.
    pub fn point(&self, node: NodeRef<Div>, x: f32, y: f32) -> Point {
        match rect_of(node) {
            Some(rect) => Point::new(x - rect.left() as f32, y - rect.top() as f32),
            None => Point::new(x, y),
        }
    }

    /// Start the frame loop. Each frame measures the surface and answers
    /// whatever has arrived.
    pub fn run(&self, node: NodeRef<Div>) {
        let dock = *self;

        fn tick(dock: Dock, node: NodeRef<Div>) {
            request_animation_frame(move || {
                dock.measure(rect_of(node));
                dock.turn();

                tick(dock, node);
            });
        }

        tick(dock, node);
    }
}

fn rect_of(node: NodeRef<Div>) -> Option<web_sys::DomRect> {
    node.get_untracked()
        .map(|element| element.get_bounding_client_rect())
}
