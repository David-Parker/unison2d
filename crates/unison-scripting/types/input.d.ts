/** Key name strings recognized by the input system. */
declare type KeyName =
  | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
  | "Space" | "Enter" | "Escape" | "Tab" | "Backspace"
  | "ShiftLeft" | "ShiftRight" | "ControlLeft" | "ControlRight"
  | "AltLeft" | "AltRight"
  | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J"
  | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T"
  | "U" | "V" | "W" | "X" | "Y" | "Z"
  | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  | "Digit0" | "Digit1" | "Digit2" | "Digit3" | "Digit4"
  | "Digit5" | "Digit6" | "Digit7" | "Digit8" | "Digit9";

/** Touch position from a touch-start event. */
declare interface TouchPosition {
  /** X coordinate in screen space. */
  x: number;
  /** Y coordinate in screen space. */
  y: number;
}

/** Input state, refreshed automatically before each update. */
declare const input: {
  /** True while the key is held down. */
  is_key_pressed(key: KeyName): boolean;
  /** True only on the frame the key was first pressed. */
  is_key_just_pressed(key: KeyName): boolean;
  /** Horizontal axis in [-1, 1] from joystick or touch joystick. */
  axis_x(): number;
  /** Vertical axis in [-1, 1] from joystick or touch joystick. */
  axis_y(): number;
  /** Array of new touch-start positions this frame. */
  touches_just_began(): TouchPosition[];
};
