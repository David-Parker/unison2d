//! Opaque handle types exposed at the public Rust + scripting boundary.
//!
//! Each ID is a `u32` newtype with `from_raw`/`raw` for ABI translation
//! to/from Lua, mirroring `LightId` in `unison-lighting`.

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct $name(pub(crate) u32);

        impl $name {
            /// Create from a raw integer (for scripting bridges).
            pub fn from_raw(raw: u32) -> Self { Self(raw) }
            /// Get the raw integer handle.
            pub fn raw(self) -> u32 { self.0 }
        }
    };
}

id_newtype!(SoundId,    "Loaded sound asset, owned by `AudioSystem`.");
id_newtype!(BusId,      "Mix bus (master, music, sfx, or user-created).");
id_newtype!(PlaybackId, "A single playing instance of a sound.");

// Backend-side IDs are kept distinct so AudioSystem can translate
// between its own external IDs and whatever the backend hands back.
id_newtype!(BackendSoundId,    "Backend-internal sound handle.");
id_newtype!(BackendBusId,      "Backend-internal bus handle.");
id_newtype!(BackendPlaybackId, "Backend-internal playback handle.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_sound_id() {
        assert_eq!(SoundId::from_raw(42).raw(), 42);
    }

    #[test]
    fn round_trip_playback_id() {
        assert_eq!(PlaybackId::from_raw(7).raw(), 7);
    }

    #[test]
    fn round_trip_bus_id() {
        assert_eq!(BusId::from_raw(3).raw(), 3);
    }
}
