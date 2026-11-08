//! The layout: what windows there are and how they divide the surface.
//!
//! Two rules keep the tree canonical, and they run after every change: a
//! group with one child is not a group, and a group whose axis matches its
//! parent's is folded into it. Without them a week of dragging leaves a tree
//! of wrappers nobody can see and every resize has to walk through.

use crate::geometry::Axis;
use crate::geometry::Side;

/// A window, as far as the dock is concerned: a number the host chose. What
/// is inside one is the host's business.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// How much of its group a child takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Share {
    /// A weight. The flexible children divide whatever the fixed ones leave.
    Flex(f32),
    /// Logical pixels, kept across a resize of the surface.
    Fixed(f32),
}

/// The windows sharing one slot, one of them showing.
///
/// A slot holds a stack rather than a window because a dock that can only
/// tile runs out of room: six windows on a laptop screen is six windows too
/// narrow to use. The stack is the answer to "where does the next one go"
/// when there is no room left to divide.
#[derive(Clone, Debug)]
pub struct Stack {
    windows: Vec<WindowId>,
    showing: usize,
}

impl Stack {
    pub fn new(window: WindowId) -> Self {
        Self {
            windows: vec![window],
            showing: 0,
        }
    }

    pub fn windows(&self) -> &[WindowId] {
        &self.windows
    }

    /// The one being drawn. A stack is never empty — it is taken out of the
    /// tree when its last window leaves — so there is always one.
    pub fn showing(&self) -> WindowId {
        match self.windows.get(self.showing) {
            Some(window) => *window,
            None => unreachable!("a stack is never empty and always shows one"),
        }
    }

    pub fn holds(&self, window: WindowId) -> bool {
        self.windows.contains(&window)
    }

    /// A window in it that is not this one, which is what a drop needs to name
    /// a pane it is landing beside.
    pub fn other(&self, window: WindowId) -> Option<WindowId> {
        self.windows.iter().copied().find(|held| *held != window)
    }

    pub fn show(&mut self, window: WindowId) {
        if let Some(at) = self.windows.iter().position(|held| *held == window) {
            self.showing = at;
        }
    }

    /// Arriving in a stack means arriving in front: it was just dropped here,
    /// so it is what the drop was for.
    fn take(&mut self, window: WindowId) {
        if self.holds(window) {
            self.show(window);

            return;
        }

        self.windows.push(window);
        self.showing = self.windows.len() - 1;
    }

    /// Whether the stack is left empty, in which case its slot goes with it.
    fn drop(&mut self, window: WindowId) -> bool {
        self.windows.retain(|held| *held != window);
        self.showing = self.showing.min(self.windows.len().saturating_sub(1));

        self.windows.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct Child {
    pub share: Share,
    pub node: Node,
}

#[derive(Clone, Debug)]
pub enum Node {
    Pane(Stack),
    Group { axis: Axis, children: Vec<Child> },
}

/// Where in the tree something is: the child index at each level from the
/// root. Empty is the root itself.
pub type Path = Vec<usize>;

/// Where a window being moved would land.
///
/// Three of them, for the four targets the board draws: a window's band and a
/// gap are both *beside a pane*, because a group nested in a group of the
/// same axis is folded into it and the two drops end up the same tree. The
/// dock's outer edge is not, which is why it is the only way to get back a
/// window spanning the whole height. A window's middle is a stack.
///
/// They name a window rather than a path on purpose: a path stops meaning
/// what it meant the moment the window being moved is taken out of the tree,
/// and a drop has to be worked out before that and carried out after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    Beside { window: WindowId, side: Side },
    Edge { side: Side },
    With { window: WindowId },
}

impl Node {
    /// Where a window is, as a path from the root to the pane holding it.
    pub fn find(&self, window: WindowId) -> Option<Path> {
        let mut path = Path::new();

        self.find_into(window, &mut path).then_some(path)
    }

    fn find_into(&self, window: WindowId, path: &mut Path) -> bool {
        match self {
            Node::Pane(stack) => stack.holds(window),
            Node::Group { children, .. } => {
                for (i, child) in children.iter().enumerate() {
                    path.push(i);

                    if child.node.find_into(window, path) {
                        return true;
                    }

                    let _ = path.pop();
                }

                false
            }
        }
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        match path.split_first() {
            None => Some(self),
            Some((i, rest)) => match self {
                Node::Pane(_) => None,
                Node::Group { children, .. } => children.get_mut(*i)?.node.at_mut(rest),
            },
        }
    }

    pub fn stack_mut(&mut self, window: WindowId) -> Option<&mut Stack> {
        let path = self.find(window)?;

        match self.at_mut(&path)? {
            Node::Pane(stack) => Some(stack),
            Node::Group { .. } => None,
        }
    }

