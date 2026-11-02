import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, listen, openUrl } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

import {
  getAutostart,
  getVitals,
  onKeyboardLock,
  onVitals,
  openDevicesSettings,
  openMediaSource,
  openWebShortcut,
  sendHotkey,
  setAutostart,
  setKeyboardLocked,
  takeScreenshot,
  toggleMic,
} from "./ipc";

beforeEach(() => {
  invoke.mockReset();
  listen.mockReset();
  openUrl.mockReset();
  invoke.mockResolvedValue(undefined);
  openUrl.mockResolvedValue(undefined);
  listen.mockResolvedValue(() => {});
});

describe("backend commands", () => {
  it("reads vitals without arguments", () => {
    getVitals();
    expect(invoke).toHaveBeenCalledWith("get_vitals");
  });

  it("passes the keyboard hold duration as camelCase", () => {
    setKeyboardLocked(true, 60);
    expect(invoke).toHaveBeenCalledWith("set_keyboard_locked", {
      locked: true,
      holdSeconds: 60,
    });
  });

  it("can lock the keyboard without a duration", () => {
    setKeyboardLocked(true, 0);
    expect(invoke).toHaveBeenCalledWith("set_keyboard_locked", {
      locked: true,
      holdSeconds: 0,
    });
  });

  it("maps the screenshot scope to the backend flag", () => {
    takeScreenshot(true);
    takeScreenshot(false);

    expect(invoke).toHaveBeenNthCalledWith(1, "take_screenshot", {
      allScreens: true,
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "take_screenshot", {
      allScreens: false,
    });
  });

  it("toggles the microphone through its dedicated command", () => {
    toggleMic();
    expect(invoke).toHaveBeenCalledWith("toggle_mic");
  });

  it("reads and writes the autostart setting", () => {
    getAutostart();
    setAutostart(false);

    expect(invoke).toHaveBeenNthCalledWith(1, "get_autostart");
    expect(invoke).toHaveBeenNthCalledWith(2, "set_autostart", { enabled: false });
  });

  it("sends the hotkey shortcut", () => {
    sendHotkey("Ctrl+Shift+M");
    expect(invoke).toHaveBeenCalledWith("send_hotkey", {
      shortcut: "Ctrl+Shift+M",
    });
  });

  it("opens the media source and the Windows devices page", () => {
    openMediaSource();
    openDevicesSettings();

    expect(invoke).toHaveBeenCalledWith("open_media_source");
    expect(openUrl).toHaveBeenCalledWith("ms-settings:devices");
  });
});

describe("event listeners", () => {
  it("forwards vitals payloads", () => {
    const callback = vi.fn();
    onVitals(callback);

    expect(listen).toHaveBeenCalledWith("vitals", expect.any(Function));
    const handler = listen.mock.calls[0][1];
    handler({ payload: { ts: 42 } });
    expect(callback).toHaveBeenCalledWith({ ts: 42 });
  });

  it("forwards the keyboard lock state", () => {
    const callback = vi.fn();
    onKeyboardLock(callback);

    expect(listen).toHaveBeenCalledWith("keyboard-lock", expect.any(Function));
    const handler = listen.mock.calls[0][1];
    handler({ payload: false });
    expect(callback).toHaveBeenCalledWith(false);
  });
});

describe("openWebShortcut", () => {
  it("accepts a bare domain and adds https", async () => {
    await openWebShortcut("exemple.com");

    expect(openUrl).toHaveBeenCalledWith("https://exemple.com");
  });

  it("trims the address and keeps an explicit scheme", async () => {
    await openWebShortcut("  http://exemple.com/page  ");

    expect(openUrl).toHaveBeenCalledWith("http://exemple.com/page");
  });

  it("rejects an empty address without opening anything", async () => {
    await expect(openWebShortcut("   ")).rejects.toThrow(
      "Aucune adresse web configurée.",
    );
    expect(openUrl).not.toHaveBeenCalled();
  });
});
