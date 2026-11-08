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

#[derive(Clone, Debug)]
pub struct Child {
    pub share: Share,
    pub node: Node,
}

#[derive(Clone, Debug)]
pub enum Node {
    Window(WindowId),
    Group { axis: Axis, children: Vec<Child> },
}

/// Where in the tree something is: the child index at each level from the
/// root. Empty is the root itself.
pub type Path = Vec<usize>;

/// Where a window being moved would land.
///
/// Two of them, for the three targets the board draws: a window's edge and a
/// gap are both *beside a window*, because a group nested in a group of the
/// same axis is folded into it and the two drops end up the same tree. The
/// dock's outer edge is not, which is why it is the only way to get back a
/// window spanning the whole height.
///
/// It names a window rather than a path on purpose: a path stops meaning what
/// it meant the moment the window being moved is taken out of the tree, and a
/// drop has to be worked out before that and carried out after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    Beside { window: WindowId, side: Side },
    Edge { side: Side },
}

impl Node {
    /// Where a window is, as a path from the root.
    pub fn find(&self, window: WindowId) -> Option<Path> {
        let mut path = Path::new();

        self.find_into(window, &mut path).then_some(path)
    }

    fn find_into(&self, window: WindowId, path: &mut Path) -> bool {
        match self {
            Node::Window(id) => *id == window,
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

    fn at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        match path.split_first() {
            None => Some(self),
            Some((i, rest)) => match self {
                Node::Window(_) => None,
                Node::Group { children, .. } => children.get_mut(*i)?.node.at_mut(rest),
            },
        }
    }

    /// The last window under this node, which is what a gap uses to name the
    /// neighbour a drop lands beside.
    pub fn last(&self) -> WindowId {
        match self {
            Node::Window(id) => *id,
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
        *root = Some(Node::Window(window));

        return;
    };

    match slot {
        Slot::Edge { side } => place_edge(node, window, side, share),
        Slot::Beside {
            window: target,
            side,
        } => place_beside(node, window, target, side, share),
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
                node: Node::Window(window),
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
                node: Node::Window(window),
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
    let taken = std::mem::replace(node, Node::Window(window));

    let arriving = Child {
        share,
        node: Node::Window(window),
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

/// Take a window out. The tree is left canonical, so removing the second to
/// last child of a group dissolves the group.
pub fn remove(root: &mut Option<Node>, window: WindowId) -> Option<Share> {
    let node = root.as_mut()?;
    let path = node.find(window)?;

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
