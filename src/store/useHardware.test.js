import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getHardware: vi.fn(),
}));

vi.mock("../lib/ipc", () => ({
  getHardware: mocks.getHardware,
}));

async function loadStore() {
  vi.resetModules();
  return import("./useHardware");
}

const PAYLOAD = { components: [{ label: "Processeur", name: "i5" }], peripherals: [] };

beforeEach(() => {
  mocks.getHardware.mockReset();
});

describe("useHardware", () => {
  it("loads the hardware description on mount", async () => {
    mocks.getHardware.mockResolvedValue(PAYLOAD);
    const { useHardware } = await loadStore();
    const { result } = renderHook(() => useHardware());

    await waitFor(() => expect(result.current.hardware).toEqual(PAYLOAD));
    expect(result.current.error).toBeNull();
  });

  it("keeps the last description when a refresh fails", async () => {
    vi.useFakeTimers();
    mocks.getHardware
      .mockResolvedValueOnce(PAYLOAD)
      .mockRejectedValueOnce(new Error("enumeration failed"));
    const { useHardware } = await loadStore();
    const { result, unmount } = renderHook(() => useHardware());

    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(result.current.hardware).toEqual(PAYLOAD);

    await act(() => vi.advanceTimersByTimeAsync(3000));
    expect(result.current.hardware).toEqual(PAYLOAD);
    expect(result.current.error).toContain("enumeration failed");

    unmount();
    vi.useRealTimers();
  });
});
