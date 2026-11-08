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
use crate::solve::Gap;
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
    /// A tab is held but has not travelled far enough to be a move.
    Pressed { window: WindowId, origin: Point },

    Moving {
        window: WindowId,
        /// Where in the tab the cursor took hold of it, and how big that tab
        /// was. What is carried is the tab: it is what was picked up, and it
        /// is the shape of the hole it will go back into.
        grab: Point,
        size: Size,
        at: Point,
    },

    /// Every gap under the pointer: one along a line, and at a crossing the
    /// two or three that meet there. Never empty — a gesture with nothing
    /// held is not started.
    Resizing { held: Vec<Held> },
}

/// A gap being dragged, and what it divided when the drag started.
///
/// Counting from the start rather than from the last position is what stops
/// a drag past a minimum and back from drifting.
#[derive(Clone, Debug)]
struct Held {
    group: Path,
    index: usize,
    axis: Axis,
    origin: f32,
    before: (Share, Share),
    extent: (f32, f32),
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

    /// A press lands on a gap, a tab, or nothing at all. A gap starts
    /// resizing at once — there is nothing else it could mean — while a tab
    /// waits to see whether the pointer travels, so that a press which goes
    /// nowhere has only brought a window forward.
    fn press(&mut self, at: Point) {
        let Some(solved) = self.solved() else {
            return;
        };

        // Every gap near the pointer is taken hold of, so the corner where
        // four windows meet moves as one thing. A gap is grown in both
        // directions to find it: a crossing is where one gap's end meets
        // another's side, and the ends are exactly what thickening leaves
        // alone.
        let held: Vec<Held> = solved
            .gaps
            .iter()
            .filter(|gap| gap.rect.grown(self.metrics.grab).contains(at))
            .filter_map(|gap| self.hold(gap, at))
            .collect();

        if !held.is_empty() {
            self.gesture = Some(Gesture::Resizing { held });

            return;
        }

        let Some(pane) = solved.at(at) else {
            return;
        };

        let Some((tab, _)) = self.tab_at(pane, at) else {
            return;
        };

        // A tab is picked by pressing it and moved by dragging it, and both
        // are the same press: picking happens at once, and whether it was
        // also a drag is answered by whether the pointer goes anywhere.
        self.show(tab);

        self.gesture = Some(Gesture::Pressed {
            window: tab,
            origin: at,
        });
    }

    /// What a gap divides, written down so a drag can be measured from it.
    fn hold(&self, gap: &Gap, at: Point) -> Option<Held> {
        let (a, b) = self.shares(&gap.group, gap.index)?;

        Some(Held {
            group: gap.group.clone(),
            index: gap.index,
            axis: gap.axis,
            origin: at.along(gap.axis),
            before: (a, b),
            extent: gap.pair,
        })
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
        tabs(&pane.windows, pane.rect, &self.metrics)
            .into_iter()
            .find(|(_, rect)| rect.contains(at))
    }

