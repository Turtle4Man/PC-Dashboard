import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  state: { vitals: null },
  toggleMedia: vi.fn(),
  mediaNext: vi.fn(),
  mediaPrevious: vi.fn(),
  openMediaSource: vi.fn(),
}));

vi.mock("../store/useVitals", () => ({
  useVitals: () => ({ vitals: mocks.state.vitals }),
}));

vi.mock("../lib/ipc", () => ({
  toggleMedia: mocks.toggleMedia,
  mediaNext: mocks.mediaNext,
  mediaPrevious: mocks.mediaPrevious,
  openMediaSource: mocks.openMediaSource,
}));

import MediaPill from "./MediaPill";

const THUMB = "data:image/png;base64,AAA";

function setMedia(media) {
  mocks.state.vitals = { media };
}

beforeEach(() => {
  mocks.state.vitals = null;
  mocks.toggleMedia.mockResolvedValue(undefined);
  mocks.mediaNext.mockResolvedValue(undefined);
  mocks.mediaPrevious.mockResolvedValue(undefined);
  mocks.openMediaSource.mockResolvedValue(undefined);
});

describe("idle state", () => {
  it("waits for vitals before saying anything", () => {
    render(<MediaPill />);

    expect(screen.getByRole("button", { name: "…" })).toBeDisabled();
    for (const button of screen.getAllByRole("button")) {
      expect(button).toBeDisabled();
    }
  });

  it("says nothing is playing once a session was reported", () => {
    setMedia({});
    render(<MediaPill />);

    const title = screen.getByRole("button", { name: "Aucune lecture" });
    expect(title).toBeDisabled();
    expect(screen.getAllByRole("button")).toHaveLength(4);
  });
});

describe("active session", () => {
  it("shows the track and enables the controls", () => {
    setMedia({ playing: true, title: "Classic Pokemon Lofi", artist: "chill" });
    render(<MediaPill />);

    expect(screen.getByText("Classic Pokemon Lofi")).toBeInTheDocument();
    expect(screen.getByText("chill")).toBeInTheDocument();
    expect(screen.getByTitle("Mettre en pause")).toBeEnabled();
    expect(screen.getByTitle("Piste précédente")).toBeEnabled();
    expect(screen.getByTitle("Piste suivante")).toBeEnabled();
    expect(screen.getByTitle("Ouvrir la source")).toBeEnabled();
  });

  it("offers to resume a paused track", () => {
    setMedia({ playing: false, title: "Paused song" });
    render(<MediaPill />);

    expect(screen.getByTitle("Reprendre la lecture")).toBeInTheDocument();
    expect(screen.queryByTitle("Mettre en pause")).not.toBeInTheDocument();
  });

  it("sends the transport commands", async () => {
    setMedia({ playing: true, title: "Song" });
    render(<MediaPill />);

    const previous = screen.getByTitle("Piste précédente");
    fireEvent.click(previous);
    await waitFor(() => expect(mocks.mediaPrevious).toHaveBeenCalledTimes(1));

    const next = screen.getByTitle("Piste suivante");
    fireEvent.click(next);
    await waitFor(() => expect(mocks.mediaNext).toHaveBeenCalledTimes(1));

    const toggle = screen.getByTitle("Mettre en pause");
    fireEvent.click(toggle);
    await waitFor(() => expect(mocks.toggleMedia).toHaveBeenCalledTimes(1));

    const source = screen.getByTitle("Ouvrir la source");
    fireEvent.click(source);
    await waitFor(() => expect(mocks.openMediaSource).toHaveBeenCalledTimes(1));
  });

  it("disables the controls while a command runs", async () => {
    let finish;
    mocks.mediaNext.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    setMedia({ playing: true, title: "Song" });
    render(<MediaPill />);

    const next = screen.getByTitle("Piste suivante");
    fireEvent.click(next);
    expect(next).toBeDisabled();

    await act(async () => finish());
    await waitFor(() => expect(next).toBeEnabled());
  });
});

describe("artwork cache", () => {
  it("keeps the last artwork when a sample omits the thumbnail", () => {
    setMedia({ playing: true, title: "Song", thumb: THUMB });
    const { container, rerender } = render(<MediaPill />);
    expect(container.querySelector(".mediapill__bg")).not.toBeNull();

    setMedia({ playing: true, title: "Song" });
    rerender(<MediaPill />);

    expect(container.querySelector(".mediapill__bg")).not.toBeNull();
  });

  it("clears the artwork when the backend sends null", () => {
    setMedia({ playing: true, title: "Song", thumb: THUMB });
    const { container, rerender } = render(<MediaPill />);

    setMedia({ playing: true, title: "Song", thumb: null });
    rerender(<MediaPill />);

    expect(container.querySelector(".mediapill__bg")).toBeNull();
  });

  it("drops the artwork when playback stops", () => {
    setMedia({ playing: true, title: "Song", thumb: THUMB });
    const { container, rerender } = render(<MediaPill />);

    setMedia({});
    rerender(<MediaPill />);

    expect(container.querySelector(".mediapill__bg")).toBeNull();
  });
});
