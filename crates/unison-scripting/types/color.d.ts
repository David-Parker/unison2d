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

/** Color constructor table. */
declare const Color: {
  /** Create a Color from a hex integer (e.g. `0xFF8800`). */
  hex(this: void, hex: number): Color;
  /** Create a Color from RGBA floats in [0, 1]. */
  rgba(this: void, r: number, g: number, b: number, a: number): Color;
};
