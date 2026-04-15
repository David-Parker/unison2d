/** Anchor positions for UI layout nodes. */
declare type Anchor =
  | "top_left" | "top" | "top_right"
  | "left" | "center" | "right"
  | "bottom_left" | "bottom" | "bottom_right";

/** Base fields shared by all UI node types. */
declare interface UINodeBase {
  /** Whether this node is visible. Defaults to true. */
  visible?: boolean;
}

/** Vertical layout container. */
declare interface UIColumnNode extends UINodeBase {
  /** Node type identifier. */
  type: "column";
  /** Screen anchor position for the column. */
  anchor?: Anchor;
  /** Vertical gap between children in pixels. */
  gap?: number;
  /** Padding inside the column in pixels. */
  padding?: number;
  /** Child nodes. */
  children?: UINode[];
}

/** Horizontal layout container. */
declare interface UIRowNode extends UINodeBase {
  /** Node type identifier. */
  type: "row";
  /** Screen anchor position for the row. */
  anchor?: Anchor;
  /** Horizontal gap between children in pixels. */
  gap?: number;
  /** Padding inside the row in pixels. */
  padding?: number;
  /** Child nodes. */
  children?: UINode[];
}

/** Panel with optional background color. */
declare interface UIPanelNode extends UINodeBase {
  /** Node type identifier. */
  type: "panel";
  /** Screen anchor position for the panel. */
  anchor?: Anchor;
  /** Padding inside the panel in pixels. */
  padding?: number;
  /** Background color as a hex integer. */
  bg_color?: number;
  /** Child nodes. */
  children?: UINode[];
}

/** Text label. */
declare interface UILabelNode extends UINodeBase {
  /** Node type identifier. */
  type: "label";
  /** Text content to display. */
  text: string;
  /** Font size in pixels. */
  font_size?: number;
  /** Font color as a hex integer. */
  font_color?: number;
}

/** Clickable button. The on_click value is emitted as a string event. */
declare interface UIButtonNode extends UINodeBase {
  /** Node type identifier. */
  type: "button";
  /** Button label text. */
  text: string;
  /** Event name to emit when clicked. Listen with unison.events.on(). */
  on_click?: string;
  /** Button width in pixels. */
  width?: number;
  /** Button height in pixels. */
  height?: number;
  /** Font size in pixels. */
  font_size?: number;
  /** Font color as a hex integer. */
  font_color?: number;
  /** Background color as a hex integer. */
  bg_color?: number;
}

/** Icon displaying a texture. */
declare interface UIIconNode extends UINodeBase {
  /** Node type identifier. */
  type: "icon";
  /** Texture ID from unison.assets.load_texture(). */
  texture: TextureId;
}

/** Horizontal progress bar with a value from 0 to 1. */
declare interface UIProgressBarNode extends UINodeBase {
  /** Node type identifier. */
  type: "progress_bar";
  /** Progress value in [0, 1]. */
  value: number;
  /** Bar width in pixels. */
  width?: number;
  /** Bar height in pixels. */
  height?: number;
}

/** Empty space used for layout purposes. */
declare interface UISpacerNode extends UINodeBase {
  /** Node type identifier. */
  type: "spacer";
  /** Size of the spacer in pixels. */
  value: number;
}

/** Union of all UI node types. */
declare type UINode =
  | UIColumnNode | UIRowNode | UIPanelNode | UILabelNode
  | UIButtonNode | UIIconNode | UIProgressBarNode | UISpacerNode;

/** UI handle created by unison.UI.new(). */
declare interface UI {
  /** Render one frame of UI from a nested node table. Call in render. */
  frame(this: UI, tree: UINode[]): void;
}

// UI factory is now unison.UI.new(fontId)
// See unison.d.ts.
