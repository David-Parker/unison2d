/** Collision contact info passed to collision callbacks. */
declare interface CollisionInfo {
  /** Contact normal X component. */
  normal_x: number;
  /** Contact normal Y component. */
  normal_y: number;
  /** Penetration depth. */
  penetration: number;
  /** Contact point X coordinate. */
  contact_x: number;
  /** Contact point Y coordinate. */
  contact_y: number;
}

/** Opaque object ID (number). */
declare type ObjectId = number;

/** String-keyed pub/sub event bus and collision callbacks. */
declare const events: {
  /** Register a callback for a named event. Multiple listeners are allowed. */
  on(name: string, callback: (data?: any) => void): void;
  /** Emit a named event with optional data. Callbacks fire at end of frame. */
  emit(name: string, data?: any): void;
  /** Called for every collision pair each frame. */
  on_collision(callback: (a: ObjectId, b: ObjectId, info: CollisionInfo) => void): void;
  /** Called when the given object collides with anything. */
  on_collision_for(id: ObjectId, callback: (other: ObjectId, info: CollisionInfo) => void): void;
  /** Called when objects a and b collide. */
  on_collision_between(a: ObjectId, b: ObjectId, callback: (info: CollisionInfo) => void): void;
  /** Clear all string-keyed event handlers and pending events. Collision handlers are NOT cleared. */
  clear(): void;
};
