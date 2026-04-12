/** The game module returned by the entry script. */
declare interface Game {
  /** Called once after the VM is initialized and all globals are registered. */
  init?: (this: void) => void;
  /** Called each frame with the time delta in seconds. */
  update?: (this: void, dt: number) => void;
  /** Called each frame for drawing. */
  render?: (this: void) => void;
}
