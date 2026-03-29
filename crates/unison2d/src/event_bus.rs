//! EventBus — generic, type-erased event bus with batched dispatch.
//!
//! Events accumulate via `emit()` or through `EventSink` handles, and handlers
//! fire when `flush()` is called. Handlers receive the event payload and a mutable
//! reference to a context type (typically `World`).
//!
//! ```ignore
//! let mut bus = EventBus::<MyContext>::new();
//! bus.on::<ScoreEvent>(|event, ctx| {
//!     println!("Scored {}!", event.points);
//! });
//! bus.emit(ScoreEvent { points: 100 });
//! bus.flush(&mut ctx);  // handler fires here
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

use unison_core::EventSink;

// ── Public types ──

/// Opaque handle for unsubscribing a handler.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct HandlerId(u64);

/// A generic, type-erased event bus.
///
/// `C` is the context type passed to handlers (e.g., `World`).
pub struct EventBus<C: 'static> {
    /// Per-event-type channels (each owns its queue + handlers).
    channels: HashMap<TypeId, Box<dyn AnyChannel<C>>>,
    /// Sinks created by `create_sink()` — drained on flush.
    sinks: Vec<EventSink>,
    /// Next handler ID.
    next_id: u64,
}

impl<C: 'static> EventBus<C> {
    /// Create a new empty event bus.
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            sinks: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a handler for event type `T`. Returns a `HandlerId` for unsubscribing.
    pub fn on<T: 'static>(&mut self, handler: impl FnMut(&T, &mut C) + 'static) -> HandlerId {
        let id = HandlerId(self.next_id);
        self.next_id += 1;

        let channel = self.channel_mut::<T>();
        channel.handlers.push((id, Box::new(handler)));
        id
    }

    /// Remove a handler by its `HandlerId`.
    pub fn off(&mut self, id: HandlerId) {
        for channel in self.channels.values_mut() {
            if channel.remove_handler(id) {
                return;
            }
        }
    }

    /// Emit an event directly into the bus queue.
    ///
    /// The event won't fire handlers immediately — it's queued until `flush()`.
    pub fn emit<T: 'static>(&mut self, event: T) {
        self.channel_mut::<T>().events.push(event);
    }

    /// Create a new `EventSink` linked to this bus.
    ///
    /// The sink can be cloned and given to subsystems (UI, physics, etc.).
    /// All events emitted through the sink are collected during `flush()`.
    pub fn create_sink(&mut self) -> EventSink {
        let sink = EventSink::new();
        self.sinks.push(sink.clone());
        sink
    }

    /// Drain all sinks and queues, fire handlers, and clear.
    ///
    /// Events emitted by handlers during this flush are NOT processed in this
    /// flush — they stay queued for the next `flush()` call (prevents infinite loops).
    pub fn flush(&mut self, ctx: &mut C) {
        // 1. Collect type-erased events from all sinks
        let mut sink_events: Vec<Box<dyn Any>> = Vec::new();
        for sink in &self.sinks {
            sink_events.extend(sink.drain());
        }

        // 2. Route sink events into typed channels
        for event in sink_events {
            self.route_boxed_event(event);
        }

        // 3. Fire handlers for each channel that has queued events
        for channel in self.channels.values_mut() {
            channel.flush(ctx);
        }
    }

    /// Merge pending events from another bus into this one.
    ///
    /// Used for re-entrant events: take the bus out with `mem::take`, flush,
    /// then absorb any events that were emitted into the temporary replacement.
    pub fn absorb(&mut self, other: &mut Self) {
        for (type_id, other_channel) in &mut other.channels {
            if other_channel.event_count() == 0 {
                continue;
            }
            if let Some(my_channel) = self.channels.get_mut(type_id) {
                my_channel.absorb_from(other_channel.as_mut());
            } else {
                // No channel for this type yet — take ownership
                self.channels.insert(*type_id, other_channel.take_events());
            }
        }
    }

    // ── Internal ──

    /// Get or create the typed channel for event type `T`.
    fn channel_mut<T: 'static>(&mut self) -> &mut TypedChannel<T, C> {
        let type_id = TypeId::of::<T>();
        self.channels
            .entry(type_id)
            .or_insert_with(|| Box::new(TypedChannel::<T, C>::new()));

        self.channels.get_mut(&type_id).unwrap()
            .as_any_mut()
            .downcast_mut::<TypedChannel<T, C>>()
            .unwrap()
    }

    /// Try to route a Box<dyn Any> event into the matching channel.
    fn route_boxed_event(&mut self, mut event: Box<dyn Any>) {
        for channel in self.channels.values_mut() {
            match channel.try_push_boxed(event) {
                Ok(()) => return,
                Err(returned) => event = returned,
            }
        }
        // Event type has no channel — silently dropped (no handler registered)
    }
}

