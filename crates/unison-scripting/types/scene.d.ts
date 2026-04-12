/**
 * A scene is a table with optional lifecycle hooks.
 * When active via `engine.set_scene()` or `engine.switch_scene()`,
 * the scene's update/render replace game.update/game.render.
 */
declare interface Scene {
  /** Called when the scene becomes active. */
  on_enter?: () => void;
  /** Called each frame with time delta in seconds. */
  update?: (dt: number) => void;
  /** Called each frame for drawing. */
  render?: () => void;
  /** Called when switching away from this scene. */
  on_exit?: () => void;
}
