//! The dock: one tree, one surface, and at most one gesture.

use crate::action::Action;
use crate::event::Event;
use crate::event_host::EventHost;
use crate::event_pointer::EventPointer;
use crate::frame::Cursor;
use crate::frame::Frame;
use crate::frame::GapFrame;
use crate::frame::PaneFrame;
use crate::frame::TabFrame;
use crate::frame::WindowFrame;
use crate::frame::Zone;
use crate::geometry::Axis;
use crate::geometry::Point;
use crate::geometry::Rect;
use crate::geometry::Side;
use crate::geometry::Size;
use crate::metrics::Metrics;
use crate::solve::Pane;
use crate::solve::Solved;
use crate::solve::solve;
use crate::tree::Node;
use crate::tree::Path;
use crate::tree::Share;
use crate::tree::Slot;
use crate::tree::WindowId;
use crate::tree::place;
use crate::tree::remove;

/// The most regions that can be on offer at once: the four bands of a window
/// and its middle.
pub const REGIONS: usize = 5;

/// A window the host has opened, and the name the dock writes on it.
#[derive(Clone, Debug)]
struct Window {
    id: WindowId,
    title: String,
}

/// What the pointer is in the middle of. One value, so resizing while moving
/// is not a state the dock can be in.
#[derive(Clone, Debug)]
enum Gesture {
    /// A grip is held but has not travelled far enough to be a move.
    Pressed { window: WindowId, origin: Point },

    Moving {
        window: WindowId,
        /// Where in the ghost the cursor is holding it.
        grab: Point,
        at: Point,
    },

    Resizing {
        group: Path,
        index: usize,
        /// Where along the axis the drag started, and what the pair measured
        /// then. Counting from the start rather than from the last position
        /// is what stops a drag past a minimum and back from drifting.
        origin: f32,
        before: (Share, Share),
        extent: (f32, f32),
    },
}

#[derive(Clone, Debug, Default)]
pub struct StateDock {
    metrics: Metrics,
    surface: Size,
    root: Option<Node>,
    windows: Vec<Window>,
    gesture: Option<Gesture>,
    /// Whether the layout the dock is in now is one the windows should be
    /// moved into rather than snapped to. Only the events that change the
    /// layout touch it: a pointer moving over a dock that has just settled
    /// must not cut the settling short.
    settling: bool,
}

impl StateDock {
    pub fn new(metrics: Metrics) -> Self {
        Self {
            metrics,
            ..Self::default()
        }
    }

    pub fn metrics(&self) -> Metrics {
        self.metrics
    }

    pub fn handle_events(&mut self, events: &[Event]) {
        for event in events {
            self.handle_event(event);
        }
    }

