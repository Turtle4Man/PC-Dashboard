// Durations offered for the keyboard lock. The backend clamps anything outside
// its own safety bounds, so these values only shape the interface.
export const KEYBOARD_LOCK_OPTIONS = [
  { value: 30, label: "30 s" },
  { value: 60, label: "1 min" },
  { value: 300, label: "5 min" },
  { value: 900, label: "15 min" },
  { value: 0, label: "Illimitée" },
];

export const KEYBOARD_LOCK_VALUES = KEYBOARD_LOCK_OPTIONS.map(
  (option) => option.value,
);

export function keyboardLockLabel(seconds) {
  const option = KEYBOARD_LOCK_OPTIONS.find((entry) => entry.value === seconds);
  return option ? option.label : `${seconds} s`;
}

// `0` has no automatic release, so the tile cannot promise a duration.
export function keyboardLockTitle(seconds) {
  return seconds === 0
    ? "Verrouiller le clavier sans limite de durée"
    : `Verrouiller le clavier pendant ${keyboardLockLabel(seconds)}`;
}
