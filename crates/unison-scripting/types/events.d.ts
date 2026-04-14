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

// Events are now accessed via unison.events.*
// See unison.d.ts for the UnisonEvents interface.
