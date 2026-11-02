import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  state: { vitals: null, settings: null, keyboardHandler: null },
  getKeyboardLocked: vi.fn(),
  onKeyboardLock: vi.fn(),
  openDevicesSettings: vi.fn(),
  openWebShortcut: vi.fn(),
  sendHotkey: vi.fn(),
  setKeyboardLocked: vi.fn(),
  takeScreenshot: vi.fn(),
  toggleMic: vi.fn(),
  updateMicMuted: vi.fn(),
}));

vi.mock("../lib/ipc", () => ({
  getKeyboardLocked: mocks.getKeyboardLocked,
  onKeyboardLock: mocks.onKeyboardLock,
  openDevicesSettings: mocks.openDevicesSettings,
  openWebShortcut: mocks.openWebShortcut,
  sendHotkey: mocks.sendHotkey,
  setKeyboardLocked: mocks.setKeyboardLocked,
  takeScreenshot: mocks.takeScreenshot,
  toggleMic: mocks.toggleMic,
}));

vi.mock("../store/useVitals", () => ({
  useVitals: () => ({ vitals: mocks.state.vitals }),
  updateMicMuted: mocks.updateMicMuted,
}));

vi.mock("../store/useSettings", () => ({
  useSettings: () => ({ settings: mocks.state.settings, updateSettings: vi.fn() }),
}));

import ActionGrid from "./ActionGrid";

function emptyButton() {
  return { type: "empty", value: "", icon: null };
}

function makeSettings(patch = {}) {
  return {
    keyboardLockSeconds: 60,
    screenshotScope: "all",
    buttons: [
      { type: "mic", value: "", icon: null },
      { type: "keyboard", value: "", icon: null },
      { type: "screenshot", value: "", icon: null },
      { type: "devices", value: "", icon: null },
      { type: "web", value: "exemple.com", icon: null },
      { type: "hotkey", value: "Ctrl+Shift+M", icon: null },
      emptyButton(),
      emptyButton(),
      emptyButton(),
    ],
    ...patch,
  };
}

function renderGrid(settings = makeSettings(), vitals = { mic: { muted: false } }) {
  mocks.state.settings = settings;
  mocks.state.vitals = vitals;
  return render(<ActionGrid />);
}

beforeEach(() => {
  mocks.state.keyboardHandler = null;
  mocks.getKeyboardLocked.mockResolvedValue(false);
  mocks.onKeyboardLock.mockImplementation((callback) => {
    mocks.state.keyboardHandler = callback;
    return Promise.resolve(() => {});
  });
  mocks.setKeyboardLocked.mockResolvedValue(true);
  mocks.takeScreenshot.mockResolvedValue({});
  mocks.openDevicesSettings.mockResolvedValue(undefined);
  mocks.openWebShortcut.mockResolvedValue(undefined);
  mocks.sendHotkey.mockResolvedValue(undefined);
  mocks.toggleMic.mockResolvedValue({ muted: true, discordSynced: true });
  mocks.updateMicMuted.mockReset();
  vi.spyOn(console, "error").mockImplementation(() => {});
  vi.spyOn(console, "warn").mockImplementation(() => {});
});

describe("tile titles", () => {
  it("names the microphone action after its current state", async () => {
    const { unmount } = renderGrid(makeSettings(), { mic: { muted: false } });
    expect(screen.getByTitle("Couper le micro")).toBeInTheDocument();
    unmount();

    renderGrid(makeSettings(), { mic: { muted: true } });
    expect(screen.getByTitle("Réactiver le micro")).toBeInTheDocument();
  });

  it("shows the configured keyboard duration", () => {
    const { unmount } = renderGrid(makeSettings({ keyboardLockSeconds: 60 }));
    expect(
      screen.getByTitle("Verrouiller le clavier pendant 1 min"),
    ).toBeInTheDocument();
    unmount();

    renderGrid(makeSettings({ keyboardLockSeconds: 30 }));
    expect(
      screen.getByTitle("Verrouiller le clavier pendant 30 s"),
    ).toBeInTheDocument();
  });

  it("does not promise a duration for the unlimited lock", () => {
    renderGrid(makeSettings({ keyboardLockSeconds: 0 }));

    expect(
      screen.getByTitle("Verrouiller le clavier sans limite de durée"),
    ).toBeInTheDocument();
  });

  it("describes the screenshot scope", () => {
    const { unmount } = renderGrid(makeSettings({ screenshotScope: "all" }));
    expect(
      screen.getByTitle("Capture d'écran — tous les écrans"),
    ).toBeInTheDocument();
    unmount();

    renderGrid(makeSettings({ screenshotScope: "primary" }));
    expect(
      screen.getByTitle("Capture d'écran — écran principal"),
    ).toBeInTheDocument();
  });

  it("shows the configured web and hotkey shortcuts", () => {
    renderGrid();

    expect(screen.getByTitle("Ouvrir exemple.com")).toBeInTheDocument();
    expect(screen.getByTitle("Raccourci Ctrl+Shift+M")).toBeInTheDocument();
  });

  it("only renders configured buttons", () => {
    renderGrid();

    expect(screen.getAllByRole("button")).toHaveLength(6);
  });
});

