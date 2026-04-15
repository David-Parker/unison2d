//! `unison.audio.*` Lua bindings. Uses the thread-local engine pointer
//! to call methods on `engine.audio`.

use mlua::prelude::*;
use unison_audio::{MusicOptions, PlayParams};
use super::engine_state::with_engine_ptr;

fn read_opt_f32(t: &LuaTable, key: &str) -> Option<f32> {
    t.get::<f32>(key).ok()
}
fn read_opt_string(t: &LuaTable, key: &str) -> Option<String> {
    t.get::<String>(key).ok()
}
fn read_tween(t: Option<LuaTable>) -> Option<f32> {
    t.and_then(|t| read_opt_f32(&t, "tween"))
}
fn read_fade_out(t: Option<LuaTable>) -> Option<f32> {
    t.and_then(|t| read_opt_f32(&t, "fade_out"))
}

fn play_params_from_opts(t: Option<LuaTable>, default_bus: unison_audio::BusId) -> PlayParams {
    let mut p = PlayParams::with_bus(default_bus);
    if let Some(t) = t {
        if let Some(v) = read_opt_f32(&t, "volume") { p.volume = v; }
        if let Some(v) = read_opt_f32(&t, "pitch")  { p.pitch  = v; }
        if let Ok(b) = t.get::<bool>("looping")     { p.looping = b; }
        p.fade_in = read_opt_f32(&t, "fade_in");
        if let Some(name) = read_opt_string(&t, "bus") {
            with_engine_ptr(|e| {
                if let Some(bid) = e.audio.bus_by_name(&name) { p.bus = bid; }
            });
        }
    }
    p
}

pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let audio = lua.create_table()?;

    // unload(SoundId)
    audio.set("unload", lua.create_function(|_, id: u32| {
        with_engine_ptr(|e| e.audio.unload(unison_audio::SoundId::from_raw(id)));
        Ok(())
    })?)?;

    // play(SoundId, opts?) -> PlaybackId
    audio.set("play", lua.create_function(|_, (id, opts): (u32, Option<LuaTable>)| {
        let pb = with_engine_ptr(|e| {
            let params = play_params_from_opts(opts, e.audio.sfx_bus());
            e.audio.play(unison_audio::SoundId::from_raw(id), params).ok()
        }).flatten().map(|p| p.raw()).unwrap_or(0);
        Ok(pb)
    })?)?;

    // stop, pause, resume, is_playing
    audio.set("stop", lua.create_function(|_, (id, opts): (u32, Option<LuaTable>)| {
        let fade = read_fade_out(opts);
        with_engine_ptr(|e| e.audio.stop(unison_audio::PlaybackId::from_raw(id), fade));
        Ok(())
    })?)?;
    audio.set("pause", lua.create_function(|_, id: u32| {
        with_engine_ptr(|e| e.audio.pause(unison_audio::PlaybackId::from_raw(id)));
        Ok(())
    })?)?;
    audio.set("resume", lua.create_function(|_, id: u32| {
        with_engine_ptr(|e| e.audio.resume(unison_audio::PlaybackId::from_raw(id)));
        Ok(())
    })?)?;
    audio.set("is_playing", lua.create_function(|_, id: u32| {
        Ok(with_engine_ptr(|e| e.audio.is_playing(unison_audio::PlaybackId::from_raw(id))).unwrap_or(false))
    })?)?;

    // set_volume / set_pitch
    audio.set("set_volume", lua.create_function(|_, (id, v, opts): (u32, f32, Option<LuaTable>)| {
        let tween = read_tween(opts);
        with_engine_ptr(|e| e.audio.set_volume(unison_audio::PlaybackId::from_raw(id), v, tween));
        Ok(())
    })?)?;
    audio.set("set_pitch", lua.create_function(|_, (id, p, opts): (u32, f32, Option<LuaTable>)| {
        let tween = read_tween(opts);
        with_engine_ptr(|e| e.audio.set_pitch(unison_audio::PlaybackId::from_raw(id), p, tween));
        Ok(())
    })?)?;

    // play_music / stop_music / pause_music / resume_music / current_music
    audio.set("play_music", lua.create_function(|_, (id, opts): (u32, Option<LuaTable>)| {
        let mut m = MusicOptions::default();
        if let Some(t) = opts {
            if let Some(v) = read_opt_f32(&t, "volume")    { m.volume = v; }
            m.fade_in   = read_opt_f32(&t, "fade_in");
            m.crossfade = read_opt_f32(&t, "crossfade");
            if let Some(name) = read_opt_string(&t, "bus") {
                m.bus = with_engine_ptr(|e| e.audio.bus_by_name(&name)).flatten();
            }
        }
        Ok(with_engine_ptr(|e| e.audio.play_music(unison_audio::SoundId::from_raw(id), m).ok())
           .flatten().map(|p| p.raw()).unwrap_or(0))
    })?)?;
    audio.set("stop_music", lua.create_function(|_, opts: Option<LuaTable>| {
        let fade = read_fade_out(opts);
        with_engine_ptr(|e| e.audio.stop_music(fade));
        Ok(())
    })?)?;
    audio.set("pause_music", lua.create_function(|_, ()| {
        with_engine_ptr(|e| e.audio.pause_music()); Ok(())
    })?)?;
    audio.set("resume_music", lua.create_function(|_, ()| {
        with_engine_ptr(|e| e.audio.resume_music()); Ok(())
    })?)?;
    audio.set("current_music", lua.create_function(|_, ()| {
        Ok(with_engine_ptr(|e| e.audio.current_music()).flatten().map(|p| p.raw()))
    })?)?;

    // master / bus
    audio.set("set_master_volume", lua.create_function(|_, (v, opts): (f32, Option<LuaTable>)| {
        let tween = read_tween(opts);
        with_engine_ptr(|e| e.audio.set_master_volume(v, tween)); Ok(())
    })?)?;
    audio.set("set_bus_volume", lua.create_function(|_, (name, v, opts): (String, f32, Option<LuaTable>)| {
        let tween = read_tween(opts);
        with_engine_ptr(|e| {
            if let Some(bid) = e.audio.bus_by_name(&name) {
                e.audio.set_bus_volume(bid, v, tween);
            }
        });
        Ok(())
    })?)?;
    audio.set("create_bus", lua.create_function(|_, name: String| {
        with_engine_ptr(|e| { e.audio.create_bus(&name); });
        Ok(())
    })?)?;

    // stop_all
    audio.set("stop_all", lua.create_function(|_, opts: Option<LuaTable>| {
        let fade = read_fade_out(opts);
        with_engine_ptr(|e| e.audio.stop_all(fade));
        Ok(())
    })?)?;

    unison.set("audio", audio)?;
    Ok(())
}
