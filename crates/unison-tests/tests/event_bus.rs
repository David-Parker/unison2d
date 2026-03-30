//! Integration tests for the EventBus system.
//!
//! Tests cover: EventSink → EventBus flow, collision events (physics → handler),
//! UI events → EventBus flow, handler mutation, custom game events, and event chains.

use std::cell::RefCell;
use std::rc::Rc;

use unison_core::{Color, Vec2};
use unison_physics::mesh::create_ring_mesh;
use unison_physics::Material;
use unison_render::TextureId;
use unison2d::{CollisionEvent, EventBus, ObjectId, SoftBodyDesc, World};

// ── Helpers ──

fn spawn_soft_body(world: &mut World, pos: Vec2) -> ObjectId {
    let mesh = create_ring_mesh(1.0, 0.5, 8, 2);
    world.objects.spawn_soft_body(SoftBodyDesc {
        mesh,
        material: Material::RUBBER,
        position: pos,
        color: Color::WHITE,
        texture: TextureId::NONE,
    })
}

// ── EventSink → EventBus flow ──

#[test]
fn sink_events_reach_handler() {
    let mut bus = EventBus::<World>::new();
    let sink = bus.create_sink();

    let received = Rc::new(RefCell::new(Vec::new()));
    let r = received.clone();
    bus.on::<String>(move |event, _world| {
        r.borrow_mut().push(event.clone());
    });

    sink.emit("hello".to_string());
    sink.emit("world".to_string());

    let mut world = World::new();
    bus.flush(&mut world);

    assert_eq!(*received.borrow(), vec!["hello", "world"]);
}

#[test]
fn multiple_sinks_all_drain() {
    let mut bus = EventBus::<World>::new();
    let sink1 = bus.create_sink();
    let sink2 = bus.create_sink();

    let total = Rc::new(RefCell::new(0i32));
    let t = total.clone();
    bus.on::<i32>(move |val, _world| {
        *t.borrow_mut() += val;
    });

    sink1.emit(10i32);
    sink2.emit(20i32);
    sink1.emit(5i32);

    let mut world = World::new();
    bus.flush(&mut world);

    assert_eq!(*total.borrow(), 35);
}

// ── Collision events ──

#[test]
fn collision_event_fires_on_contact() {
    let mut world = World::new();
    let mut bus = EventBus::<World>::new();

    // Spawn two soft bodies close together so they collide
    let _a = spawn_soft_body(&mut world, Vec2::new(0.0, 3.0));
    let _b = spawn_soft_body(&mut world, Vec2::new(0.5, 3.0));

    world.objects.set_collision_events_enabled(true);

    let fired = Rc::new(RefCell::new(false));
    let f = fired.clone();
    bus.on::<CollisionEvent>(move |_event, _world| {
        *f.borrow_mut() = true;
    });

    // Step until bodies overlap and generate a collision event
    for _ in 0..120 {
        world.step(1.0 / 60.0);

        // Translate physics events → CollisionEvent
        for event in world.objects.translate_collision_events() {
            bus.emit(event);
        }

        let mut temp = std::mem::take(&mut bus);
        temp.flush(&mut world);
        let mut replacement = std::mem::take(&mut bus);
        temp.absorb(&mut replacement);
        bus = temp;

        if *fired.borrow() {
            break;
        }
    }

    assert!(*fired.borrow(), "Collision event should have fired");
}

