//! Points and rectangles, and the two directions a group can run in.

/// A point on the surface, in logical pixels from its top left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// How far it is along an axis, which is the only thing a resize cares
    /// about.
    pub fn along(self, axis: Axis) -> f32 {
        match axis {
            Axis::Row => self.x,
            Axis::Col => self.y,
        }
    }

    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

impl Size {
    pub fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }

    pub fn along(self, axis: Axis) -> f32 {
        match axis {
            Axis::Row => self.w,
            Axis::Col => self.h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn size(self) -> Size {
        Size::new(self.w, self.h)
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// Grown by `by` on every side. What makes a five pixel gap something a
    /// drag can actually land on.
    pub fn grown(self, by: f32) -> Self {
        Self::new(
            self.x - by,
            self.y - by,
            self.w + by * 2.0,
            self.h + by * 2.0,
        )
    }

    pub fn extent(self, axis: Axis) -> f32 {
        self.size().along(axis)
    }

    pub fn origin(self, axis: Axis) -> f32 {
        Point::new(self.x, self.y).along(axis)
    }

    /// The same rectangle with one axis replaced, which is how a group hands
    /// a slice of itself to a child.
    pub fn slice(self, axis: Axis, origin: f32, extent: f32) -> Self {
        match axis {
            Axis::Row => Self::new(origin, self.y, extent, self.h),
            Axis::Col => Self::new(self.x, origin, self.w, extent),
        }
    }
}

/// Which way a group's children are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Side by side, so the gaps between them are vertical.
    Row,
    /// One above the next.
    Col,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Up,
    Down,
}

impl Side {
    pub fn axis(self) -> Axis {
        match self {
            Side::Left | Side::Right => Axis::Row,
            Side::Up | Side::Down => Axis::Col,
        }
    }

    /// Does something dropped on this side come before what is already there?
    pub fn leading(self) -> bool {
        matches!(self, Side::Left | Side::Up)
    }
}
