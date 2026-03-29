//! EventSink — a cheap, cloneable write handle for emitting events into a shared buffer.
//!
//! Subsystems (UI, physics, etc.) store an `EventSink` and call `emit()` to produce events.
//! The engine's `EventBus` owns the read side and drains all sinks during `flush()`.
//!
//! ```ignore
//! let sink = bus.create_sink();
//! sink.emit(MyEvent { score: 100 });
//! // Later, bus.flush() collects this event and fires registered handlers.
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A shared buffer that accumulates type-erased events.
///
/// Created by `EventSink::new()`. The sink and the bus both hold an `Rc` to the same buffer.
#[derive(Default)]
struct EventBuffer {
    events: Vec<Box<dyn Any>>,
}

/// A cheap, cloneable handle for emitting events into a shared buffer.
///
/// Multiple clones write to the same underlying buffer. The `EventBus` drains
/// all registered sinks during `flush()`.
#[derive(Clone)]
pub struct EventSink {
    buffer: Rc<RefCell<EventBuffer>>,
}

impl EventSink {
    /// Create a new event sink with an empty buffer.
    pub fn new() -> Self {
        Self {
            buffer: Rc::new(RefCell::new(EventBuffer::default())),
        }
    }

    /// Emit an event into the shared buffer.
    ///
    /// The event is stored as a type-erased `Box<dyn Any>` and will be downcast
    /// by the `EventBus` during flush.
    pub fn emit<T: 'static>(&self, event: T) {
        self.buffer.borrow_mut().events.push(Box::new(event));
    }

    /// Drain all pending events from the buffer.
    ///
    /// Returns the events and leaves the buffer empty. Called by the `EventBus`
    /// during flush.
    pub fn drain(&self) -> Vec<Box<dyn Any>> {
        std::mem::take(&mut self.buffer.borrow_mut().events)
    }

    /// Returns true if there are no pending events.
    pub fn is_empty(&self) -> bool {
        self.buffer.borrow().events.is_empty()
    }
}

impl Default for EventSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sink_emit_and_drain() {
        let sink = EventSink::new();
        sink.emit(42u32);
        sink.emit(99u32);

        let events = sink.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(*events[0].downcast_ref::<u32>().unwrap(), 42);
        assert_eq!(*events[1].downcast_ref::<u32>().unwrap(), 99);
    }

    #[test]
    fn test_sink_clone_shares_buffer() {
        let sink1 = EventSink::new();
        let sink2 = sink1.clone();

        sink1.emit(1u32);
        sink2.emit(2u32);

        let events = sink1.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(*events[0].downcast_ref::<u32>().unwrap(), 1);
        assert_eq!(*events[1].downcast_ref::<u32>().unwrap(), 2);
    }

    #[test]
    fn test_sink_drain_clears() {
        let sink = EventSink::new();
        sink.emit(42u32);

        let events = sink.drain();
        assert_eq!(events.len(), 1);

        let events2 = sink.drain();
        assert!(events2.is_empty());
    }

    #[test]
    fn test_sink_multiple_types() {
        let sink = EventSink::new();
        sink.emit(42u32);
        sink.emit("hello".to_string());
        sink.emit(3.14f64);

        let events = sink.drain();
        assert_eq!(events.len(), 3);
        assert_eq!(*events[0].downcast_ref::<u32>().unwrap(), 42);
        assert_eq!(*events[1].downcast_ref::<String>().unwrap(), "hello");
        assert!((*events[2].downcast_ref::<f64>().unwrap() - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_sink_is_empty() {
        let sink = EventSink::new();
        assert!(sink.is_empty());

        sink.emit(1u32);
        assert!(!sink.is_empty());

        sink.drain();
        assert!(sink.is_empty());
    }
}