    /// The dock has nothing to ask of the world yet. See `action.rs`.
    pub fn emit_actions(&mut self, _actions: &mut Vec<Action>) {}

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::FromPointer(event) => self.handle_event_pointer(event),
            Event::FromHost(event) => self.handle_event_host(event),
        }
    }

    fn handle_event_host(&mut self, event: &EventHost) {
        // Dragging the browser's own edge is continuous, so the windows keep
        // up with it; a window opening or closing is not, so the rest of them
        // move over to make room.
        self.settling = !matches!(event, EventHost::Resized { .. });

        match event {
            EventHost::Resized { size } => self.surface = *size,

            EventHost::Opened {
                window,
                title,
                at,
                share,
            } => {
                if self.windows.iter().any(|open| open.id == *window) {
                    return;
                }

                self.windows.push(Window {
                    id: *window,
                    title: title.clone(),
                });

                place(&mut self.root, *window, *at, *share);
            }

            EventHost::Closed { window } => {
                self.windows.retain(|open| open.id != *window);
                remove(&mut self.root, *window);

                if self.moving() == Some(*window) {
                    self.gesture = None;
                }
            }
        }
    }

    fn handle_event_pointer(&mut self, event: &EventPointer) {
        match event {
            EventPointer::Down { at } => self.press(*at),
            EventPointer::Moved { at } => self.drag(*at),
            EventPointer::Up { at } => self.release(*at),
            EventPointer::Cancelled => self.cancel(),
        }
    }

    /// A press lands on a gap, a grip, or nothing at all. A gap starts
    /// resizing at once — there is nothing else it could mean — while a grip
    /// waits to see whether the pointer travels, so that a click stays a
    /// click.
    fn press(&mut self, at: Point) {
        let Some(solved) = self.solved() else {
            return;
        };

        let hit = solved
            .gaps
            .iter()
            .find(|gap| gap.rect.grown(2.0).contains(at));

        if let Some(gap) = hit {
            let Some((a, b)) = self.shares(&gap.group, gap.index) else {
                return;
            };

            self.gesture = Some(Gesture::Resizing {
                group: gap.group.clone(),
                index: gap.index,
                origin: at.along(gap.axis),
                before: (a, b),
                extent: gap.pair,
            });

            return;
        }

        let Some(pane) = solved.at(at) else {
            return;
        };

        let Some((tab, rect)) = self.tab_at(pane, at) else {
            return;
        };

        // The grip is the handle; the rest of a tab picks it. Keeping them
        // apart is what lets a stack be leafed through without anything being
        // picked up by accident.
        if Rect::new(rect.x, rect.y, self.metrics.grip, rect.h).contains(at) {
            self.gesture = Some(Gesture::Pressed {
                window: tab,
                origin: at,
            });

            return;
        }

        self.show(tab);
    }

    /// Bring a window to the front of its stack.
    fn show(&mut self, window: WindowId) {
        if let Some(root) = self.root.as_mut()
            && let Some(stack) = root.stack_mut(window)
        {
            stack.show(window);
        }
    }

    /// Which tab a point is on, and where that tab is.
    fn tab_at(&self, pane: &Pane, at: Point) -> Option<(WindowId, Rect)> {
        tabs(pane, &self.metrics).find(|(_, rect)| rect.contains(at))
    }

    fn drag(&mut self, at: Point) {
        match self.gesture.clone() {
            Some(Gesture::Pressed { window, origin }) => {
                if at.distance(origin) < self.metrics.threshold {
                    return;
                }

                let Some(rect) = self.solved().and_then(|solved| solved.rect(window)) else {
                    return;
                };

                self.gesture = Some(Gesture::Moving {
                    window,
                    grab: self.grab(origin, rect),
                    at,
                });
            }

            Some(Gesture::Moving { window, grab, .. }) => {
                self.gesture = Some(Gesture::Moving { window, grab, at });
            }

            Some(Gesture::Resizing {
                group,
                index,
                origin,
                before,
                extent,
            }) => {
                // A window under the pointer keeps up with it.
                self.settling = false;

                self.resize(&group, index, at, origin, before, extent);
            }

            None => {}
        }
    }

    fn release(&mut self, at: Point) {
        if let Some(Gesture::Moving { window, .. }) = self.gesture.clone()
            && let (_, Some(slot)) = self.targets(window, at)
        {
            self.root = self.dropped(window, slot);
            self.settling = true;
        }

        self.gesture = None;
    }

    /// Nothing decided. A resize is put back where it was, since half a drag
    /// is not a size anybody asked for.
    fn cancel(&mut self) {
        if let Some(Gesture::Resizing {
            group,
            index,
            before,
            ..
        }) = self.gesture.clone()
        {
            self.write(&group, index, before.0, before.1);
            self.settling = true;
        }

        self.gesture = None;
    }

    /// Move the boundary between two children. Fixed children are written in
    /// pixels and flexible ones in weight, so dragging a sidebar's edge
    /// changes its width and dragging between two flexible windows only
    /// changes how they share what they already had.
    fn resize(
        &mut self,
        group: &Path,
        index: usize,
        at: Point,
        origin: f32,
        before: (Share, Share),
        extent: (f32, f32),
    ) {
        let Some(axis) = self.axis(group) else {
            return;
        };

        let (a0, b0) = extent;
        let total = a0 + b0;
        let min = self.metrics.min.along(axis);

        if total < min * 2.0 {
            return;
        }

        let a = (a0 + at.along(axis) - origin).clamp(min, total - min);
        let b = total - a;

        let (next_a, next_b) = match before {
            (Share::Fixed(_), Share::Fixed(_)) => (Share::Fixed(a), Share::Fixed(b)),
            (Share::Fixed(_), flex) => (Share::Fixed(a), flex),
            (flex, Share::Fixed(_)) => (flex, Share::Fixed(b)),
            (Share::Flex(wa), Share::Flex(wb)) => {
                let weight = wa + wb;

                (
                    Share::Flex(weight * a / total),
                    Share::Flex(weight * b / total),
                )
            }
        };

        self.write(group, index, next_a, next_b);
    }

    /// The tree a drop would produce. One function, used by the preview and
    /// by the drop itself, so what is drawn and what happens cannot differ.
    fn dropped(&self, window: WindowId, slot: Slot) -> Option<Node> {
        let mut root = self.root.clone();

        // The window keeps the share it had: a sidebar dragged to the other
        // side of the dock is still the width somebody set it to.
        let share = remove(&mut root, window).unwrap_or(Share::Flex(1.0));

        place(&mut root, window, slot, share);

        root
    }

    /// Where a window being moved would land, and every region on offer where
    /// the pointer is.
    ///
    /// The two are worked out together and handed back together, so the shape
    /// drawn and the drop carried out cannot be about different things. The
    /// three targets the board draws, in the order they take precedence: the
    /// dock's edge, a gap, then the window under the pointer.
    fn targets(&self, moving: WindowId, at: Point) -> (Vec<Zone>, Option<Slot>) {
        let nothing = (Vec::new(), None);

        if !self.area().contains(at) {
            return nothing;
        }

        let Some(solved) = self.solved() else {
            return nothing;
        };

        let edge = self.metrics.edge;
        let area = self.area();

        let outer = [
            (at.x - area.x, Side::Left),
            (area.right() - at.x, Side::Right),
            (at.y - area.y, Side::Up),
            (area.bottom() - at.y, Side::Down),
        ]
        .into_iter()
        .filter(|(distance, _)| *distance < edge)
        .min_by(|(a, _), (b, _)| a.total_cmp(b));

        if let Some((_, side)) = outer {
            let zone = Zone::rect(strip(area, side, edge), true);

            return (vec![zone], Some(Slot::Edge { side }));
        }

        let gap = solved
            .gaps
            .iter()
            .find(|gap| gap.rect.grown(edge / 2.0).contains(at));

        if let Some(gap) = gap {
            let Some(before) = self.leaf(&gap.group, gap.index, moving) else {
                return nothing;
            };

            let side = match gap.axis {
                Axis::Row => Side::Right,
                Axis::Col => Side::Down,
            };

            if before == moving {
                return nothing;
            }

            let zone = Zone::rect(gap.rect.grown(edge / 2.0), true);

            return (
                vec![zone],
                Some(Slot::Beside {
                    window: before,
                    side,
                }),
            );
        }

        let Some(pane) = solved.at(at) else {
            return nothing;
        };

        // A window alone in its own slot has nowhere to go in it. One of
        // several does: leaving the stack to take a side of it is a move.
        let Some(anchor) = pane.windows.iter().copied().find(|held| *held != moving) else {
            return nothing;
        };

        let rect = pane.rect;
        let chosen = band(rect, at, self.metrics.band);
        let zones = regions(rect, self.metrics.band, chosen, !pane.holds(moving));

        let slot = match chosen {
            Some(side) => Some(Slot::Beside {
                window: anchor,
                side,
            }),
            // The middle is the stack. Dropping a window into the stack it is
            // already in would be nothing happening, so it is not offered.
            None if !pane.holds(moving) => Some(Slot::With { window: anchor }),
            None => None,
        };

        (zones, slot)
    }

    pub fn frame(&self) -> Frame {
        let Some(solved) = self.solved() else {
            return Frame {
                settling: self.settling,
                ..Frame::default()
            };
        };

        let moving = self.moving();

        let panes = solved
            .panes
            .iter()
            .map(|pane| PaneFrame {
                rect: pane.rect,
                tabs: tabs(pane, &self.metrics)
                    .map(|(window, rect)| TabFrame {
                        window,
                        title: self.title(window),
                        rect,
                        grip: Rect::new(rect.x, rect.y, self.metrics.grip, rect.h),
                        showing: window == pane.showing,
                        lifted: moving == Some(window),
                    })
                    .collect(),
            })
            .collect();

        let windows = solved
            .panes
            .iter()
            .flat_map(|pane| {
                pane.windows.iter().map(|window| WindowFrame {
                    window: *window,
                    rect: self.body(pane.rect),
                    showing: *window == pane.showing,
                    lifted: moving == Some(*window),
                })
            })
            .collect();

        let gaps = solved
            .gaps
            .iter()
            .map(|gap| GapFrame {
                rect: gap.rect,
                axis: gap.axis,
            })
            .collect();

        let (ghost, regions) = match &self.gesture {
            Some(Gesture::Moving { window, grab, at }) => {
                let size = self.ghost(*window, &solved);
                let ghost = Rect::new(at.x - grab.x, at.y - grab.y, size.w, size.h);

                (Some(ghost), self.targets(*window, *at).0)
            }

            _ => (None, Vec::new()),
        };

        Frame {
            panes,
            windows,
            gaps,
            ghost,
            regions,
            cursor: self.cursor(),
            settling: self.settling,
        }
    }

    fn cursor(&self) -> Cursor {
        match &self.gesture {
            Some(Gesture::Moving { .. }) => Cursor::Grabbing,
            Some(Gesture::Resizing { group, .. }) => match self.axis(group) {
                Some(Axis::Row) => Cursor::ColResize,
                Some(Axis::Col) => Cursor::RowResize,
                None => Cursor::Default,
            },
            _ => Cursor::Default,
        }
    }

    pub fn area(&self) -> Rect {
        Rect::new(0.0, 0.0, self.surface.w, self.surface.h)
    }

    fn solved(&self) -> Option<Solved> {
        let root = self.root.as_ref()?;

        Some(solve(root, self.area(), &self.metrics))
    }

    fn moving(&self) -> Option<WindowId> {
        match self.gesture {
            Some(Gesture::Moving { window, .. }) => Some(window),
            _ => None,
        }
    }

    fn title(&self, window: WindowId) -> String {
        self.windows
            .iter()
            .find(|open| open.id == window)
            .map(|open| open.title.clone())
            .unwrap_or_default()
    }

    /// What is left of a slot under its tabs.
    fn body(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.x,
            rect.y + self.metrics.tab,
            rect.w,
            (rect.h - self.metrics.tab).max(0.0),
        )
    }

    /// Where the cursor sits inside the ghost: the same place it had on the
    /// window, in proportion, so a window picked up by its grip is still held
    /// by its grip.
    fn grab(&self, at: Point, rect: Rect) -> Point {
        let size = self.capped(rect.size());

        Point::new(
            (at.x - rect.x) * size.w / rect.w.max(1.0),
            (at.y - rect.y) * size.h / rect.h.max(1.0),
        )
    }

    fn ghost(&self, window: WindowId, solved: &Solved) -> Size {
        match solved.rect(window) {
            Some(rect) => self.capped(rect.size()),
            None => self.metrics.ghost,
        }
    }

    fn capped(&self, size: Size) -> Size {
        let scale = (self.metrics.ghost.w / size.w)
            .min(self.metrics.ghost.h / size.h)
            .min(1.0);

        Size::new(size.w * scale, size.h * scale)
    }

    fn group(&self, path: &Path) -> Option<&Node> {
        let mut node = self.root.as_ref()?;

        for index in path {
            let Node::Group { children, .. } = node else {
                return None;
            };

            node = &children.get(*index)?.node;
        }

        Some(node)
    }

    fn axis(&self, path: &Path) -> Option<Axis> {
        match self.group(path)? {
            Node::Group { axis, .. } => Some(*axis),
            Node::Pane(_) => None,
        }
    }

    fn shares(&self, path: &Path, index: usize) -> Option<(Share, Share)> {
        let Node::Group { children, .. } = self.group(path)? else {
            return None;
        };

        Some((children.get(index)?.share, children.get(index + 1)?.share))
    }

    /// A window in the pane a gap's left or upper side ends in, which is the
    /// neighbour a drop on that gap lands beside. Never the window being
    /// moved, since that one is about to be taken out of the tree.
    fn leaf(&self, path: &Path, index: usize, moving: WindowId) -> Option<WindowId> {
        let Node::Group { children, .. } = self.group(path)? else {
            return None;
        };

        children.get(index)?.node.last().other(moving)
    }

    fn write(&mut self, path: &Path, index: usize, a: Share, b: Share) {
        let Some(root) = self.root.as_mut() else {
            return;
        };

        let mut node = root;

        for step in path {
            let Node::Group { children, .. } = node else {
                return;
            };

            let Some(child) = children.get_mut(*step) else {
                return;
            };

            node = &mut child.node;
        }

        let Node::Group { children, .. } = node else {
            return;
        };

        if let Some(child) = children.get_mut(index) {
            child.share = a;
        }

        if let Some(child) = children.get_mut(index + 1) {
            child.share = b;
        }
    }
}

