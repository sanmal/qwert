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

// Must stay in sync with ExitCode in src-tauri/src/cli/exit_code.rs
export const EXIT_CODE = {
  SUCCESS: 0,
  GENERAL: 1,
  USAGE: 2,
  NOT_FOUND: 3,
  CONFLICT: 4,
  VALIDATION: 5,
} as const;
export type ExitCode = (typeof EXIT_CODE)[keyof typeof EXIT_CODE];
