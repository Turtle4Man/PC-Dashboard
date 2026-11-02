import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const state = vi.hoisted(() => ({ vitals: null }));

vi.mock("../store/useVitals", () => ({
  useVitals: () => ({ vitals: state.vitals }),
}));

import StatsList from "./StatsList";

const GB = 1024 ** 3;

function makeVitals(patch = {}) {
  return {
    cpu: { global: 12.6 },
    gpus: [{ name: "GPU", usage: 3.2 }],
    mem: { used: 8 * GB, total: 16 * GB },
    net_ping: [{ name: "Internet", ms: 6.9 }],
    disks: [
      { name: "_Total", resp_ms: 0.2 },
      { name: "J:", resp_ms: 0.45 },
      { name: "C:", resp_ms: 0.32 },
    ],
    ...patch,
  };
}

function row(label) {
  return screen.getByText(label).closest(".stat-row");
}

beforeEach(() => {
  state.vitals = null;
});

describe("pending state", () => {
  it("shows placeholders until the first sample", () => {
    render(<StatsList />);

    expect(screen.getByText("CPU")).toBeInTheDocument();
    expect(row("CPU")).toHaveTextContent("…");
    expect(row("Réseau")).toHaveTextContent("…");
  });
});

describe("system usage", () => {
  it("rounds CPU, GPU and RAM percentages", () => {
    state.vitals = makeVitals({
      gpus: [
        { name: "iGPU", usage: 1.2 },
        { name: "dGPU", usage: 44.6 },
      ],
    });
    render(<StatsList />);

    expect(row("CPU")).toHaveTextContent("13 %");
    expect(row("GPU")).toHaveTextContent("45 %");
    expect(row("RAM")).toHaveTextContent("50 %");
  });

  it("has no GPU reading when none is reported", () => {
    state.vitals = makeVitals({ gpus: [] });
    render(<StatsList />);

    expect(row("GPU")).toHaveTextContent("0 %");
  });

  it("marks high usage as a problem", () => {
    state.vitals = makeVitals({
      cpu: { global: 95 },
      mem: { used: 15 * GB, total: 16 * GB },
    });
    render(<StatsList />);

    expect(row("CPU")).toHaveClass("level-bad");
    expect(row("RAM")).toHaveClass("level-bad");
    expect(row("GPU")).toHaveClass("level-ok");
  });
});

describe("network", () => {
  it("shows a single ping", () => {
    state.vitals = makeVitals();
    render(<StatsList />);

    expect(row("Réseau")).toHaveTextContent("6.9 ms");
  });

  it("lists every interface when several are measured", () => {
    state.vitals = makeVitals({
      net_ping: [
        { name: "Ethernet", ms: 4 },
        { name: "Wi-Fi", ms: 120 },
      ],
    });
    render(<StatsList />);

    expect(screen.getByText("Réseau")).toHaveClass("stat-row__label");
    expect(screen.getByText("Ethernet").closest(".stat-row")).toHaveTextContent(
      "4.0 ms",
    );
    expect(screen.getByText("Wi-Fi").closest(".stat-row")).toHaveTextContent(
      "120 ms",
    );
  });
});

describe("disks", () => {
  it("hides the aggregate row and sorts drives by name", () => {
    state.vitals = makeVitals();
    const { container } = render(<StatsList />);

    const labels = Array.from(
      container.querySelectorAll(".stat-group--disk .stat-row__label"),
    ).map((element) => element.textContent);

    expect(screen.queryByText("_Total")).not.toBeInTheDocument();
    expect(labels).toEqual(["Disques", "C:", "J:"]);
    expect(screen.getByText("C:").closest(".stat-row")).toHaveTextContent(
      "0.32 ms",
    );
  });

  it("keeps a single disk on one line", () => {
    state.vitals = makeVitals({ disks: [{ name: "C:", resp_ms: 0.5 }] });
    render(<StatsList />);

    expect(row("Disque")).toHaveTextContent("0.50 ms");
  });
});
