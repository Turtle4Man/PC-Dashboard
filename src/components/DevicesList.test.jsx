import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({ vitals: null }));

vi.mock("../store/useVitals", () => ({
  useVitals: () => ({ vitals: state.vitals }),
}));

import DevicesList from "./DevicesList";

function makeVitals(devices) {
  return { devices };
}

function rows(container) {
  return Array.from(container.querySelectorAll(".device")).map((element) => ({
    name: element.querySelector(".device__name").textContent,
    level: element.querySelector(".device__batt").className,
    value: element.querySelector(".battvalue__num")?.textContent ?? null,
  }));
}

beforeEach(() => {
  state.vitals = null;
});

describe("pending and empty states", () => {
  it("waits for the first sample", () => {
    render(<DevicesList />);

    expect(screen.getByText("…")).toBeInTheDocument();
  });

  it("shows a dash when nothing is connected", () => {
    state.vitals = makeVitals([]);
    render(<DevicesList />);

    expect(screen.getByText("—")).toBeInTheDocument();
  });
});

describe("battery levels", () => {
  it("shows the percentage and marks a low battery", () => {
    state.vitals = makeVitals([{ name: "G305 Lightspeed", level: 18 }]);
    const { container } = render(<DevicesList />);

    expect(rows(container)).toEqual([
      { name: "G305 Lightspeed", level: "device__batt level-bad", value: "18%" },
    ]);
  });

  it("keeps a healthy battery green", () => {
    state.vitals = makeVitals([{ name: "AirPods", level: 90 }]);
    const { container } = render(<DevicesList />);

    expect(rows(container)[0].level).toBe("device__batt level-ok");
    expect(rows(container)[0].value).toBe("90%");
  });

  it("falls back to the coarse receiver state", () => {
    state.vitals = makeVitals([{ name: "G915", state: "low" }]);
    const { container } = render(<DevicesList />);

    expect(rows(container)[0].level).toBe("device__batt level-bad");
    expect(screen.getByTitle("Batterie faible")).toBeInTheDocument();
  });

  it("marks a high receiver state as healthy", () => {
    state.vitals = makeVitals([{ name: "G915", state: "high" }]);
    const { container } = render(<DevicesList />);

    expect(rows(container)[0].level).toBe("device__batt level-ok");
    expect(screen.getByTitle("Batterie haute")).toBeInTheDocument();
  });

  it("shows an unknown battery without a colour alarm", () => {
    state.vitals = makeVitals([{ name: "Casque" }]);
    const { container } = render(<DevicesList />);

    expect(rows(container)[0].level).toBe("device__batt level-na");
    expect(rows(container)[0].value).toBeNull();
  });
});

describe("ordering", () => {
  it("sorts devices by name regardless of case", () => {
    state.vitals = makeVitals([
      { name: "clavier G915", level: 60 },
      { name: "AirPods", level: 80 },
      { name: "G305", level: 40 },
    ]);
    const { container } = render(<DevicesList />);

    expect(rows(container).map((row) => row.name)).toEqual([
      "AirPods",
      "clavier G915",
      "G305",
    ]);
  });
});
