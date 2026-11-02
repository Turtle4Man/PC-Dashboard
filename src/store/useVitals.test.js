import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getVitals: vi.fn(),
  onVitals: vi.fn(),
  handler: null,
}));

vi.mock("../lib/ipc", () => ({
  getVitals: mocks.getVitals,
  onVitals: mocks.onVitals,
}));

async function loadStore() {
  vi.resetModules();
  return import("./useVitals");
}

beforeEach(() => {
  mocks.handler = null;
  mocks.getVitals.mockReset();
  mocks.getVitals.mockResolvedValue(null);
  mocks.onVitals.mockReset();
  mocks.onVitals.mockImplementation((callback) => {
    mocks.handler = callback;
    return Promise.resolve(() => {});
  });
});

describe("useVitals", () => {
  it("loads the latest sample on mount and exposes devices", async () => {
    mocks.getVitals.mockResolvedValue({
      ts: 1,
      mic: { muted: false },
      bt: [{ name: "G305", level: 80 }],
    });
    const { useVitals } = await loadStore();
    const { result } = renderHook(() => useVitals());

    await waitFor(() => expect(result.current.vitals).not.toBeNull());
    expect(result.current.vitals.devices).toEqual([
      { name: "G305", level: 80 },
    ]);
    expect(result.current.vitals.bt).toBeUndefined();
  });

  it("applies pushed samples and maps Bluetooth devices", async () => {
    const { useVitals } = await loadStore();
    const { result } = renderHook(() => useVitals());
    await waitFor(() => expect(mocks.handler).not.toBeNull());

    act(() => {
      mocks.handler({
        ts: 2,
        mic: { muted: false },
        bt: [{ name: "AirPods", level: 55 }],
      });
    });

    expect(result.current.vitals.ts).toBe(2);
    expect(result.current.vitals.devices).toEqual([
      { name: "AirPods", level: 55 },
    ]);
  });

  it("shows a microphone change without waiting for the next poll", async () => {
    mocks.getVitals.mockResolvedValue({
      ts: 1,
      mic: { muted: false },
      bt: [],
    });
    const { useVitals, updateMicMuted } = await loadStore();
    const { result } = renderHook(() => useVitals());
    await waitFor(() => expect(result.current.vitals).not.toBeNull());

    act(() => updateMicMuted(true));

    expect(result.current.vitals.mic.muted).toBe(true);
  });

  it("ignores a microphone update before the first sample", async () => {
    const { useVitals, updateMicMuted } = await loadStore();
    const { result } = renderHook(() => useVitals());

    act(() => updateMicMuted(true));

    expect(result.current.vitals).toBeNull();
  });
});
