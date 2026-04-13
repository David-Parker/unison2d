// {{PROJECT_NAME}} — entry point.

function on_load(): void {
    print("{{PROJECT_NAME}} loaded");
}

function on_update(dt: number): void {
    // game tick
}

function on_draw(): void {
    // render
}

// Expose to the engine as Lua globals.
(globalThis as any).on_load = on_load;
(globalThis as any).on_update = on_update;
(globalThis as any).on_draw = on_draw;
