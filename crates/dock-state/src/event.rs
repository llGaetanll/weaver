//! The whole vocabulary of things that happen to a dock.

use crate::event_host::EventHost;
use crate::event_pointer::EventPointer;

#[derive(Clone, Debug)]
pub enum Event {
    FromPointer(EventPointer),
    FromHost(EventHost),
}
