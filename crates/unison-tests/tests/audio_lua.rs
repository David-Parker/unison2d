//! Integration tests for `unison.audio.*` Lua bindings.
//!
//! These tests install a `StubBackend` in place of the default audio system
//! so the recorded event stream can be asserted against the expected behavior
//! after running a Lua snippet via `ScriptedGame`.

use unison_audio::{AudioSystem, StubBackend, StubEvent};
use unison_scripting::ScriptedGame;
use unison2d::{Engine, Game};

/// Build an Engine with a StubBackend-backed AudioSystem and register a
/// single fake sound asset under `path` so `unison.audio.load(path)` works.
fn setup_engine(asset_path: &str) -> Engine {
    let mut engine = Engine::new();
    // Replace the default audio system with one backed by StubBackend.
    engine.audio = AudioSystem::with_backend(Box::new(StubBackend::new()));
    // Register a dummy asset so `unison.audio.load("dummy.wav")` succeeds.
    engine.assets_mut().insert(asset_path.to_string(), vec![0u8; 8]);
    engine
}

/// Run the given Lua snippet inside a `game.init()` body.
fn run_lua_in_init(engine: &mut Engine, snippet: &str) {
    let src = format!(
        r#"
            local game = {{}}
            function game.init()
                {snippet}
            end
            function game.update(dt) end
            return game
        "#
    );
    let mut game = ScriptedGame::new(&src);
    game.init(engine);
}

#[test]
fn lua_play_and_stop_round_trips_through_backend() {
    let mut engine = setup_engine("dummy.wav");

    run_lua_in_init(&mut engine, r#"
        local snd = unison.audio.load("dummy.wav")
        assert(snd ~= nil, "load should return a SoundId")
        local pb  = unison.audio.play(snd)
        assert(pb ~= 0, "play should return a non-zero PlaybackId")
        unison.audio.stop(pb)
    "#);

    let events = &engine.audio.backend_for_test().events;
    assert!(
        events.iter().any(|e| matches!(e, StubEvent::LoadSound { .. })),
        "expected LoadSound, got {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(e, StubEvent::Play { .. })),
        "expected Play, got {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(e, StubEvent::Stop { .. })),
        "expected Stop, got {events:?}"
    );
}

#[test]
fn lua_set_bus_volume_routes_to_named_bus() {
    let mut engine = setup_engine("dummy.wav");

    // Pre-resolve the expected backend bus id for `ui` by creating it up front
    // via the engine, then comparing with the bus id emitted by SetBusVolume
    // after the Lua snippet ran. We can't fully inspect bus ids without
    // recording them directly, so instead assert we saw a SetBusVolume
    // event with the expected volume.
    run_lua_in_init(&mut engine, r#"
        unison.audio.create_bus("ui")
        unison.audio.set_bus_volume("ui", 0.42)
    "#);

    let events = &engine.audio.backend_for_test().events;
    // The CreateBus events include the 3 built-ins + the user's "ui" bus.
    let create_bus_count = events.iter()
        .filter(|e| matches!(e, StubEvent::CreateBus))
        .count();
    assert_eq!(
        create_bus_count, 4,
        "expected 4 CreateBus events (master/music/sfx/ui), got {events:?}"
    );
    // Find the SetBusVolume for the "ui" bus (the 4th bus registered → backend
    // id == 3, since the stub backend increments from 0).
    let set_bus = events.iter().find(|e| matches!(e, StubEvent::SetBusVolume { .. }));
    let set_bus = set_bus.expect("expected at least one SetBusVolume event");
    match set_bus {
        StubEvent::SetBusVolume { v, bus, .. } => {
            assert!((*v - 0.42).abs() < 1e-6, "volume mismatch: {v}");
            // "ui" is the 4th bus registered; StubBackend hands out bus ids
            // starting at 0, so backend id should be 3.
            assert_eq!(bus.raw(), 3, "unexpected backend bus id: {bus:?}");
        }
        _ => unreachable!(),
    }
}

#[test]
fn lua_play_music_starts_and_crossfades() {
    let mut engine = setup_engine("dummy.wav");
    engine.assets_mut().insert("other.wav".to_string(), vec![0u8; 8]);

    run_lua_in_init(&mut engine, r#"
        local a = unison.audio.load("dummy.wav")
        local b = unison.audio.load("other.wav")
        unison.audio.play_music(a)
        unison.audio.play_music(b, { crossfade = 1.0 })
    "#);

    let events = &engine.audio.backend_for_test().events;
    // We should have at least two Play events (one per music track).
    let play_count = events.iter()
        .filter(|e| matches!(e, StubEvent::Play { .. }))
        .count();
    assert!(play_count >= 2, "expected >=2 Play events, got {events:?}");

    // The second play_music should have stopped the first (crossfade).
    let stop_with_fade = events.iter().any(|e| matches!(
        e, StubEvent::Stop { fade_out: Some(f), .. } if (*f - 1.0).abs() < 1e-6
    ));
    assert!(
        stop_with_fade,
        "expected a Stop with fade_out≈1.0 from crossfade, got {events:?}"
    );

    // Music looping should be set.
    let looped_play = events.iter().any(|e| matches!(
        e, StubEvent::Play { looping: true, .. }
    ));
    assert!(looped_play, "expected at least one looping Play event");
}

#[test]
fn lua_world_play_sound_at_routes_through_play_spatial() {
    let mut engine = setup_engine("dummy.wav");

    run_lua_in_init(&mut engine, r#"
        local snd = unison.audio.load("dummy.wav")
        assert(snd ~= nil, "load should return a SoundId")
        local w = unison.World.new()
        local pb = w:play_sound_at(snd, 1.0, 2.0)
        assert(pb ~= 0, "play_sound_at should return a non-zero PlaybackId")
    "#);

    let events = &engine.audio.backend_for_test().events;
    let spatial = events.iter().find(|e| matches!(e, StubEvent::PlaySpatial { .. }));
    let spatial = spatial.expect(&format!("expected PlaySpatial event, got {events:?}"));
    match spatial {
        StubEvent::PlaySpatial { position, .. } => {
            assert!(
                (position.x - 1.0).abs() < 1e-6 && (position.y - 2.0).abs() < 1e-6,
                "expected PlaySpatial at (1, 2), got ({}, {})", position.x, position.y
            );
        }
        _ => unreachable!(),
    }
}