#[test]
fn collision_event_has_contact_data() {
    let mut world = World::new();
    let mut bus = EventBus::<World>::new();

    let _a = spawn_soft_body(&mut world, Vec2::new(0.0, 3.0));
    let _b = spawn_soft_body(&mut world, Vec2::new(0.5, 3.0));

    world.objects.set_collision_events_enabled(true);

    let contact = Rc::new(RefCell::new(None::<CollisionEvent>));
    let c = contact.clone();
    bus.on::<CollisionEvent>(move |event, _world| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(event.clone());
        }
    });

    for _ in 0..120 {
        world.step(1.0 / 60.0);
        for event in world.objects.translate_collision_events() {
            bus.emit(event);
        }
        let mut temp = std::mem::take(&mut bus);
        temp.flush(&mut world);
        let mut replacement = std::mem::take(&mut bus);
        temp.absorb(&mut replacement);
        bus = temp;

        if contact.borrow().is_some() {
            break;
        }
    }

    let event = contact.borrow().clone().expect("Should have received a collision event");
    assert!(event.normal.x.is_finite() && event.normal.y.is_finite());
    assert!(event.penetration >= 0.0);
    assert!(event.contact_point.x.is_finite() && event.contact_point.y.is_finite());
}

#[test]
fn collision_for_filters_by_object() {
    let mut world = World::new();
    let mut bus = EventBus::<World>::new();

    // Three bodies: a and b close together, c far away
    let a = spawn_soft_body(&mut world, Vec2::new(0.0, 3.0));
    let _b = spawn_soft_body(&mut world, Vec2::new(0.5, 3.0));
    let _c = spawn_soft_body(&mut world, Vec2::new(100.0, 3.0));

    world.objects.set_collision_events_enabled(true);

    let a_collisions = Rc::new(RefCell::new(0u32));
    let ac = a_collisions.clone();
    let target = a;
    bus.on::<CollisionEvent>(move |event, _world| {
        if event.object_a == target || event.object_b == target {
            *ac.borrow_mut() += 1;
        }
    });

    for _ in 0..120 {
        world.step(1.0 / 60.0);
        for event in world.objects.translate_collision_events() {
            bus.emit(event);
        }
        let mut temp = std::mem::take(&mut bus);
        temp.flush(&mut world);
        let mut replacement = std::mem::take(&mut bus);
        temp.absorb(&mut replacement);
        bus = temp;
    }

    // a and b should collide; c is too far away to be involved
    assert!(*a_collisions.borrow() > 0, "Object A should have been in at least one collision");
}

#[test]
fn collision_between_filters_by_pair() {
    let mut world = World::new();
    let mut bus = EventBus::<World>::new();

    let a = spawn_soft_body(&mut world, Vec2::new(0.0, 3.0));
    let b = spawn_soft_body(&mut world, Vec2::new(0.5, 3.0));
    let c = spawn_soft_body(&mut world, Vec2::new(100.0, 3.0));

    world.objects.set_collision_events_enabled(true);

    let ab_count = Rc::new(RefCell::new(0u32));
    let ac_count = Rc::new(RefCell::new(0u32));

    let ab = ab_count.clone();
    let (ta, tb) = (a, b);
    bus.on::<CollisionEvent>(move |event, _world| {
        let pair = (event.object_a == ta && event.object_b == tb)
            || (event.object_a == tb && event.object_b == ta);
        if pair {
            *ab.borrow_mut() += 1;
        }
    });

    let ac = ac_count.clone();
    let (ta2, tc) = (a, c);
    bus.on::<CollisionEvent>(move |event, _world| {
        let pair = (event.object_a == ta2 && event.object_b == tc)
            || (event.object_a == tc && event.object_b == ta2);
        if pair {
            *ac.borrow_mut() += 1;
        }
    });

    for _ in 0..120 {
        world.step(1.0 / 60.0);
        for event in world.objects.translate_collision_events() {
            bus.emit(event);
        }
        let mut temp = std::mem::take(&mut bus);
        temp.flush(&mut world);
        let mut replacement = std::mem::take(&mut bus);
        temp.absorb(&mut replacement);
        bus = temp;
    }

    assert!(*ab_count.borrow() > 0, "A-B pair should have collided");
    assert_eq!(*ac_count.borrow(), 0, "A-C pair should NOT have collided (too far apart)");
}