    /// The last pane under this node, which is what a gap uses to name the
    /// neighbour a drop lands beside.
    pub fn last(&self) -> &Stack {
        match self {
            Node::Pane(stack) => stack,
            Node::Group { children, .. } => match children.last() {
                Some(child) => child.node.last(),
                None => unreachable!("a group always has children"),
            },
        }
    }
}

/// Put a window where a slot says, with the share it takes of whatever group
/// it lands in. The tree may be empty, in which case the window becomes the
/// whole of it and the slot does not matter.
pub fn place(root: &mut Option<Node>, window: WindowId, slot: Slot, share: Share) {
    let Some(node) = root.as_mut() else {
        *root = Some(Node::Pane(Stack::new(window)));

        return;
    };

    match slot {
        Slot::Edge { side } => place_edge(node, window, side, share),
        Slot::Beside {
            window: target,
            side,
        } => place_beside(node, window, target, side, share),
        Slot::With { window: target } => {
            if let Some(stack) = node.stack_mut(target) {
                stack.take(window);
            }
        }
    }

    // A slot names a window, and a window can have been closed between a drop
    // being worked out and carried out. Putting it on the end is wrong; losing
    // it is worse.
    if node.find(window).is_none() {
        place_edge(node, window, Side::Right, share);
    }

    normalize(node);
}

/// Along the whole of one side of the dock, which is the only drop that can
/// reach past the group a window is in.
fn place_edge(root: &mut Node, window: WindowId, side: Side, share: Share) {
    let fits = matches!(root, Node::Group { axis, .. } if *axis == side.axis());

    if fits {
        let Node::Group { children, .. } = root else {
            unreachable!("just matched a group");
        };

        let at = if side.leading() { 0 } else { children.len() };

        children.insert(
            at,
            Child {
                share,
                node: Node::Pane(Stack::new(window)),
            },
        );
    } else {
        wrap(root, window, side, share);
    }
}

fn place_beside(root: &mut Node, window: WindowId, target: WindowId, side: Side, share: Share) {
    let Some(path) = root.find(target) else {
        return;
    };

    let Some((index, parent_path)) = path.split_last() else {
        // The target is the whole tree, so there is no group to join.
        wrap(root, window, side, share);

        return;
    };

    let Some(parent) = root.at_mut(parent_path) else {
        return;
    };

    let Node::Group { axis, children } = parent else {
        unreachable!("a path with a parent leads through a group");
    };

    if *axis == side.axis() {
        let at = if side.leading() { *index } else { index + 1 };

        children.insert(
            at,
            Child {
                share,
                node: Node::Pane(Stack::new(window)),
            },
        );
    } else {
        let Some(child) = children.get_mut(*index) else {
            return;
        };

        wrap(&mut child.node, window, side, share);
    }
}

/// Turn a node into a group of two along a new axis: the node, and the window
/// arriving beside it.
fn wrap(node: &mut Node, window: WindowId, side: Side, share: Share) {
    let taken = std::mem::replace(node, Node::Pane(Stack::new(window)));

    let arriving = Child {
        share,
        node: Node::Pane(Stack::new(window)),
    };
    let existing = Child {
        share: Share::Flex(1.0),
        node: taken,
    };

    let children = if side.leading() {
        vec![arriving, existing]
    } else {
        vec![existing, arriving]
    };

    *node = Node::Group {
        axis: side.axis(),
        children,
    };
}

/// Take a window out. Its slot goes with it when it was the last window in
/// it, and the tree is left canonical — so removing the second to last child
/// of a group dissolves the group.
pub fn remove(root: &mut Option<Node>, window: WindowId) -> Option<Share> {
    let node = root.as_mut()?;
    let path = node.find(window)?;

    let Some(Node::Pane(stack)) = node.at_mut(&path) else {
        return None;
    };

    if !stack.drop(window) {
        return None;
    }

    let Some((index, parent_path)) = path.split_last() else {
        *root = None;

        return None;
    };

    let Node::Group { children, .. } = node.at_mut(parent_path)? else {
        return None;
    };

    let share = children.remove(*index).share;

    normalize(node);

    Some(share)
}

/// The two rules that keep a tree canonical, applied bottom up.
pub fn normalize(node: &mut Node) {
    let Node::Group { axis, children } = node else {
        return;
    };

    for child in children.iter_mut() {
        normalize(&mut child.node);
    }

    let axis = *axis;
    let mut folded: Vec<Child> = Vec::with_capacity(children.len());

    for child in children.drain(..) {
        match child.node {
            Node::Group {
                axis: inner,
                children: grandchildren,
            } if inner == axis => folded.extend(rescale(grandchildren, child.share)),
            node => folded.push(Child {
                share: child.share,
                node,
            }),
        }
    }

    *children = folded;

    if children.len() == 1 {
        let Some(only) = children.pop() else {
            unreachable!("just checked the length");
        };

        // The share belongs to the slot, and the slot is the parent's.
        *node = only.node;
    }
}

/// Fit a folded group's children into the share the group itself had, so that
/// folding changes the tree and not the picture.
fn rescale(children: Vec<Child>, share: Share) -> Vec<Child> {
    let flex: f32 = children
        .iter()
        .filter_map(|child| match child.share {
            Share::Flex(weight) => Some(weight),
            Share::Fixed(_) => None,
        })
        .sum();

    match share {
        Share::Flex(outer) if flex > 0.0 => children
            .into_iter()
            .map(|child| Child {
                share: match child.share {
                    Share::Flex(weight) => Share::Flex(weight * outer / flex),
                    fixed => fixed,
                },
                node: child.node,
            })
            .collect(),

        // A fixed group has pixels to divide rather than a weight, so its
        // flexible children become fixed and the group keeps its width.
        Share::Fixed(pixels) => {
            let taken: f32 = children
                .iter()
                .filter_map(|child| match child.share {
                    Share::Fixed(px) => Some(px),
                    Share::Flex(_) => None,
                })
                .sum();
            let rest = (pixels - taken).max(0.0);

            children
                .into_iter()
                .map(|child| Child {
                    share: match child.share {
                        Share::Flex(weight) if flex > 0.0 => Share::Fixed(rest * weight / flex),
                        other => other,
                    },
                    node: child.node,
                })
                .collect()
        }

        _ => children,
    }
}
