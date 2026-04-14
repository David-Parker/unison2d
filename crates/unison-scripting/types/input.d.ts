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

// Input state is now accessed via unison.input.*
// See unison.d.ts for the UnisonInput interface.