/// Which band of a window a point is in, or nothing at all when it is in the
/// middle.
///
/// The distances are fractions of the width and the height, so comparing them
/// cuts the window along its diagonals — which is what makes a wide short
/// window behave like a tall narrow one, and what makes the four bands meet
/// the middle exactly at its corners.
fn band(rect: Rect, at: Point, band: f32) -> Option<Side> {
    let left = (at.x - rect.x) / rect.w.max(1.0);
    let up = (at.y - rect.y) / rect.h.max(1.0);

    let (near, side) = [
        (left, Side::Left),
        (1.0 - left, Side::Right),
        (up, Side::Up),
        (1.0 - up, Side::Down),
    ]
    .into_iter()
    .min_by(|(a, _), (b, _)| a.total_cmp(b))?;

    // The middle is the room tabs will want. Until they have it, being there
    // means no drop rather than the nearest one.
    (near < band).then_some(side)
}

/// Everything a window is divided into: the four bands and the middle, in a
/// fixed order so that a region keeps its place in the list while the pointer
/// moves between them.
fn regions(rect: Rect, band: f32, chosen: Option<Side>, stackable: bool) -> Vec<Zone> {
    let sides = [Side::Left, Side::Up, Side::Right, Side::Down];

    let mut zones: Vec<Zone> = sides
        .into_iter()
        .map(|side| trapezium(rect, side, band, chosen == Some(side)))
        .collect();

    let middle = Rect::new(
        rect.x + rect.w * band,
        rect.y + rect.h * band,
        rect.w * (1.0 - band * 2.0),
        rect.h * (1.0 - band * 2.0),
    );

    // The middle is the stack: a window dropped there joins this one rather
    // than taking a side of it.
    zones.push(Zone::rect(middle, stackable && chosen.is_none()));

    zones
}

