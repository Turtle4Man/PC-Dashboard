import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_SETTINGS, updateSettings } from "../store/useSettings";
import { normalizeButtons } from "../lib/buttons";

vi.mock("../components/StatsList", () => ({
  default: () => <div data-testid="stats" />,
}));
vi.mock("../components/Clock", () => ({
  default: () => <div data-testid="clock" />,
}));
vi.mock("../components/MediaPill", () => ({
  default: () => <div data-testid="media" />,
}));
vi.mock("../components/ActionGrid", () => ({
  default: () => <div data-testid="actions" />,
}));
vi.mock("../components/DevicesList", () => ({
  default: () => <div data-testid="devices" />,
}));

import HomeScreen from "./HomeScreen";

function resetSettings(patch = {}) {
  updateSettings({
    ...DEFAULT_SETTINGS,
    buttons: normalizeButtons(DEFAULT_SETTINGS.buttons),
    ...patch,
  });
}

beforeEach(() => {
  resetSettings();
});

describe("HomeScreen", () => {
  it("shows every block by default", () => {
    render(<HomeScreen />);

    expect(screen.getByTestId("stats")).toBeInTheDocument();
    expect(screen.getByTestId("clock")).toBeInTheDocument();
    expect(screen.getByTestId("media")).toBeInTheDocument();
    expect(screen.getByTestId("actions")).toBeInTheDocument();
    expect(screen.getByTestId("devices")).toBeInTheDocument();
  });

  it("hides the statistics but keeps the column aligned", () => {
    resetSettings({ showStats: false });
    const { container } = render(<HomeScreen />);

    expect(screen.queryByTestId("stats")).not.toBeInTheDocument();
    expect(container.querySelector(".stats")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });

  it("hides the clock independently of the media controls", () => {
    resetSettings({ showClock: false });
    render(<HomeScreen />);

    expect(screen.queryByTestId("clock")).not.toBeInTheDocument();
    expect(screen.getByTestId("media")).toBeInTheDocument();
  });

  it("hides the action buttons", () => {
    resetSettings({ showTiles: false });
    render(<HomeScreen />);

    expect(screen.queryByTestId("actions")).not.toBeInTheDocument();
  });

  it("hides the devices but keeps the column aligned", () => {
    resetSettings({ showDevices: false });
    const { container } = render(<HomeScreen />);

    expect(screen.queryByTestId("devices")).not.toBeInTheDocument();
    expect(container.querySelector(".devices")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });
});
