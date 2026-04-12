/** Deterministic pseudo-random number generator (xorshift64). */
declare interface Rng {
  /** Random float in [min, max). */
  range(this: Rng, min: number, max: number): number;
  /** Random integer in [min, max] (inclusive). */
  range_int(this: Rng, min: number, max: number): number;
}

/** Rng constructor table. */
declare interface RngConstructor {
  /** Create a new RNG with the given seed. Seed 0 is treated as 1. */
  new: (this: void, seed: number) => Rng;
}

/** @noSelf */
declare const Rng: RngConstructor;