/// The band itself: the wedge between two diagonals with the middle taken
/// out, which is a trapezium with its short side against the middle.
fn trapezium(rect: Rect, side: Side, band: f32, chosen: bool) -> Zone {
    let x = rect.w * band;
    let y = rect.h * band;

    let (left, top) = (rect.x, rect.y);
    let (right, bottom) = (rect.right(), rect.bottom());

    let points = match side {
        Side::Left => [
            Point::new(left, top),
            Point::new(left + x, top + y),
            Point::new(left + x, bottom - y),
            Point::new(left, bottom),
        ],
        Side::Right => [
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(right - x, bottom - y),
            Point::new(right - x, top + y),
        ],
        Side::Up => [
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right - x, top + y),
            Point::new(left + x, top + y),
        ],
        Side::Down => [
            Point::new(left, bottom),
            Point::new(left + x, bottom - y),
            Point::new(right - x, bottom - y),
            Point::new(right, bottom),
        ],
    };

    Zone { points, chosen }
}

/// The strip along one side of the dock.
fn strip(area: Rect, side: Side, edge: f32) -> Rect {
    match side {
        Side::Left => Rect::new(area.x, area.y, edge, area.h),
        Side::Right => Rect::new(area.right() - edge, area.y, edge, area.h),
        Side::Up => Rect::new(area.x, area.y, area.w, edge),
        Side::Down => Rect::new(area.x, area.bottom() - edge, area.w, edge),
    }
}

/// Where a slot's tabs are. They divide the top of it, up to a width past
/// which a tab is wider than its name needs.
fn tabs(pane: &Pane, metrics: &Metrics) -> impl Iterator<Item = (WindowId, Rect)> {
    let count = pane.windows.len().max(1) as f32;
    let width = (pane.rect.w / count).min(metrics.tab_width);
    let rect = pane.rect;
    let tab = metrics.tab;

    pane.windows
        .clone()
        .into_iter()
        .enumerate()
        .map(move |(i, window)| {
            (
                window,
                Rect::new(rect.x + width * i as f32, rect.y, width, tab),
            )
        })
}
