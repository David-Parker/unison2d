//! Audio subsystem for Unison 2D.
//!
//! Provides cross-platform music + SFX playback, 2D-spatial audio, bus mixing,
//! and tweened parameter changes via a swappable [`AudioBackend`] trait.
//!
//! See `docs/api/audio.md` for usage.

pub mod backend;
pub mod id;
pub mod params;
pub mod stub_backend;
pub mod system;

#[cfg(feature = "backend-kira")]
pub mod kira_backend;

pub use backend::{AudioBackend, BackendPlayParams, BackendSpatialParams};
pub use id::{BackendBusId, BackendPlaybackId, BackendSoundId, BusId, PlaybackId, SoundId};
pub use params::{AudioError, PlayParams, Rolloff, SpatialParams};
pub use stub_backend::{StubBackend, StubEvent};
pub use system::{AudioSystem, MusicOptions, StopOptions};

#[cfg(feature = "backend-kira")]
pub use kira_backend::KiraBackend;
