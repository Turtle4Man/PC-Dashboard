import { useEffect, useRef, useState } from "react";
import { useSettings } from "../store/useSettings";
import { useVitals, updateMicMuted } from "../store/useVitals";
import {
  getKeyboardLocked,
  onKeyboardLock,
  openDevicesSettings,
  openWebShortcut,
  sendHotkey,
  setKeyboardLocked,
  takeScreenshot,
  toggleMic,
} from "../lib/ipc";
import { buttonLabel } from "../lib/buttons";
import { keyboardLockTitle } from "../lib/keyboard";
import ButtonIcon from "./ButtonIcon";

const ICON_SIZE = 50;
const FEEDBACK_MS = 1500;

export default function ActionGrid() {
  const { vitals } = useVitals();
  const { settings } = useSettings();
  const muted = vitals?.mic?.muted === true;
  const [keyboardLocked, setKeyboardLockedState] = useState(false);
  const [status, setStatus] = useState(null);
  const feedbackTimer = useRef(null);

  useEffect(() => () => clearTimeout(feedbackTimer.current), []);

  // The lock lives in the backend and can lift itself, so the tile follows the
  // backend instead of remembering the last click.
  useEffect(() => {
    let disposed = false;
    let unlisten = null;

    getKeyboardLocked()
      .then((locked) => {
        if (!disposed) setKeyboardLockedState(locked === true);
      })
      .catch(() => {});

    onKeyboardLock((locked) => {
      if (!disposed) setKeyboardLockedState(locked === true);
    })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, []);

  function flash(index, state) {
    clearTimeout(feedbackTimer.current);
    setStatus({ index, state });
    feedbackTimer.current = setTimeout(() => setStatus(null), FEEDBACK_MS);
  }

  function clearStatus() {
    clearTimeout(feedbackTimer.current);
    setStatus(null);
  }

  async function run(index, button) {
    setStatus({ index, state: "pending" });
    try {
      switch (button.type) {
        case "mic": {
          // The Windows microphone flips first, before the slower Discord
          // sync, so the icon can change right away instead of waiting for
          // the next vitals poll.
          const previous = muted;
          updateMicMuted(!previous);
          let result;
          try {
            result = await toggleMic();
          } catch (error) {
            updateMicMuted(previous);
            throw error;
          }
          updateMicMuted(result.muted);
          if (!result.discordSynced) {
            // The Windows microphone is muted; only Discord could not follow.
            console.warn("Discord n'a pas pu être synchronisé.");
            flash(index, "warn");
            return;
          }
          break;
        }
        case "keyboard": {
          const locked = await setKeyboardLocked(
            !keyboardLocked,
            settings.keyboardLockSeconds,
          );
          setKeyboardLockedState(locked === true);
          break;
        }
        case "screenshot":
          await takeScreenshot(settings.screenshotScope === "all");
          break;
        case "devices":
          await openDevicesSettings();
          break;
        case "web":
          await openWebShortcut(button.value);
          break;
        case "hotkey":
          if (!button.value.trim()) {
            throw new Error("Aucun raccourci clavier configuré.");
          }
          await sendHotkey(button.value);
          break;
        default:
          return;
      }
      clearStatus();
    } catch (error) {
      console.error(error);
      flash(index, "err");
    }
  }

  function title(button) {
    switch (button.type) {
      case "mic":
        return muted ? "Réactiver le micro" : "Couper le micro";
      case "keyboard":
        return keyboardLocked
          ? "Réactiver le clavier"
          : keyboardLockTitle(settings.keyboardLockSeconds);
      case "screenshot":
        return settings.screenshotScope === "all"
          ? "Capture d'écran — tous les écrans"
          : "Capture d'écran — écran principal";
      case "devices":
        return "Réglages Bluetooth et appareils";
      case "web":
        return button.value.trim()
          ? `Ouvrir ${button.value}`
          : "Raccourci web — non configuré";
      case "hotkey":
        return button.value.trim()
          ? `Raccourci ${button.value}`
          : "Raccourci clavier — non configuré";
      default:
        return buttonLabel(button.type);
    }
  }

  return (
    <div className="tiles">
      {settings.buttons.map((button, index) => {
        if (button.type === "empty") {
          return <div className="tile tile--empty" key={index} aria-hidden="true" />;
        }

        const state = status?.index === index ? status.state : null;
        const active =
          (button.type === "mic" && muted) ||
          (button.type === "keyboard" && keyboardLocked);

        return (
          <button
            key={index}
            type="button"
            title={title(button)}
            className={`tile ${active ? "tile--active" : ""} ${
              state === "warn" ? "tile--warn" : ""
            } ${state === "err" ? "tile--err" : ""}`}
            onClick={() => run(index, button)}
            disabled={state === "pending"}
          >
            <ButtonIcon
              button={button}
              size={ICON_SIZE}
              muted={muted}
              locked={keyboardLocked}
            />
          </button>
        );
      })}
    </div>
  );
}
