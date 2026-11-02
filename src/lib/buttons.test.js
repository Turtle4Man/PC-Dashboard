import { describe, expect, it } from "vitest";
import {
  BUTTON_COUNT,
  BUTTON_TYPES,
  DEFAULT_BUTTONS,
  MAX_ICON_LENGTH,
  buttonLabel,
  needsValue,
  normalizeButtons,
  updateButton,
} from "./buttons";

const ICON = "data:image/png;base64,AAAA";

describe("normalizeButtons", () => {
  it("always returns exactly nine buttons", () => {
    expect(normalizeButtons(undefined)).toHaveLength(BUTTON_COUNT);
    expect(normalizeButtons(null)).toHaveLength(BUTTON_COUNT);
    expect(normalizeButtons([])).toHaveLength(BUTTON_COUNT);
    expect(normalizeButtons([{ type: "mic" }])).toHaveLength(BUTTON_COUNT);
    expect(
      normalizeButtons(Array.from({ length: 12 }, () => ({ type: "mic" }))),
    ).toHaveLength(BUTTON_COUNT);
  });

  it("replaces missing or unknown types with an empty button", () => {
    const buttons = normalizeButtons([{ type: "soundboard" }, {}, 42]);

    expect(buttons[0]).toEqual({ type: "empty", value: "", icon: null });
    expect(buttons[1].type).toBe("empty");
    expect(buttons[2].type).toBe("empty");
  });

  it("keeps the value of web and hotkey buttons only", () => {
    const buttons = normalizeButtons([
      { type: "web", value: "https://exemple.com" },
      { type: "hotkey", value: "Ctrl+Shift+M" },
      { type: "mic", value: "ignored" },
    ]);

    expect(buttons[0].value).toBe("https://exemple.com");
    expect(buttons[1].value).toBe("Ctrl+Shift+M");
    expect(buttons[2].value).toBe("");
  });

  it("ignores values that are not strings", () => {
    const buttons = normalizeButtons([{ type: "web", value: 42 }]);

    expect(buttons[0].value).toBe("");
  });

  it("keeps data image icons up to the storage limit", () => {
    const buttons = normalizeButtons([{ type: "mic", icon: ICON }]);

    expect(buttons[0].icon).toBe(ICON);
  });

  it("drops icons that are not data images or are too large", () => {
    const oversized = `data:image/png;base64,${"A".repeat(MAX_ICON_LENGTH)}`;
    const buttons = normalizeButtons([
      { type: "mic", icon: "https://exemple.com/icon.png" },
      { type: "mic", icon: oversized },
    ]);

    expect(buttons[0].icon).toBeNull();
    expect(buttons[1].icon).toBeNull();
  });

  it("accepts an icon exactly at the size limit", () => {
    const prefix = "data:image/png;base64,";
    const icon = prefix + "A".repeat(MAX_ICON_LENGTH - prefix.length);

    expect(normalizeButtons([{ type: "mic", icon }])[0].icon).toBe(icon);
  });

  it("does not share objects between normalized buttons", () => {
    const buttons = normalizeButtons([]);

    expect(buttons[0]).not.toBe(buttons[1]);
    buttons[0].type = "mic";
    expect(buttons[1].type).toBe("empty");
  });
});

describe("updateButton", () => {
  const buttons = normalizeButtons(DEFAULT_BUTTONS);

  it("returns a new array with only the target button patched", () => {
    const updated = updateButton(buttons, 2, { type: "hotkey", value: "F13" });

    expect(updated).not.toBe(buttons);
    expect(updated[2]).toMatchObject({ type: "hotkey", value: "F13" });
    expect(updated[0]).toBe(buttons[0]);
    expect(updated[3]).toBe(buttons[3]);
    expect(buttons[2].type).toBe("empty");
  });

  it("leaves the list untouched for an unknown index", () => {
    const updated = updateButton(buttons, 99, { type: "mic" });

    expect(updated).toEqual(buttons);
    expect(updated).not.toBe(buttons);
  });
});

describe("button metadata", () => {
  it("only asks a value for web shortcuts and hotkeys", () => {
    expect(needsValue("web")).toBe(true);
    expect(needsValue("hotkey")).toBe(true);
    expect(needsValue("mic")).toBe(false);
    expect(needsValue("screenshot")).toBe(false);
  });

  it("uses the expected French labels", () => {
    expect(buttonLabel("mic")).toBe("Couper le micro");
    expect(buttonLabel("keyboard")).toBe("Verrouiller le clavier");
    expect(buttonLabel("unknown")).toBe("Vide");
  });

  it("does not offer the abandoned soundboard action", () => {
    expect(BUTTON_TYPES.map((type) => type.value)).not.toContain("soundboard");
  });

  it("starts with microphone, screenshot and devices configured", () => {
    expect(DEFAULT_BUTTONS).toHaveLength(BUTTON_COUNT);
    expect(DEFAULT_BUTTONS.map((button) => button.type).slice(0, 4)).toEqual([
      "mic",
      "screenshot",
      "empty",
      "devices",
    ]);
  });
});
