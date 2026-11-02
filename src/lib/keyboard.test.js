import { describe, expect, it } from "vitest";
import {
  KEYBOARD_LOCK_OPTIONS,
  KEYBOARD_LOCK_VALUES,
  keyboardLockLabel,
  keyboardLockTitle,
} from "./keyboard";

describe("keyboard lock durations", () => {
  it("offers the five expected choices, unlimited last", () => {
    expect(KEYBOARD_LOCK_OPTIONS.map((option) => option.value)).toEqual([
      30, 60, 300, 900, 0,
    ]);
    expect(KEYBOARD_LOCK_OPTIONS.at(-1).label).toBe("Illimitée");
    expect(KEYBOARD_LOCK_VALUES).toEqual([30, 60, 300, 900, 0]);
  });

  it("formats a known duration with its label", () => {
    expect(keyboardLockLabel(60)).toBe("1 min");
    expect(keyboardLockLabel(900)).toBe("15 min");
    expect(keyboardLockLabel(0)).toBe("Illimitée");
  });

  it("falls back to seconds for an unlisted value", () => {
    expect(keyboardLockLabel(45)).toBe("45 s");
  });

  it("promises a duration only when the lock expires by itself", () => {
    expect(keyboardLockTitle(60)).toBe("Verrouiller le clavier pendant 1 min");
    expect(keyboardLockTitle(30)).toBe("Verrouiller le clavier pendant 30 s");
    expect(keyboardLockTitle(0)).toBe(
      "Verrouiller le clavier sans limite de durée",
    );
  });
});