#[test]
fn collision_between_symmetric() {
    // CollisionEvent should match regardless of which body is object_a vs object_b
    let mut bus = EventBus::<World>::new();
    let a = ObjectId::PLACEHOLDER; // Use known IDs for this unit-style test
    let b = ObjectId::default();

    let matched = Rc::new(RefCell::new(false));
    let m = matched.clone();
    let (ta, tb) = (a, b);
    bus.on::<CollisionEvent>(move |event, _world| {
        let pair = (event.object_a == ta && event.object_b == tb)
            || (event.object_a == tb && event.object_b == ta);
        if pair {
            *m.borrow_mut() = true;
        }
    });

    // Emit with reversed order (b, a) instead of (a, b)
    bus.emit(CollisionEvent {
        object_a: b,
        object_b: a,
        normal: Vec2::new(1.0, 0.0),
        penetration: 0.1,
        contact_point: Vec2::ZERO,
    });

    let mut world = World::new();
    bus.flush(&mut world);

    assert!(*matched.borrow(), "Symmetric pair matching should work");
}

#[test]
fn no_collision_events_when_disabled() {
    let mut world = World::new();
    let mut bus = EventBus::<World>::new();

    let _a = spawn_soft_body(&mut world, Vec2::new(0.0, 3.0));
    let _b = spawn_soft_body(&mut world, Vec2::new(0.5, 3.0));

    // collision events are disabled by default
    assert!(!world.objects.collision_events_enabled());

    let fired = Rc::new(RefCell::new(false));
    let f = fired.clone();
    bus.on::<CollisionEvent>(move |_event, _world| {
        *f.borrow_mut() = true;
    });

    for _ in 0..120 {
        world.step(1.0 / 60.0);
        for event in world.objects.translate_collision_events() {
            bus.emit(event);
        }
        let mut temp = std::mem::take(&mut bus);
        temp.flush(&mut world);
        let mut replacement = std::mem::take(&mut bus);
        temp.absorb(&mut replacement);
        bus = temp;
    }

    assert!(!*fired.borrow(), "No collision events should fire when disabled");
}

#[test]
fn auto_enable_on_register() {
    let mut world = World::new();
    assert!(!world.objects.collision_events_enabled());

    // Simulates what ctx.on_collision() does internally
    world.objects.set_collision_events_enabled(true);
    assert!(world.objects.collision_events_enabled());
}

// ── Handler mutation ──

#[test]
fn handler_spawns_object_in_world() {
    let mut bus = EventBus::<World>::new();

    #[derive(Clone)]
    struct SpawnEvent;

    let spawned_id = Rc::new(RefCell::new(None::<ObjectId>));
    let s = spawned_id.clone();
    bus.on::<SpawnEvent>(move |_event, world| {
        let mesh = create_ring_mesh(0.5, 0.3, 6, 2);
        let id = world.objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material: Material::RUBBER,
            position: Vec2::new(5.0, 5.0),
            color: Color::RED,
            texture: TextureId::NONE,
        });
        *s.borrow_mut() = Some(id);
    });

    bus.emit(SpawnEvent);

    let mut world = World::new();
    bus.flush(&mut world);

    let id = spawned_id.borrow().expect("Handler should have spawned an object");
    let pos = world.objects.get_position(id);
    assert!((pos.x - 5.0).abs() < 1.0 && (pos.y - 5.0).abs() < 1.0,
        "Spawned object should be near (5, 5)");
}

#[test]
fn handler_moves_object() {
    let mut world = World::new();
    let obj = spawn_soft_body(&mut world, Vec2::new(0.0, 5.0));

    #[derive(Clone)]
    struct MoveEvent {
        target: ObjectId,
        new_pos: Vec2,
    }

    let mut bus = EventBus::<World>::new();
    bus.on::<MoveEvent>(move |event, world| {
        world.objects.set_position(event.target, event.new_pos);
    });

    bus.emit(MoveEvent {
        target: obj,
        new_pos: Vec2::new(99.0, 99.0),
    });
    bus.flush(&mut world);

    let pos = world.objects.get_position(obj);
    // Position should be approximately at the target (soft body position is center of mass)
    assert!((pos.x - 99.0).abs() < 1.0 && (pos.y - 99.0).abs() < 1.0,
        "Object should have moved near (99, 99), got ({}, {})", pos.x, pos.y);
}

