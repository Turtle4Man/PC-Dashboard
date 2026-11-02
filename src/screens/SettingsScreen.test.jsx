import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { normalizeButtons } from "../lib/buttons";

const mocks = vi.hoisted(() => ({
  getAutostart: vi.fn(),
  getDiscordStatus: vi.fn(),
  setAutostart: vi.fn(),
}));

vi.mock("../lib/ipc", () => ({
  getAutostart: mocks.getAutostart,
  getDiscordStatus: mocks.getDiscordStatus,
  setAutostart: mocks.setAutostart,
}));

import SettingsScreen from "./SettingsScreen";
import { getSettings, updateSettings, DEFAULT_SETTINGS } from "../store/useSettings";

function resetSettings(patch = {}) {
  updateSettings({
    ...DEFAULT_SETTINGS,
    buttons: normalizeButtons(DEFAULT_SETTINGS.buttons),
    ...patch,
  });
}

async function renderSettings() {
  render(<SettingsScreen />);
  // The startup toggle is disabled until the backend answered, and the Discord
  // note switches from "Vérification…" once its status is known.
  await waitFor(() => {
    expect(
      screen.getByRole("checkbox", { name: /Lancer au démarrage de Windows/ }),
    ).toBeEnabled();
    expect(
      document.querySelector(".button-setting__status")?.textContent ?? "",
    ).not.toMatch(/Vérification/);
  });
}

beforeEach(() => {
  resetSettings();
  mocks.getAutostart.mockResolvedValue(true);
  mocks.getDiscordStatus.mockResolvedValue(true);
  mocks.setAutostart.mockResolvedValue(false);
});

describe("keyboard settings", () => {
  it("offers every duration, including the unlimited lock", async () => {
    await renderSettings();
    const group = screen.getByRole("group", { name: "Durée du verrouillage" });

    expect(
      within(group)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["30 s", "1 min", "5 min", "15 min", "Illimitée"]);
    expect(within(group).getByRole("button", { name: "1 min" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("stores an unlimited lock and explains it", async () => {
    await renderSettings();
    const group = screen.getByRole("group", { name: "Durée du verrouillage" });

    fireEvent.click(within(group).getByRole("button", { name: "Illimitée" }));

    expect(getSettings().keyboardLockSeconds).toBe(0);
    expect(screen.getByText(/reste verrouillé jusqu'au clic sur le bouton/)).toBeInTheDocument();
    expect(within(group).getByRole("button", { name: "Illimitée" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("stores a limited duration", async () => {
    await renderSettings();
    const group = screen.getByRole("group", { name: "Durée du verrouillage" });

    fireEvent.click(within(group).getByRole("button", { name: "15 min" }));

    expect(getSettings().keyboardLockSeconds).toBe(900);
    expect(screen.getByText(/se déverrouille tout seul/)).toBeInTheDocument();
  });
});

describe("Discord note", () => {
  it("sits in the microphone button when Discord is running", async () => {
    await renderSettings();

    expect(
      screen.getByText(
        "Discord est lancé : le bouton coupe aussi le micro Discord.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Discord" }),
    ).not.toBeInTheDocument();
  });

  it("says Discord is optional when it is closed", async () => {
    mocks.getDiscordStatus.mockResolvedValue(false);
    await renderSettings();

    expect(
      screen.getByText(
        "Discord n'est pas lancé : le bouton coupe seulement le micro Windows.",
      ),
    ).toBeInTheDocument();
  });

  it("disappears with the microphone button", async () => {
    await renderSettings();
    expect(screen.getByText(/Discord est lancé/)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Action du bouton 1"), {
      target: { value: "empty" },
    });

    expect(screen.queryByText(/Discord est lancé/)).not.toBeInTheDocument();
    expect(screen.getByText("Bouton 1")).toBeInTheDocument();
  });
});

describe("autostart setting", () => {
  it("reflects the OS state and writes the change", async () => {
    await renderSettings();
    const toggle = screen.getByRole("checkbox", {
      name: /Lancer au démarrage de Windows/,
    });
    expect(toggle).toBeChecked();

    fireEvent.click(toggle);

    expect(mocks.setAutostart).toHaveBeenCalledWith(false);
    await waitFor(() => expect(toggle).not.toBeChecked());
  });

  it("stays checked and explains a backend failure", async () => {
    mocks.setAutostart.mockRejectedValue(new Error("registry denied"));
    await renderSettings();
    const toggle = screen.getByRole("checkbox", {
      name: /Lancer au démarrage de Windows/,
    });

    fireEvent.click(toggle);

    await screen.findByText("Impossible de modifier ce réglage pour le moment.");
    expect(toggle).toBeChecked();
  });
});

describe("other settings", () => {
  it("stores the screenshot scope", async () => {
    await renderSettings();
    const group = screen.getByRole("group", { name: "Portée" });

    fireEvent.click(within(group).getByRole("button", { name: "Écran principal" }));

    expect(getSettings().screenshotScope).toBe("primary");
  });

  it("hides a section from the home screen", async () => {
    await renderSettings();
    const toggle = screen.getByRole("checkbox", { name: /Horloge/ });
    expect(toggle).toBeChecked();

    fireEvent.click(toggle);

    expect(getSettings().showClock).toBe(false);
    expect(toggle).not.toBeChecked();
  });

  it("applies the chosen theme", async () => {
    await renderSettings();
    const group = screen.getByRole("group", { name: "Thème" });

    fireEvent.click(within(group).getByRole("button", { name: "Sombre" }));

    expect(getSettings().theme).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});

describe("button editor", () => {
  it("reveals the hotkey field and stores the shortcut", async () => {
    await renderSettings();

    fireEvent.change(screen.getByLabelText("Action du bouton 3"), {
      target: { value: "hotkey" },
    });
    const input = screen.getByLabelText("Raccourci clavier du bouton 3");
    expect(input).toHaveAttribute("placeholder", "Ctrl+Shift+M");

    fireEvent.change(input, { target: { value: "F13" } });

    expect(getSettings().buttons[2]).toMatchObject({
      type: "hotkey",
      value: "F13",
    });
  });

  it("clears the value when the action changes", async () => {
    await renderSettings();
    fireEvent.change(screen.getByLabelText("Action du bouton 1"), {
      target: { value: "web" },
    });
    fireEvent.change(screen.getByLabelText("Adresse web du bouton 1"), {
      target: { value: "exemple.com" },
    });
    expect(getSettings().buttons[0].value).toBe("exemple.com");

    fireEvent.change(screen.getByLabelText("Action du bouton 1"), {
      target: { value: "hotkey" },
    });

    expect(getSettings().buttons[0]).toMatchObject({ type: "hotkey", value: "" });
  });
});
