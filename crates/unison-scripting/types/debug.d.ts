/** Development utilities. Available in all builds. */
declare namespace debug {
  /** Print varargs to stderr, joined with tabs. Values are converted via `tostring`. */
  function log(...args: any[]): void;
  /** Draw a 0.1-unit point at world position (x, y). Color is a hex integer. */
  function draw_point(x: number, y: number, color: number): void;
  /** Toggle physics debug visualization. Currently a no-op; reserved for future engine support. */
  function show_physics(enabled: boolean): void;
  /** Toggle FPS counter overlay. Currently a no-op; reserved for future engine support. */
  function show_fps(enabled: boolean): void;
}