describe("keyboard lock", () => {
  it("locks with the configured duration and follows the backend", async () => {
    renderGrid(makeSettings({ keyboardLockSeconds: 60 }));

    fireEvent.click(
      screen.getByTitle("Verrouiller le clavier pendant 1 min"),
    );

    expect(mocks.setKeyboardLocked).toHaveBeenCalledWith(true, 60);
    const tile = await screen.findByTitle("Réactiver le clavier");
    expect(tile).toHaveClass("tile--active");
  });

  it("passes zero when the lock has no duration", async () => {
    renderGrid(makeSettings({ keyboardLockSeconds: 0 }));

    fireEvent.click(
      screen.getByTitle("Verrouiller le clavier sans limite de durée"),
    );

    expect(mocks.setKeyboardLocked).toHaveBeenCalledWith(true, 0);
  });

  it("goes back to unlocked when the backend lifts the lock", async () => {
    renderGrid();
    fireEvent.click(
      screen.getByTitle("Verrouiller le clavier pendant 1 min"),
    );
    await screen.findByTitle("Réactiver le clavier");

    act(() => {
      mocks.state.keyboardHandler(false);
    });

    expect(
      screen.getByTitle("Verrouiller le clavier pendant 1 min"),
    ).toBeInTheDocument();
  });
});

describe("microphone", () => {
  it("toggles optimistically, then confirms the backend state", async () => {
    mocks.toggleMic.mockResolvedValue({ muted: true, discordSynced: true });
    renderGrid();

    fireEvent.click(screen.getByTitle("Couper le micro"));

    await waitFor(() => expect(mocks.toggleMic).toHaveBeenCalledTimes(1));
    expect(mocks.updateMicMuted.mock.calls).toEqual([[true], [true]]);
  });

  it("warns but keeps the Windows mute when Discord fails", async () => {
    mocks.toggleMic.mockResolvedValue({ muted: true, discordSynced: false });
    renderGrid();

    const tile = screen.getByTitle("Couper le micro");
    fireEvent.click(tile);

    await waitFor(() => expect(tile).toHaveClass("tile--warn"));
    expect(tile).not.toHaveClass("tile--err");
    expect(mocks.updateMicMuted).toHaveBeenLastCalledWith(true);
  });

  it("rolls the icon back when Windows refuses", async () => {
    mocks.toggleMic.mockRejectedValue(new Error("no device"));
    renderGrid();

    const tile = screen.getByTitle("Couper le micro");
    fireEvent.click(tile);

    await waitFor(() => expect(tile).toHaveClass("tile--err"));
    expect(mocks.updateMicMuted.mock.calls).toEqual([[true], [false]]);
  });
});

describe("other actions", () => {
  it("captures the configured screens", async () => {
    const { unmount } = renderGrid(makeSettings({ screenshotScope: "all" }));
    fireEvent.click(screen.getByTitle("Capture d'écran — tous les écrans"));
    await waitFor(() =>
      expect(mocks.takeScreenshot).toHaveBeenCalledWith(true),
    );
    unmount();
    mocks.takeScreenshot.mockClear();

    renderGrid(makeSettings({ screenshotScope: "primary" }));
    fireEvent.click(screen.getByTitle("Capture d'écran — écran principal"));
    await waitFor(() =>
      expect(mocks.takeScreenshot).toHaveBeenCalledWith(false),
    );
  });

  it("opens the Windows devices page", async () => {
    renderGrid();

    fireEvent.click(screen.getByTitle("Réglages Bluetooth et appareils"));

    await waitFor(() => expect(mocks.openDevicesSettings).toHaveBeenCalled());
  });

  it("opens the configured web shortcut", async () => {
    renderGrid();

    fireEvent.click(screen.getByTitle("Ouvrir exemple.com"));

    await waitFor(() =>
      expect(mocks.openWebShortcut).toHaveBeenCalledWith("exemple.com"),
    );
  });

  it("rejects an empty hotkey without calling the backend", async () => {
    const settings = makeSettings();
    settings.buttons[5] = { type: "hotkey", value: "   ", icon: null };
    renderGrid(settings);

    const tile = screen.getByTitle("Raccourci clavier — non configuré");
    fireEvent.click(tile);

    await waitFor(() => expect(tile).toHaveClass("tile--err"));
    expect(mocks.sendHotkey).not.toHaveBeenCalled();
  });

  it("shows an error when the web shortcut cannot be opened", async () => {
    mocks.openWebShortcut.mockRejectedValue(
      new Error("Aucune adresse web configurée."),
    );
    renderGrid();

    const tile = screen.getByTitle("Ouvrir exemple.com");
    fireEvent.click(tile);

    await waitFor(() => expect(tile).toHaveClass("tile--err"));
  });
});
