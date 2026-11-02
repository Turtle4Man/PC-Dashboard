export const BUTTON_COUNT = 9;

// Keep icons small enough to stay well inside the localStorage quota once
// several of them are stored.
export const MAX_ICON_LENGTH = 200_000;

export const BUTTON_TYPES = [
  { value: "empty", label: "Vide" },
  { value: "mic", label: "Couper le micro" },
  { value: "keyboard", label: "Verrouiller le clavier" },
  { value: "screenshot", label: "Capture d'écran" },
  { value: "devices", label: "Menu Bluetooth" },
  { value: "web", label: "Raccourci web" },
  { value: "hotkey", label: "Raccourci clavier" },
];

const TYPE_VALUES = BUTTON_TYPES.map((type) => type.value);
const VALUE_TYPES = new Set(["web", "hotkey"]);

export function needsValue(type) {
  return VALUE_TYPES.has(type);
}

export function buttonLabel(type) {
  return BUTTON_TYPES.find((entry) => entry.value === type)?.label ?? "Vide";
}

function emptyButton() {
  return { type: "empty", value: "", icon: null };
}

export const DEFAULT_BUTTONS = [
  { type: "mic", value: "", icon: null },
  { type: "screenshot", value: "", icon: null },
  emptyButton(),
  { type: "devices", value: "", icon: null },
  ...Array.from({ length: BUTTON_COUNT - 4 }, emptyButton),
];

function normalizeButton(button) {
  const type = TYPE_VALUES.includes(button?.type) ? button.type : "empty";
  const icon =
    typeof button?.icon === "string" &&
    button.icon.startsWith("data:image/") &&
    button.icon.length <= MAX_ICON_LENGTH
      ? button.icon
      : null;

  return {
    type,
    value: needsValue(type) && typeof button?.value === "string" ? button.value : "",
    icon,
  };
}

export function normalizeButtons(value) {
  const buttons = Array.isArray(value) ? value : [];
  return Array.from({ length: BUTTON_COUNT }, (_, index) =>
    normalizeButton(buttons[index]),
  );
}

export function updateButton(buttons, index, patch) {
  return buttons.map((button, buttonIndex) =>
    buttonIndex === index ? { ...button, ...patch } : button,
  );
}
