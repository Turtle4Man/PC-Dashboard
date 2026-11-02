import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

export function getVitals() {
  return invoke("get_vitals");
}

export function getHardware() {
  return invoke("get_hardware");
}

export function onVitals(callback) {
  return listen("vitals", (event) => callback(event.payload));
}

export function getMicMuted() {
  return invoke("get_mic_muted");
}

export function setMicMuted(muted) {
  return invoke("set_mic_muted", { muted });
}

export function toggleMic() {
  return invoke("toggle_mic");
}

export function getDiscordStatus() {
  return invoke("get_discord_status");
}

export function getKeyboardLocked() {
  return invoke("get_keyboard_locked");
}

export function setKeyboardLocked(locked, holdSeconds) {
  return invoke("set_keyboard_locked", { locked, holdSeconds });
}

export function onKeyboardLock(callback) {
  return listen("keyboard-lock", (event) => callback(event.payload));
}

export function toggleMedia() {
  return invoke("toggle_media");
}

export function mediaNext() {
  return invoke("media_next");
}

export function mediaPrevious() {
  return invoke("media_previous");
}

export function openMediaSource() {
  return invoke("open_media_source");
}

export function takeScreenshot(allScreens = true) {
  return invoke("take_screenshot", { allScreens });
}

export function openWebShortcut(url) {
  const text = url.trim();
  if (!text) {
    return Promise.reject(new Error("Aucune adresse web configurée."));
  }
  const target = /^https?:\/\//i.test(text) ? text : `https://${text}`;
  return openUrl(target);
}

export function sendHotkey(shortcut) {
  return invoke("send_hotkey", { shortcut });
}

export function getAutostart() {
  return invoke("get_autostart");
}

export function setAutostart(enabled) {
  return invoke("set_autostart", { enabled });
}

// URI non documentée : sur Windows 25H2, ms-settings:devices ouvre la page
// « Appareils ». À re-valider à chaque version majeure de Windows.
export function openDevicesSettings() {
  return openUrl("ms-settings:devices");
}