// ── Custom game events ──

#[test]
fn custom_event_type() {
    #[derive(Clone, Debug, PartialEq)]
    struct ScoreUp {
        player: u32,
        points: i64,
    }

    let mut bus = EventBus::<World>::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let r = received.clone();
    bus.on::<ScoreUp>(move |event, _world| {
        r.borrow_mut().push(event.clone());
    });

    bus.emit(ScoreUp { player: 1, points: 100 });
    bus.emit(ScoreUp { player: 2, points: 250 });

    let mut world = World::new();
    bus.flush(&mut world);

    let events = received.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], ScoreUp { player: 1, points: 100 });
    assert_eq!(events[1], ScoreUp { player: 2, points: 250 });
}

#[test]
fn event_chain_deferred_to_next_flush() {
    // Handler for event A emits event B. B should NOT fire in the same flush.
    #[derive(Clone)]
    struct EventA;
    #[derive(Clone)]
    struct EventB;

    let mut bus = EventBus::<World>::new();

    let b_fired_during_first_flush = Rc::new(RefCell::new(false));
    let b_fired_during_second_flush = Rc::new(RefCell::new(false));

    // We can't emit into the bus from within a handler (we don't have &mut bus).
    // Instead, we use an EventSink to emit B from A's handler.
    let sink = bus.create_sink();
    bus.on::<EventA>(move |_event, _world| {
        sink.emit(EventB);
    });

    let bf1 = b_fired_during_first_flush.clone();
    let bf2 = b_fired_during_second_flush.clone();
    let first_flush_done = Rc::new(RefCell::new(false));
    let ffd = first_flush_done.clone();
    bus.on::<EventB>(move |_event, _world| {
        if *ffd.borrow() {
            *bf2.borrow_mut() = true;
        } else {
            *bf1.borrow_mut() = true;
        }
    });

    bus.emit(EventA);

    let mut world = World::new();

    // First flush: A fires, emits B into sink. B should NOT fire yet.
    let mut temp = std::mem::take(&mut bus);
    temp.flush(&mut world);
    let mut replacement = std::mem::take(&mut bus);
    temp.absorb(&mut replacement);
    bus = temp;

    assert!(!*b_fired_during_first_flush.borrow(),
        "EventB should NOT fire during the same flush as EventA");

    *first_flush_done.borrow_mut() = true;

    // Second flush: B should fire now (sink drains on flush).
    let mut temp = std::mem::take(&mut bus);
    temp.flush(&mut world);
    let mut replacement = std::mem::take(&mut bus);
    temp.absorb(&mut replacement);
    let _bus = temp;

    assert!(*b_fired_during_second_flush.borrow(),
        "EventB should fire on the second flush");
}

// ── Drain (pull-based consumption) ──

#[test]
fn drain_returns_queued_events() {
    let mut bus = EventBus::<World>::new();

    #[derive(Clone, Debug, PartialEq)]
    struct MenuAction(u32);

    bus.emit(MenuAction(1));
    bus.emit(MenuAction(2));
    bus.emit(MenuAction(3));

    let drained = bus.drain::<MenuAction>();
    assert_eq!(drained, vec![MenuAction(1), MenuAction(2), MenuAction(3)]);

    // Second drain should be empty
    let drained2 = bus.drain::<MenuAction>();
    assert!(drained2.is_empty());
}

#[test]
fn drain_does_not_affect_other_event_types() {
    let mut bus = EventBus::<World>::new();

    bus.emit(42i32);
    bus.emit("hello".to_string());
    bus.emit(99i32);

    // Drain only i32 events
    let ints = bus.drain::<i32>();
    assert_eq!(ints, vec![42, 99]);

    // String events should still be in the bus
    let received = Rc::new(RefCell::new(Vec::new()));
    let r = received.clone();
    bus.on::<String>(move |s, _| { r.borrow_mut().push(s.clone()); });

    let mut world = World::new();
    bus.flush(&mut world);

    assert_eq!(*received.borrow(), vec!["hello"]);
}

