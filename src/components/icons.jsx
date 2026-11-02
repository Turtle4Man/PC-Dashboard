import micOnRaw from "../assets/icons/Mic_on.svg?raw";
import micOffRaw from "../assets/icons/Mic_off.svg?raw";
import screenshotRaw from "../assets/icons/Screenshot.svg?raw";
import pauseRaw from "../assets/icons/Pause.svg?raw";
import playRaw from "../assets/icons/Play.svg?raw";
import previousRaw from "../assets/icons/Previous.svg?raw";
import nextRaw from "../assets/icons/Next.svg?raw";
import devicesRaw from "../assets/icons/Devices.svg?raw";
import discordRaw from "../assets/icons/Discord.svg?raw";
import headphonesRaw from "../assets/icons/Headphones.svg?raw";
import mouseRaw from "../assets/icons/Mouse.svg?raw";
import keyboardRaw from "../assets/icons/Keyboard.svg?raw";
import keyboardOffRaw from "../assets/icons/Keyboard_off.svg?raw";
import buttonsRaw from "../assets/icons/Buttons.svg?raw";
import settingsRaw from "../assets/icons/Settings.svg?raw";
import configRaw from "../assets/icons/Config.svg?raw";
import webRaw from "../assets/icons/Web.svg?raw";
import gamepadRaw from "../assets/icons/Gamepad.svg?raw";
import batteryNaRaw from "../assets/icons/Battery_na.svg?raw";
import batteryRaw from "../assets/icons/Battery.svg?raw";

export function MicOnIcon({ size }) {
  return <RawIcon raw={micOnRaw} size={size} alt="Micro actif" />;
}

export function MicOffIcon({ size }) {
  return <RawIcon raw={micOffRaw} size={size} alt="Micro coupé" />;
}

export function ScreenshotIcon({ size }) {
  return <RawIcon raw={screenshotRaw} size={size} alt="Capture d'écran" />;
}

export function PauseIcon({ size }) {
  return <RawIcon raw={pauseRaw} size={size} alt="Pause" />;
}

export function PlayIcon({ size }) {
  return <RawIcon raw={playRaw} size={size} alt="Lecture" />;
}

export function PreviousIcon({ size }) {
  return <RawIcon raw={previousRaw} size={size} alt="Précédent" />;
}

export function NextIcon({ size }) {
  return <RawIcon raw={nextRaw} size={size} alt="Suivant" />;
}

export function DevicesIcon({ size }) {
  return <RawIcon raw={devicesRaw} size={size} alt="Appareils" />;
}

export function DiscordIcon({ size }) {
  return <RawIcon raw={discordRaw} size={size} alt="Discord" className="brand" />;
}

export function HeadphonesIcon({ size }) {
  return <RawIcon raw={headphonesRaw} size={size} alt="Casque" />;
}

export function MouseIcon({ size }) {
  return <RawIcon raw={mouseRaw} size={size} alt="Souris" />;
}

export function KeyboardIcon({ size }) {
  return <RawIcon raw={keyboardRaw} size={size} alt="Clavier" />;
}

export function KeyboardOffIcon({ size }) {
  return <RawIcon raw={keyboardOffRaw} size={size} alt="Clavier verrouillé" />;
}

export function ButtonsIcon({ size }) {
  return <RawIcon raw={buttonsRaw} size={size} alt="Accueil" />;
}

export function SettingsIcon({ size }) {
  return <RawIcon raw={settingsRaw} size={size} alt="Réglages" />;
}

export function ConfigIcon({ size }) {
  return <RawIcon raw={configRaw} size={size} alt="Outils" />;
}

export function WebIcon({ size }) {
  return <RawIcon raw={webRaw} size={size} alt="Web" />;
}

export function GamepadIcon({ size }) {
  return <RawIcon raw={gamepadRaw} size={size} alt="Manette" />;
}

export function BatteryNaIcon({ size }) {
  return <RawIcon raw={batteryNaRaw} size={size} alt="Niveau de batterie inconnu" />;
}

function RawIcon({ raw, size = 22, alt = "", className = "" }) {
  const svg = raw
    .replace(/fill="#e3e3e3"/gi, 'fill="currentColor"')
    .replace(/fill="#d9d9d9"/gi, 'fill="currentColor"')
    .replace(/fill="white"/gi, 'fill="currentColor"');
  return (
    <span
      className={`rawicon ${className}`.trim()}
      role="img"
      aria-label={alt}
      style={{ width: size, height: size }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

export function BatteryIcon({ size }) {
  return <RawIcon raw={batteryRaw} size={size} alt="Batterie" />;
}

// Icons are rendered through RawIcon so their fills follow currentColor.
