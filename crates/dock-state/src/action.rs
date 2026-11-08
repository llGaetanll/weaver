//! What the dock asks the world to do.
//!
//! **Empty, on purpose.** A dock decides where rectangles go, and every
//! rectangle is derived — so there is nothing yet it has to ask for, and
//! nothing it could ask for twice. Writing the layout down is the first thing
//! that will live here; it is left out until the format and when to write are
//! decided rather than guessed.

#[derive(Clone, Debug)]
pub enum Action {}