impl<C: 'static> Default for EventBus<C> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Type-erased channel (queue + handlers for one event type) ──

trait AnyChannel<C> {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove_handler(&mut self, id: HandlerId) -> bool;
    fn flush(&mut self, ctx: &mut C);
    fn event_count(&self) -> usize;
    fn absorb_from(&mut self, other: &mut dyn AnyChannel<C>);
    fn take_events(&mut self) -> Box<dyn AnyChannel<C>>;
    fn try_push_boxed(&mut self, event: Box<dyn Any>) -> Result<(), Box<dyn Any>>;
}

struct TypedChannel<T: 'static, C: 'static> {
    events: Vec<T>,
    handlers: Vec<(HandlerId, Box<dyn FnMut(&T, &mut C)>)>,
}

impl<T: 'static, C: 'static> TypedChannel<T, C> {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            handlers: Vec::new(),
        }
    }
}

impl<T: 'static, C: 'static> AnyChannel<C> for TypedChannel<T, C> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove_handler(&mut self, id: HandlerId) -> bool {
        let before = self.handlers.len();
        self.handlers.retain(|(h, _)| *h != id);
        self.handlers.len() < before
    }

    fn flush(&mut self, ctx: &mut C) {
        if self.events.is_empty() {
            return;
        }
        let events = std::mem::take(&mut self.events);
        for event in &events {
            for (_, handler) in &mut self.handlers {
                handler(event, ctx);
            }
        }
    }

    fn event_count(&self) -> usize {
        self.events.len()
    }

    fn absorb_from(&mut self, other: &mut dyn AnyChannel<C>) {
        if let Some(other) = other.as_any_mut().downcast_mut::<TypedChannel<T, C>>() {
            self.events.append(&mut other.events);
        }
    }

    fn take_events(&mut self) -> Box<dyn AnyChannel<C>> {
        let mut new_channel = TypedChannel::<T, C>::new();
        new_channel.events = std::mem::take(&mut self.events);
        Box::new(new_channel)
    }

    fn try_push_boxed(&mut self, event: Box<dyn Any>) -> Result<(), Box<dyn Any>> {
        match event.downcast::<T>() {
            Ok(typed) => {
                self.events.push(*typed);
                Ok(())
            }
            Err(event) => Err(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestCtx {
        value: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ScoreEvent {
        points: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct DamageEvent {
        amount: f32,
    }

    #[test]
    fn test_on_emit_flush() {
        let mut bus = EventBus::<TestCtx>::new();
        let fired = Rc::new(RefCell::new(false));
        let fired_clone = fired.clone();

        bus.on::<ScoreEvent>(move |_event, _ctx| {
            *fired_clone.borrow_mut() = true;
        });
        bus.emit(ScoreEvent { points: 100 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);

        assert!(*fired.borrow());
    }

    #[test]
    fn test_handler_receives_event_data() {
        let mut bus = EventBus::<TestCtx>::new();
        let points = Rc::new(RefCell::new(0));
        let points_clone = points.clone();

        bus.on::<ScoreEvent>(move |event, _ctx| {
            *points_clone.borrow_mut() = event.points;
        });
        bus.emit(ScoreEvent { points: 42 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);

        assert_eq!(*points.borrow(), 42);
    }

    #[test]
    fn test_handler_receives_mut_context() {
        let mut bus = EventBus::<TestCtx>::new();

        bus.on::<ScoreEvent>(|event, ctx| {
            ctx.value += event.points;
        });
        bus.emit(ScoreEvent { points: 10 });
        bus.emit(ScoreEvent { points: 20 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);

        assert_eq!(ctx.value, 30);
    }

    #[test]
    fn test_multiple_handlers_same_type() {
        let mut bus = EventBus::<TestCtx>::new();
        let count = Rc::new(RefCell::new(0));

        let c1 = count.clone();
        bus.on::<ScoreEvent>(move |_, _| { *c1.borrow_mut() += 1; });
        let c2 = count.clone();
        bus.on::<ScoreEvent>(move |_, _| { *c2.borrow_mut() += 1; });

        bus.emit(ScoreEvent { points: 1 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);

        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn test_multiple_event_types() {
        let mut bus = EventBus::<TestCtx>::new();
        let score_fired = Rc::new(RefCell::new(false));
        let damage_fired = Rc::new(RefCell::new(false));

        let sf = score_fired.clone();
        bus.on::<ScoreEvent>(move |_, _| { *sf.borrow_mut() = true; });
        let df = damage_fired.clone();
        bus.on::<DamageEvent>(move |_, _| { *df.borrow_mut() = true; });

        bus.emit(ScoreEvent { points: 1 });
        // Don't emit DamageEvent

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);

        assert!(*score_fired.borrow());
        assert!(!*damage_fired.borrow());
    }

    #[test]
    fn test_off_removes_handler() {
        let mut bus = EventBus::<TestCtx>::new();
        let count = Rc::new(RefCell::new(0));
        let c = count.clone();
        let id = bus.on::<ScoreEvent>(move |_, _| { *c.borrow_mut() += 1; });

        bus.emit(ScoreEvent { points: 1 });
        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);
        assert_eq!(*count.borrow(), 1);

        bus.off(id);
        bus.emit(ScoreEvent { points: 1 });
        bus.flush(&mut ctx);
        assert_eq!(*count.borrow(), 1); // should not have incremented
    }

    #[test]
    fn test_emit_without_handler() {
        let mut bus = EventBus::<TestCtx>::new();
        bus.emit(ScoreEvent { points: 100 });
        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx); // should not panic
    }

    #[test]
    fn test_flush_without_events() {
        let mut bus = EventBus::<TestCtx>::new();
        let fired = Rc::new(RefCell::new(false));
        let f = fired.clone();
        bus.on::<ScoreEvent>(move |_, _| { *f.borrow_mut() = true; });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx); // no events, handler should not fire

        assert!(!*fired.borrow());
    }

    #[test]
    fn test_reentrant_events_deferred() {
        let mut bus = EventBus::<TestCtx>::new();
        let mut bus2 = EventBus::<TestCtx>::new();

        bus.on::<ScoreEvent>(|event, ctx| {
            ctx.value += event.points;
        });

        bus.emit(ScoreEvent { points: 10 });
        bus2.emit(ScoreEvent { points: 99 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);
        assert_eq!(ctx.value, 10);

        bus.absorb(&mut bus2);
        bus.flush(&mut ctx);
        assert_eq!(ctx.value, 10 + 99);
    }

    #[test]
    fn test_absorb_merges_events() {
        let mut bus1 = EventBus::<TestCtx>::new();
        let mut bus2 = EventBus::<TestCtx>::new();

        bus1.on::<ScoreEvent>(|event, ctx| {
            ctx.value += event.points;
        });

        bus2.emit(ScoreEvent { points: 50 });
        bus1.absorb(&mut bus2);

        let mut ctx = TestCtx { value: 0 };
        bus1.flush(&mut ctx);
        assert_eq!(ctx.value, 50);
    }

    #[test]
    fn test_create_sink_drains_on_flush() {
        let mut bus = EventBus::<TestCtx>::new();
        let sink = bus.create_sink();

        bus.on::<ScoreEvent>(|event, ctx| {
            ctx.value += event.points;
        });

        sink.emit(ScoreEvent { points: 77 });

        let mut ctx = TestCtx { value: 0 };
        bus.flush(&mut ctx);
        assert_eq!(ctx.value, 77);
    }
}
