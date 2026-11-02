import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { BUTTON_COUNT } from "../lib/buttons";

const STORAGE_KEY = "pc-dashboard.settings";

async function loadSettings(stored) {
  vi.resetModules();
  localStorage.clear();
  if (stored !== undefined) {
    localStorage.setItem(STORAGE_KEY, stored);
  }
  return import("./useSettings");
}

beforeEach(() => {
  localStorage.clear();
});

describe("default settings", () => {
  it("starts from the documented defaults when nothing is stored", async () => {
    const { getSettings } = await loadSettings();
    const settings = getSettings();

    expect(settings.theme).toBe("system");
    expect(settings.screenshotScope).toBe("all");
    expect(settings.hourFormat).toBe("24");
    expect(settings.keyboardLockSeconds).toBe(60);
    expect(settings.showStats).toBe(true);
    expect(settings.showClock).toBe(true);
    expect(settings.showTiles).toBe(true);
    expect(settings.showDevices).toBe(true);
    expect(settings.buttons).toHaveLength(BUTTON_COUNT);
  });

  it("ignores corrupted storage instead of crashing", async () => {
    const { getSettings } = await loadSettings("{not json");

    expect(getSettings().theme).toBe("system");
    expect(getSettings().keyboardLockSeconds).toBe(60);
  });
});

describe("normalization", () => {
  it("replaces unknown values with the defaults", async () => {
    const { getSettings } = await loadSettings(
      JSON.stringify({
        theme: "neon",
        screenshotScope: "left",
        hourFormat: "36",
        keyboardLockSeconds: 42,
        showStats: "yes",
        buttons: "not an array",
      }),
    );
    const settings = getSettings();

    expect(settings.theme).toBe("system");
    expect(settings.screenshotScope).toBe("all");
    expect(settings.hourFormat).toBe("24");
    expect(settings.keyboardLockSeconds).toBe(60);
    expect(settings.showStats).toBe(true);
    expect(settings.buttons).toHaveLength(BUTTON_COUNT);
    expect(settings.buttons.every((button) => button.type === "empty")).toBe(
      true,
    );
  });

  it("keeps every valid choice, including the unlimited lock", async () => {
    const stored = {
      theme: "dark",
      screenshotScope: "primary",
      hourFormat: "12",
      keyboardLockSeconds: 0,
      showStats: false,
      showClock: false,
      showTiles: false,
      showDevices: false,
      buttons: [
        { type: "mic", value: "" },
        { type: "hotkey", value: "Ctrl+Shift+M" },
      ],
    };
    const { getSettings } = await loadSettings(JSON.stringify(stored));
    const settings = getSettings();

    expect(settings).toMatchObject({
      theme: "dark",
      screenshotScope: "primary",
      hourFormat: "12",
      keyboardLockSeconds: 0,
      showStats: false,
      showClock: false,
      showTiles: false,
      showDevices: false,
    });
    expect(settings.buttons[1]).toMatchObject({
      type: "hotkey",
      value: "Ctrl+Shift+M",
    });
    expect(settings.buttons).toHaveLength(BUTTON_COUNT);
  });
});

describe("updateSettings", () => {
  it("merges the patch, persists it and returns true", async () => {
    const { getSettings, updateSettings } = await loadSettings();

    const persisted = updateSettings({ showClock: false, hourFormat: "12" });

    expect(persisted).toBe(true);
    expect(getSettings().showClock).toBe(false);
    expect(getSettings().hourFormat).toBe("12");
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY))).toMatchObject({
      showClock: false,
      hourFormat: "12",
    });
  });

  it("keeps working in memory when storage is full", async () => {
    const { getSettings, updateSettings } = await loadSettings();
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });

    const persisted = updateSettings({ showStats: false });

    expect(persisted).toBe(false);
    expect(getSettings().showStats).toBe(false);
  });

  it("applies a theme change immediately", async () => {
    const { updateSettings } = await loadSettings();

    updateSettings({ theme: "dark" });
    expect(document.documentElement.dataset.theme).toBe("dark");

    updateSettings({ theme: "system" });
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });

  it("notifies mounted components", async () => {
    const { useSettings } = await loadSettings();
    const { result } = renderHook(() => useSettings());

    act(() => {
      result.current.updateSettings({ showDevices: false });
    });

    expect(result.current.settings.showDevices).toBe(false);
  });
});