    fn drag(&mut self, at: Point) {
        match self.gesture.clone() {
            Some(Gesture::Pressed { window, origin }) => {
                if at.distance(origin) < self.metrics.threshold {
                    return;
                }

                let Some(tab) = self.tab(window) else {
                    return;
                };

                self.gesture = Some(Gesture::Moving {
                    window,
                    grab: Point::new(origin.x - tab.x, origin.y - tab.y),
                    size: tab.size(),
                    at,
                });
            }

            Some(Gesture::Moving {
                window, grab, size, ..
            }) => {
                self.gesture = Some(Gesture::Moving {
                    window,
                    grab,
                    size,
                    at,
                });
            }

            Some(Gesture::Resizing { held }) => {
                // A window under the pointer keeps up with it.
                self.settling = false;

                for held in &held {
                    self.resize(held, at);
                }
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
        if let Some(Gesture::Resizing { held }) = self.gesture.clone() {
            for held in &held {
                self.write(&held.group, held.index, held.before.0, held.before.1);
            }

            self.settling = true;
        }

        self.gesture = None;
    }

    /// Move the boundary between two children. Fixed children are written in
    /// pixels and flexible ones in weight, so dragging a sidebar's edge
    /// changes its width and dragging between two flexible windows only
    /// changes how they share what they already had.
    fn resize(&mut self, held: &Held, at: Point) {
        let Held {
            group,
            index,
            axis,
            origin,
            before,
            extent,
        } = held;

        let (axis, index, origin) = (*axis, *index, *origin);
        let (a0, b0) = *extent;
        let total = a0 + b0;
        let min = self.metrics.min.along(axis);

        if total < min * 2.0 {
            return;
        }

        let a = (a0 + at.along(axis) - origin).clamp(min, total - min);
        let b = total - a;

        let (next_a, next_b) = match *before {
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

        // The tabs are where the order is decided, so a window carried over
        // them lands between two of them rather than in the window under
        // them. They are looked at before the dock's own edge, because a band
        // nobody can see must not swallow a control everybody can.
        if let Some(pane) = solved.at(at)
            && at.y < pane.rect.y + self.metrics.tab
        {
            return self.ordering(pane, at, moving);
        }

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
            .find(|gap| gap.rect.thickened(gap.axis, edge).contains(at));

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

            let zone = Zone::rect(gap.rect.thickened(gap.axis, edge), true);

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
        // several does: leaving the stack to take a side of it is a move, and
        // so is going somewhere else in the order.
        let Some(anchor) = pane.windows.iter().copied().find(|held| *held != moving) else {
            return nothing;
        };

        let rect = self.body(pane.rect);
        let chosen = band(rect, at, self.metrics.band);
        let zones = regions(rect, self.metrics.band, chosen, !pane.holds(moving));

        let slot = match chosen {
            Some(side) => Some(Slot::Beside {
                window: anchor,
                side,
            }),
            // The middle is the stack, and a window joining one goes on the
            // end of it: nothing was aimed at, so nothing is displaced.
            None if !pane.holds(moving) => pane
                .windows
                .last()
                .map(|last| Slot::With {
                    window: *last,
                    before: false,
                }),
            None => None,
        };

        (zones, slot)
    }

    /// Where in a slot's order a window carried over its tabs would land.
    ///
    /// The place is worked out from where the pointer is and nothing else —
    /// which tab is under it would do just as well until the tabs move to
    /// make room, and then the two would chase each other round. Every tab in
    /// a slot is the same width, so the pointer being a third of the way
    /// along means the third place, whatever happens to be drawn there.
    fn ordering(&self, pane: &Pane, at: Point, moving: WindowId) -> (Vec<Zone>, Option<Slot>) {
        let others: Vec<WindowId> = pane
            .windows
            .iter()
            .copied()
            .filter(|held| *held != moving)
            .collect();

        let Some(first) = others.first().copied() else {
            return (Vec::new(), None);
        };

        let width = (pane.rect.w / (others.len() + 1) as f32).min(self.metrics.tab_width);
        let place = ((at.x - pane.rect.x) / width.max(1.0)).floor().max(0.0) as usize;

        let slot = match place.min(others.len()) {
            0 => Slot::With {
                window: first,
                before: true,
            },
            place => Slot::With {
                window: others[place - 1],
                before: false,
            },
        };

        // Nothing is drawn: the tabs move apart to leave the hole it would go
        // in, and the tab under the cursor is the other half of saying so.
        (Vec::new(), Some(slot))
    }

    /// The order a slot's tabs are drawn in, which is the order they would be
    /// in if the window being carried were let go of now. It leaves the slot
    /// it came from and takes its place in the one it is over, so the hole is
    /// always where the drop would land.
    fn order(&self, pane: &Pane, moving: Option<WindowId>, aim: Option<Slot>) -> Vec<WindowId> {
        let Some(moving) = moving else {
            return pane.windows.clone();
        };

        let mut order: Vec<WindowId> = pane
            .windows
            .iter()
            .copied()
            .filter(|held| *held != moving)
            .collect();

        if let Some(Slot::With { window, before }) = aim
            && let Some(at) = order.iter().position(|held| *held == window)
        {
            order.insert(if before { at } else { at + 1 }, moving);
        }

        order
    }

    pub fn frame(&self) -> Frame {
        let Some(solved) = self.solved() else {
            return Frame {
                settling: self.settling,
                ..Frame::default()
            };
        };

        let moving = self.moving();

        let (ghost, regions, aim) = match &self.gesture {
            Some(Gesture::Moving {
                window,
                grab,
                size,
                at,
            }) => {
                let (regions, aim) = self.targets(*window, *at);

                (Some(self.carried(*at, *grab, *size, &solved)), regions, aim)
            }

            _ => (None, Vec::new(), None),
        };

        let panes = solved
            .panes
            .iter()
            .map(|pane| PaneFrame {
                rect: pane.rect,
                tabs: tabs(&self.order(pane, moving, aim), pane.rect, &self.metrics)
                    .into_iter()
                    .map(|(window, rect)| TabFrame {
                        window,
                        title: self.title(window),
                        rect,
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
            Some(Gesture::Resizing { held }) => match held.first().map(|held| held.axis) {
                _ if held.iter().any(|one| Some(one.axis) != held.first().map(|f| f.axis)) => {
                    Cursor::BothResize
                }
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

    /// Where the tab under the cursor is drawn.
    ///
    /// Over a slot's tabs it sticks to the strip: somebody putting a tab in
    /// order is not also aiming it a few pixels up or down, and a tab that
    /// wanders out of line while being put in line is a tab fighting what it
    /// is being asked to do. Anywhere else it follows the pointer exactly,
    /// which is what carrying one out of a slot needs.
    fn carried(&self, at: Point, grab: Point, size: Size, solved: &Solved) -> Rect {
        let loose = Rect::new(at.x - grab.x, at.y - grab.y, size.w, size.h);

        let Some(pane) = solved.at(at) else {
            return loose;
        };

        if at.y >= pane.rect.y + self.metrics.tab {
            return loose;
        }

        let last = (pane.rect.right() - size.w).max(pane.rect.x);

        Rect::new(loose.x.clamp(pane.rect.x, last), pane.rect.y, size.w, size.h)
    }

    /// Where a window's tab is right now.
    fn tab(&self, window: WindowId) -> Option<Rect> {
        let solved = self.solved()?;
        let pane = solved.pane(window)?;

        tabs(&pane.windows, pane.rect, &self.metrics)
            .into_iter()
            .find(|(held, _)| *held == window)
            .map(|(_, rect)| rect)
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

/// Where a slot's tabs are, given the order they are in. They divide the top
/// of it, up to a width past which a tab is wider than its name needs.
fn tabs(windows: &[WindowId], rect: Rect, metrics: &Metrics) -> Vec<(WindowId, Rect)> {
    let count = windows.len().max(1) as f32;
    let width = (rect.w / count).min(metrics.tab_width);

    windows
        .iter()
        .enumerate()
        .map(|(i, window)| {
            (
                *window,
                Rect::new(rect.x + width * i as f32, rect.y, width, metrics.tab),
            )
        })
        .collect()
}
