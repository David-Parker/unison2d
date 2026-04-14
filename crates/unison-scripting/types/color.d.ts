/** Color userdata with RGBA components and interpolation. */
declare interface Color {
  /** Red component in [0, 1]. */
  readonly r: number;
  /** Green component in [0, 1]. */
  readonly g: number;
  /** Blue component in [0, 1]. */
  readonly b: number;
  /** Alpha component in [0, 1]. */
  readonly a: number;
  /** Linear interpolation between this color and another. */
  lerp(this: Color, other: Color, t: number): Color;
}

// Color constructor is now unison.Color.hex / unison.Color.rgba
// See unison.d.ts.