// ── UI events via EventSink ──

#[test]
fn ui_sink_events_reach_bus_handler() {
    // Simulates the UI → EventSink → EventBus flow:
    // Ui emits into a sink, bus.flush() drains the sink and fires handlers.
    let mut bus = EventBus::<World>::new();
    let ui_sink = bus.create_sink();

    #[derive(Clone, Debug, PartialEq)]
    struct ButtonClicked(String);

    let received = Rc::new(RefCell::new(Vec::new()));
    let r = received.clone();
    bus.on::<ButtonClicked>(move |event, _world| {
        r.borrow_mut().push(event.clone());
    });

    // Simulate UI emitting button click events
    ui_sink.emit(ButtonClicked("play".to_string()));
    ui_sink.emit(ButtonClicked("settings".to_string()));

    let mut world = World::new();
    bus.flush(&mut world);

    assert_eq!(*received.borrow(), vec![
        ButtonClicked("play".to_string()),
        ButtonClicked("settings".to_string()),
    ]);
}

#[test]
fn ui_and_collision_events_same_flush() {
    // Both UI events and collision events should fire in a single flush.
    let mut bus = EventBus::<World>::new();
    let ui_sink = bus.create_sink();

    #[derive(Clone)]
    struct UIAction;

    let ui_fired = Rc::new(RefCell::new(false));
    let collision_fired = Rc::new(RefCell::new(false));

    let uf = ui_fired.clone();
    bus.on::<UIAction>(move |_, _| { *uf.borrow_mut() = true; });

    let cf = collision_fired.clone();
    bus.on::<CollisionEvent>(move |_, _| { *cf.borrow_mut() = true; });

    // Emit a UI event via sink
    ui_sink.emit(UIAction);

    // Emit a collision event directly
    bus.emit(CollisionEvent {
        object_a: ObjectId::PLACEHOLDER,
        object_b: ObjectId::PLACEHOLDER,
        normal: Vec2::new(0.0, 1.0),
        penetration: 0.1,
        contact_point: Vec2::ZERO,
    });

    let mut world = World::new();
    bus.flush(&mut world);

    assert!(*ui_fired.borrow(), "UI event should have fired");
    assert!(*collision_fired.borrow(), "Collision event should have fired");
}

// ── Handler-less channel preservation ──

#[test]
fn handler_less_events_survive_flush() {
    let mut bus = EventBus::<World>::new();

    #[derive(Clone, Debug, PartialEq)]
    struct Unhandled(u32);

    bus.emit(Unhandled(1));
    bus.emit(Unhandled(2));

    let mut world = World::new();
    bus.flush(&mut world);

    // Events should still be drainable — flush skips channels with no handlers
    let drained = bus.drain::<Unhandled>();
    assert_eq!(drained, vec![Unhandled(1), Unhandled(2)]);
}

#[test]
fn drain_auto_ingests_sink_events() {
    let mut bus = EventBus::<World>::new();
    let sink = bus.create_sink();

    #[derive(Clone, Debug, PartialEq)]
    struct Action(String);

    // Emit via sink (type-erased)
    sink.emit(Action("play".to_string()));
    sink.emit(Action("quit".to_string()));

    // drain() automatically ingests sinks — no manual ingest_sinks() needed
    let drained = bus.drain::<Action>();
    assert_eq!(drained, vec![
        Action("play".to_string()),
        Action("quit".to_string()),
    ]);

    // Second drain is empty (events already consumed)
    let empty = bus.drain::<Action>();
    assert!(empty.is_empty());
}

#[test]
fn route_boxed_unknown_type_no_panic() {
    let mut bus = EventBus::<World>::new();

    // Register a handler for i32 to create a channel
    bus.on::<i32>(|_, _| {});

    // Emit a String through a sink — no channel for String exists
    let sink = bus.create_sink();
    sink.emit("orphan".to_string());

    let mut world = World::new();
    bus.flush(&mut world); // should not panic
}
