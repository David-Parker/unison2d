/** Lua built-in globals available at runtime. */

/**
 * Load a Lua module by name.
 *
 * Use this for lazy/deferred loading inside callbacks to avoid circular
 * dependencies that top-level `import` would create.
 *
 * ```ts
 * events.on("return_to_menu", () => {
 *     engine.switch_scene(require("scenes.menu"));
 * });
 * ```
 */
declare function require(modname: string): any;

/** Alias for LuaMultiReturn — a Lua function returning multiple values. */
type Tuple<T extends any[]> = LuaMultiReturn<T>;

/** Lua's built-in `print` — writes values to stdout, separated by tabs. */
declare function print(...values: any[]): void;
