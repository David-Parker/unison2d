//! Audio subsystem for Unison 2D.
//!
//! Provides cross-platform music + SFX playback, 2D-spatial audio, bus mixing,
//! and tweened parameter changes via a swappable [`AudioBackend`] trait.
//!
//! See `docs/api/audio.md` for usage.

pub mod id;
pub mod params;

pub use id::{BusId, PlaybackId, SoundId};
pub use params::{AudioError, PlayParams, Rolloff, SpatialParams};
