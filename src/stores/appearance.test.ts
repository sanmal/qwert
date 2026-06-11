import { describe, it, expect, beforeEach } from "vitest";
import { appearanceStore } from "./appearance";

describe("appearanceStore.reapplyAppearance (C2 hot reload)", () => {
  beforeEach(() => {
    // Reset the document root between cases.
    document.documentElement.removeAttribute("style");
    delete document.documentElement.dataset.theme;
  });

  it("applies custom CSS vars on re-apply", () => {
    appearanceStore.reapplyAppearance({ "--qw-fg": "#111111", "--qw-bg": "#eeeeee" });
    const el = document.documentElement;
    expect(el.style.getPropertyValue("--qw-fg")).toBe("#111111");
    expect(el.style.getPropertyValue("--qw-bg")).toBe("#eeeeee");
  });

  it("applies a preset via the data-theme attribute", () => {
    appearanceStore.reapplyAppearance({ "data-theme": "dark" });
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("clears stale managed vars when switching from custom colors to a preset", () => {
    appearanceStore.reapplyAppearance({ "--qw-fg": "#111111", "--qw-bg": "#eeeeee" });
    // Now a preset edit arrives — the old --qw-fg/--qw-bg must not linger.
    appearanceStore.reapplyAppearance({ "data-theme": "dark" });
    const el = document.documentElement;
    expect(el.style.getPropertyValue("--qw-fg")).toBe("");
    expect(el.style.getPropertyValue("--qw-bg")).toBe("");
    expect(el.dataset.theme).toBe("dark");
  });

  it("clears data-theme when switching from a preset back to custom colors", () => {
    appearanceStore.reapplyAppearance({ "data-theme": "dark" });
    appearanceStore.reapplyAppearance({ "--qw-fg": "#000000" });
    expect(document.documentElement.dataset.theme).toBeUndefined();
    expect(document.documentElement.style.getPropertyValue("--qw-fg")).toBe("#000000");
  });

  it("ignores unknown keys", () => {
    appearanceStore.reapplyAppearance({ "evil-key": "x", "--qw-fg": "#222222" });
    const el = document.documentElement;
    expect(el.style.getPropertyValue("evil-key")).toBe("");
    expect(el.style.getPropertyValue("--qw-fg")).toBe("#222222");
  });
});

describe("appearanceStore C3: warning signals", () => {
  beforeEach(() => {
    document.documentElement.removeAttribute("style");
    delete document.documentElement.dataset.theme;
  });

  it("setAppearanceWarning stores the warning text", () => {
    appearanceStore.setAppearanceWarning("parse error: line 3, column 7");
    expect(appearanceStore.currentWarning()).toBe("parse error: line 3, column 7");
  });

  it("reapplyAppearance clears the warning", () => {
    appearanceStore.setAppearanceWarning("some warning");
    appearanceStore.reapplyAppearance({ "--qw-fg": "#000000" });
    expect(appearanceStore.currentWarning()).toBeNull();
  });

  it("warning does not alter CSS vars (previous state preserved)", () => {
    // Apply a known config first.
    appearanceStore.reapplyAppearance({ "--qw-fg": "#111111" });
    const before = document.documentElement.style.getPropertyValue("--qw-fg");
    // A warning arrives (hot-reload error path).
    appearanceStore.setAppearanceWarning("bad toml");
    // CSS vars must be unchanged.
    expect(document.documentElement.style.getPropertyValue("--qw-fg")).toBe(before);
  });
});
