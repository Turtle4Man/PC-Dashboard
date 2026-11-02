import { useEffect, useState } from "react";
import { DEFAULT_BUTTONS, normalizeButtons } from "../lib/buttons";
import { KEYBOARD_LOCK_VALUES } from "../lib/keyboard";
import { setThemePreference } from "../lib/theme";

const STORAGE_KEY = "pc-dashboard.settings";

export const DEFAULT_SETTINGS = {
  theme: "system",
  screenshotScope: "all",
  hourFormat: "24",
  keyboardLockSeconds: 60,
  showStats: true,
  showClock: true,
  showTiles: true,
  showDevices: true,
  buttons: DEFAULT_BUTTONS,
};

function pick(value, allowed, fallback) {
  return allowed.includes(value) ? value : fallback;
}

function flag(value, fallback) {
  return typeof value === "boolean" ? value : fallback;
}

function normalize(value = {}) {
  return {
    theme: pick(value.theme, ["light", "dark", "system"], DEFAULT_SETTINGS.theme),
    screenshotScope: pick(
      value.screenshotScope,
      ["all", "primary"],
      DEFAULT_SETTINGS.screenshotScope,
    ),
    hourFormat: pick(value.hourFormat, ["12", "24"], DEFAULT_SETTINGS.hourFormat),
    keyboardLockSeconds: pick(
      value.keyboardLockSeconds,
      KEYBOARD_LOCK_VALUES,
      DEFAULT_SETTINGS.keyboardLockSeconds,
    ),
    showStats: flag(value.showStats, DEFAULT_SETTINGS.showStats),
    showClock: flag(value.showClock, DEFAULT_SETTINGS.showClock),
    showTiles: flag(value.showTiles, DEFAULT_SETTINGS.showTiles),
    showDevices: flag(value.showDevices, DEFAULT_SETTINGS.showDevices),
    buttons: normalizeButtons(value.buttons),
  };
}

function load() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? normalize(JSON.parse(raw)) : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

let settings = load();
const subscribers = new Set();

function emit() {
  for (const fn of subscribers) fn(settings);
}

export function getSettings() {
  return settings;
}

// Returns false when the settings could not be persisted (for example when a
// stored icon no longer fits in the localStorage quota). The in-memory state is
// still updated so the app keeps working for the current session.
export function updateSettings(patch) {
  settings = { ...settings, ...patch };

  let persisted = true;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    persisted = false;
  }

  if ("theme" in patch) setThemePreference(settings.theme);
  emit();
  return persisted;
}

export function useSettings() {
  const [snapshot, setSnapshot] = useState(settings);

  useEffect(() => {
    subscribers.add(setSnapshot);
    return () => {
      subscribers.delete(setSnapshot);
    };
  }, []);

  return { settings: snapshot, updateSettings };
}
