import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({ hourFormat: "24" }));

vi.mock("../store/useSettings", () => ({
  useSettings: () => ({ settings: { hourFormat: state.hourFormat } }),
}));

import Clock from "./Clock";

beforeEach(() => {
  vi.useFakeTimers();
  state.hourFormat = "24";
});

afterEach(() => {
  vi.useRealTimers();
});

describe("Clock", () => {
  it("shows 24-hour time by default", () => {
    vi.setSystemTime(new Date(2026, 10, 2, 9, 5, 0));
    render(<Clock />);

    expect(screen.getByText("09:05")).toBeInTheDocument();
  });

  it("shows 12-hour time with an AM suffix", () => {
    state.hourFormat = "12";
    vi.setSystemTime(new Date(2026, 10, 2, 9, 5, 0));
    render(<Clock />);

    expect(screen.getByText("9:05 AM")).toBeInTheDocument();
  });

  it("shows midnight as 12 AM and noon as 12 PM", () => {
    state.hourFormat = "12";
    vi.setSystemTime(new Date(2026, 10, 2, 0, 5, 0));
    const { unmount } = render(<Clock />);
    expect(screen.getByText("12:05 AM")).toBeInTheDocument();
    unmount();

    vi.setSystemTime(new Date(2026, 10, 2, 12, 5, 0));
    render(<Clock />);
    expect(screen.getByText("12:05 PM")).toBeInTheDocument();
  });

  it("advances every second", () => {
    vi.setSystemTime(new Date(2026, 10, 2, 9, 5, 0));
    render(<Clock />);
    expect(screen.getByText("09:05")).toBeInTheDocument();

    vi.setSystemTime(new Date(2026, 10, 2, 9, 6, 0));
    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(screen.getByText("09:06")).toBeInTheDocument();
  });
});
