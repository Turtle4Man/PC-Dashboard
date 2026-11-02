import { useEffect, useState } from "react";
import ButtonIcon from "../components/ButtonIcon";
import { BUTTON_TYPES, MAX_ICON_LENGTH, updateButton } from "../lib/buttons";
import { fileToIcon } from "../lib/image";
import { KEYBOARD_LOCK_OPTIONS } from "../lib/keyboard";
import { getAutostart, getDiscordStatus, setAutostart } from "../lib/ipc";
import { getSettings, useSettings } from "../store/useSettings";

const THEME_OPTIONS = [
  { value: "light", label: "Clair" },
  { value: "dark", label: "Sombre" },
  { value: "system", label: "Système" },
];

const HOUR_OPTIONS = [
  { value: "24", label: "24 h" },
  { value: "12", label: "12 h" },
];

const SCREENSHOT_OPTIONS = [
  { value: "all", label: "Tous les écrans" },
  { value: "primary", label: "Écran principal" },
];

function Segmented({ label, hint, value, options, onChange }) {
  return (
    <div className="setting">
      <span className="setting__text">
        <span className="setting__label">{label}</span>
        {hint && <span className="setting__hint">{hint}</span>}
      </span>
      <div className="segmented" role="group" aria-label={label}>
        {options.map((option) => (
          <button
            key={option.value}
            type="button"
            className={option.value === value ? "is-active" : ""}
            aria-pressed={option.value === value}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

function ButtonSetting({ index, button, discord, onChange, onIcon }) {
  return (
    <div className="button-setting">
      {button.type === "empty" ? (
        <span
          className="button-setting__preview button-setting__preview--empty"
          aria-hidden="true"
        />
      ) : (
        <label className="button-setting__preview" title="Choisir une image…">
          <ButtonIcon button={button} size={26} />
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/gif"
            aria-label={`Image du bouton ${index + 1}`}
            hidden
            onChange={onIcon}
          />
        </label>
      )}

      <div className="button-setting__body">
        <span className="button-setting__title">Bouton {index + 1}</span>
        <div className="button-setting__controls">
          <select
            value={button.type}
            aria-label={`Action du bouton ${index + 1}`}
            onChange={(event) => onChange({ type: event.target.value, value: "" })}
          >
            {BUTTON_TYPES.map((type) => (
              <option key={type.value} value={type.value}>
                {type.label}
              </option>
            ))}
          </select>

          {button.type === "web" && (
            <input
              type="url"
              value={button.value}
              placeholder="https://exemple.com"
              aria-label={`Adresse web du bouton ${index + 1}`}
              onChange={(event) => onChange({ value: event.target.value })}
            />
          )}

          {button.type === "hotkey" && (
            <input
              type="text"
              value={button.value}
              placeholder="Ctrl+Shift+M"
              aria-label={`Raccourci clavier du bouton ${index + 1}`}
              onChange={(event) => onChange({ value: event.target.value })}
            />
          )}

          {button.icon && (
            <button
              type="button"
              className="button-setting__reset"
              onClick={() => onChange({ icon: null })}
            >
              Icône par défaut
            </button>
          )}
        </div>

        {/* Discord only matters for the microphone button, so the note sits
            with that button instead of in a section of its own. */}
        {button.type === "mic" && (
          <p
            className={`button-setting__status ${
              discord === undefined
                ? "settings__status--checking"
                : discord
                  ? "settings__status--ok"
                  : "settings__status--warn"
            }`}
          >
            {discord === undefined
              ? "Vérification de Discord…"
              : discord
                ? "Discord est lancé : le bouton coupe aussi le micro Discord."
                : "Discord n'est pas lancé : le bouton coupe seulement le micro Windows."}
          </p>
        )}
      </div>
    </div>
  );
}

function Toggle({ label, hint, checked, disabled, onChange }) {
  return (
    <label className={`setting ${disabled ? "setting--disabled" : ""}`}>
      <span className="setting__text">
        <span className="setting__label">{label}</span>
        {hint && <span className="setting__hint">{hint}</span>}
      </span>
      <input
        type="checkbox"
        className="switch"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

export default function SettingsScreen() {
  const { settings, updateSettings } = useSettings();
  const [autostart, setAutostartState] = useState(null);
  const [autostartPending, setAutostartPending] = useState(false);
  const [autostartError, setAutostartError] = useState(false);
  const [buttonError, setButtonError] = useState(null);
  // `undefined` while Discord is being checked, then whether it was found.
  const [discord, setDiscord] = useState(undefined);

  // The OS is the source of truth for autostart: the stored preference only
  // records what the user asked for, but a disabled registry entry wins.
  useEffect(() => {
    let cancelled = false;
    getAutostart()
      .then((enabled) => {
        if (!cancelled) setAutostartState(enabled);
      })
      .catch(() => {
        if (!cancelled) setAutostartError(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // The status is only informative here: the backend checks Discord again when
  // the microphone button is used, so launching Discord later still works.
  useEffect(() => {
    let cancelled = false;
    getDiscordStatus()
      .then((available) => {
        if (!cancelled) setDiscord(available === true);
      })
      .catch(() => {
        if (!cancelled) setDiscord(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function onAutostart(enabled) {
    setAutostartPending(true);
    setAutostartError(false);
    try {
      const actual = await setAutostart(enabled);
      setAutostartState(actual);
    } catch {
      setAutostartError(true);
    } finally {
      setAutostartPending(false);
    }
  }

  const autostartHint = autostartError
    ? "Impossible de modifier ce réglage pour le moment."
    : "Ouvre PC Dashboard à l'ouverture de session Windows.";

  function changeButton(index, patch) {
    updateSettings({ buttons: updateButton(settings.buttons, index, patch) });
  }

  async function onIcon(index, event) {
    const file = event.target.files?.[0];
    // Clear the input so picking the same file again still fires a change.
    event.target.value = "";
    if (!file) return;

    setButtonError(null);
    try {
      const icon = await fileToIcon(file);
      if (icon.length > MAX_ICON_LENGTH) {
        throw new Error(
          "Cette image est trop détaillée pour tenir dans un bouton. Essayez une image plus simple.",
        );
      }

      const persisted = updateSettings({
        buttons: updateButton(getSettings().buttons, index, { icon }),
      });
      if (!persisted) {
        throw new Error(
          "L'icône n'a pas pu être enregistrée : le stockage local est plein. Réinitialisez une autre icône puis réessayez.",
        );
      }
    } catch (error) {
      setButtonError(
        error instanceof Error
          ? error.message
          : "Impossible de lire cette image.",
      );
    }
  }

  return (
    <div className="settings">
      <h1>Réglages</h1>

      <section>
        <h2>Apparence</h2>
        <div className="settings__list">
          <Segmented
            label="Thème"
            value={settings.theme}
            options={THEME_OPTIONS}
            onChange={(theme) => updateSettings({ theme })}
          />
          <Segmented
            label="Format de l'heure"
            value={settings.hourFormat}
            options={HOUR_OPTIONS}
            onChange={(hourFormat) => updateSettings({ hourFormat })}
          />
        </div>
      </section>

      <section>
        <h2>Capture d'écran</h2>
        <div className="settings__list">
          <Segmented
            label="Portée"
            hint="Écrans capturés par le bouton de capture."
            value={settings.screenshotScope}
            options={SCREENSHOT_OPTIONS}
            onChange={(screenshotScope) => updateSettings({ screenshotScope })}
          />
        </div>
      </section>

      <section>
        <h2>Clavier</h2>
        <div className="settings__list">
          <Segmented
            label="Durée du verrouillage"
            hint={
              settings.keyboardLockSeconds === 0
                ? "Le clavier reste verrouillé jusqu'au clic sur le bouton. La souris reste utilisable pour appuyer dessus."
                : "Le clavier se déverrouille tout seul après cette durée. La souris reste utilisable pour appuyer sur le bouton."
            }
            value={settings.keyboardLockSeconds}
            options={KEYBOARD_LOCK_OPTIONS}
            onChange={(keyboardLockSeconds) =>
              updateSettings({ keyboardLockSeconds })
            }
          />
        </div>
      </section>

      <section>
        <h2>Démarrage</h2>
        <div className="settings__list">
          <Toggle
            label="Lancer au démarrage de Windows"
            hint={autostartHint}
            checked={autostart === true}
            disabled={autostartPending || autostart === null}
            onChange={onAutostart}
          />
        </div>
      </section>

      <section>
        <h2>Éléments affichés</h2>
        <div className="settings__list">
          <Toggle
            label="Statistiques"
            hint="Utilisation du processeur, du GPU, de la mémoire, du réseau et des disques."
            checked={settings.showStats}
            onChange={(showStats) => updateSettings({ showStats })}
          />
          <Toggle
            label="Horloge"
            hint="Heure courante affichée sur l'accueil."
            checked={settings.showClock}
            onChange={(showClock) => updateSettings({ showClock })}
          />
          <Toggle
            label="Boutons d'action"
            hint="Micro, verrouillage du clavier, capture d'écran, appareils et raccourcis."
            checked={settings.showTiles}
            onChange={(showTiles) => updateSettings({ showTiles })}
          />
          <Toggle
            label="Périphériques"
            hint="Appareils connectés et niveau de batterie."
            checked={settings.showDevices}
            onChange={(showDevices) => updateSettings({ showDevices })}
          />
        </div>
      </section>

      <section>
        <h2>Boutons</h2>
        <p className="settings__note">
          Choisissez l'action de chaque bouton. Une image locale (PNG, JPEG, WebP
          ou GIF) remplace l'icône par défaut. Les boutons sont visibles sur
          l'accueil quand « Boutons d'action » est activé.
        </p>
        {buttonError && <p className="settings__error">{buttonError}</p>}
        <div className="buttons-editor">
          {settings.buttons.map((button, index) => (
            <ButtonSetting
              key={index}
              index={index}
              button={button}
              discord={discord}
              onChange={(patch) => changeButton(index, patch)}
              onIcon={(event) => onIcon(index, event)}
            />
          ))}
        </div>
      </section>
    </div>
  );
}
