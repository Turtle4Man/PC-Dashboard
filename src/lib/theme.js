import { getCurrentWindow } from "@tauri-apps/api/window";

let preference = "system";
let systemTheme = null;

function applyTheme(theme) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
}

function apply() {
  applyTheme(preference === "system" ? systemTheme : preference);
}

export function setThemePreference(next) {
  preference = next === "light" || next === "dark" ? next : "system";
  apply();
}

function mediaTheme() {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function initTheme() {
  const media = window.matchMedia("(prefers-color-scheme: dark)");

  const setSystem = (theme) => {
    systemTheme = theme;
    if (preference === "system") apply();
  };

  setSystem(mediaTheme());

  let win;
  try {
    win = getCurrentWindow();
  } catch {
    const onChange = () => setSystem(mediaTheme());
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }

  const refresh = () => {
    win
      .theme()
      .then((theme) => setSystem(theme ?? mediaTheme()))
      .catch(() => setSystem(mediaTheme()));
  };

  refresh();
  const onFocus = () => refresh();
  window.addEventListener("focus", onFocus);

  let unlisten;
  win
    .onThemeChanged(({ payload }) => setSystem(payload))
    .then((fn) => {
      unlisten = fn;
    })
    .catch(() => {});

  return () => {
    window.removeEventListener("focus", onFocus);
    unlisten?.();
  };
}
