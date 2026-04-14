/** Deterministic pseudo-random number generator (xorshift64). */
declare interface Rng {
  /** Random float in [min, max). */
  range(this: Rng, min: number, max: number): number;
  /** Random integer in [min, max] (inclusive). */
  range_int(this: Rng, min: number, max: number): number;
}

// Rng constructor is now unison.Rng.new
// See unison.d.ts.
