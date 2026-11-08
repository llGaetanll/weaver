//! The tree and a rectangle, into rectangles.
//!
//! Nothing here is stored. A rectangle that is kept is a rectangle that can
//! disagree with the tree it came from, and the surface changes size often
//! enough that it would.

use crate::geometry::Axis;
use crate::geometry::Rect;
use crate::metrics::Metrics;
use crate::tree::Child;
use crate::tree::Node;
use crate::tree::Path;
use crate::tree::Share;
use crate::tree::WindowId;

/// One gap, and the two children it divides.
#[derive(Clone, Debug)]
pub struct Gap {
    pub group: Path,
    /// The gap after this child, so it sits between `index` and `index + 1`.
    pub index: usize,
    pub axis: Axis,
    pub rect: Rect,
    /// What the two children either side of it measure right now, which is
    /// where a resize starts counting from.
    pub pair: (f32, f32),
}

#[derive(Clone, Debug, Default)]
pub struct Solved {
    pub windows: Vec<(WindowId, Rect)>,
    pub gaps: Vec<Gap>,
}

impl Solved {
    pub fn rect(&self, window: WindowId) -> Option<Rect> {
        self.windows
            .iter()
            .find(|(id, _)| *id == window)
            .map(|(_, rect)| *rect)
    }

    /// The window under a point, if any.
    pub fn at(&self, p: crate::geometry::Point) -> Option<(WindowId, Rect)> {
        self.windows
            .iter()
            .find(|(_, rect)| rect.contains(p))
            .copied()
    }
}

pub fn solve(root: &Node, area: Rect, metrics: &Metrics) -> Solved {
    let mut solved = Solved::default();
    let mut path = Path::new();

    walk(root, area, metrics, &mut path, &mut solved);

    solved
}

fn walk(node: &Node, area: Rect, metrics: &Metrics, path: &mut Path, out: &mut Solved) {
    let Node::Group { axis, children } = node else {
        let Node::Window(id) = node else {
            unreachable!("a node is a window or a group");
        };

        out.windows.push((*id, area));

        return;
    };

    let axis = *axis;
    let extents = divide(children, area.extent(axis), metrics, axis);
    let mut at = area.origin(axis);

    for (i, (child, extent)) in children.iter().zip(&extents).enumerate() {
        if i > 0 {
            out.gaps.push(Gap {
                group: path.clone(),
                index: i - 1,
                axis,
                rect: area.slice(axis, at, metrics.gap),
                pair: (extents[i - 1], *extent),
            });

            at += metrics.gap;
        }

        path.push(i);
        walk(&child.node, area.slice(axis, at, *extent), metrics, path, out);
        let _ = path.pop();

        at += extent;
    }
}

/// How much of an extent each child gets. The fixed children take their
/// pixels, the flexible ones divide what is left, and a child never goes
/// under the minimum — which is why the dock can end up larger than the
/// surface rather than crushing a window to a title bar.
fn divide(children: &[Child], extent: f32, metrics: &Metrics, axis: Axis) -> Vec<f32> {
    let gaps = metrics.gap * children.len().saturating_sub(1) as f32;
    let room = (extent - gaps).max(0.0);

    let fixed: f32 = children
        .iter()
        .filter_map(|child| match child.share {
            Share::Fixed(px) => Some(px),
            Share::Flex(_) => None,
        })
        .sum();

    let weight: f32 = children
        .iter()
        .filter_map(|child| match child.share {
            Share::Flex(w) => Some(w),
            Share::Fixed(_) => None,
        })
        .sum();

    let rest = (room - fixed).max(0.0);
    let min = metrics.min.along(axis);

    children
        .iter()
        .map(|child| match child.share {
            Share::Fixed(px) => px.max(min),
            Share::Flex(w) if weight > 0.0 => (rest * w / weight).max(min),
            Share::Flex(_) => min,
        })
        .collect()
}
