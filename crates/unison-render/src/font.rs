//! Font handle types.
//!
//! Fonts are identified by an opaque [`FontId`] allocated when a font asset is
//! loaded through the engine. The renderer itself does not own font data; the
//! engine maps [`FontId`] → asset path and UI code fetches bytes from the asset
//! store on demand.

/// Opaque handle to a font asset registered with the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub u32);

impl FontId {
    /// A null/invalid font ID.
    pub const NONE: Self = Self(u32::MAX);

    /// True if this is not the sentinel NONE value.
    pub fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }

    /// Get the raw u32 value (for serialization / FFI / scripting).
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Reconstruct from a raw u32 value.
    pub fn from_raw(val: u32) -> Self {
        Self(val)
    }
}
