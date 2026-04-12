/** Extensions added to Lua's built-in math table. */
declare namespace math {
  /** Linear interpolation: a + (b - a) * t. */
  function lerp(a: number, b: number, t: number): number;
  /** Smooth Hermite interpolation, clamped to [0, 1]. */
  function smoothstep(edge0: number, edge1: number, x: number): number;
  /** Clamp x to [min, max]. */
  function clamp(x: number, min: number, max: number): number;
}
