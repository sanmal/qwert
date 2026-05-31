export const VIEW_MODE = {
  EDITOR: "editor",
  SPLIT: "split",
  PREVIEW: "preview",
} as const;
export type ViewMode = (typeof VIEW_MODE)[keyof typeof VIEW_MODE];

export const THEME = { DARK: "dark", LIGHT: "light" } as const;
export type Theme = (typeof THEME)[keyof typeof THEME];

export const SAVE_STATE = {
  SAVED: "saved",
  UNSAVED: "unsaved",
  SAVING: "saving",
} as const;
export type SaveState = (typeof SAVE_STATE)[keyof typeof SAVE_STATE];
