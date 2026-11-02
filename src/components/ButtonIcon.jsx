import {
  DevicesIcon,
  KeyboardIcon,
  KeyboardOffIcon,
  MicOffIcon,
  MicOnIcon,
  ScreenshotIcon,
  WebIcon,
} from "./icons";

const DEFAULT_ICONS = {
  screenshot: ScreenshotIcon,
  devices: DevicesIcon,
  web: WebIcon,
  hotkey: KeyboardIcon,
};

export default function ButtonIcon({ button, size = 50, muted = false, locked = false }) {
  if (button.icon) {
    return (
      <img
        className="button-icon"
        src={button.icon}
        alt=""
        style={{ width: size, height: size }}
      />
    );
  }

  if (button.type === "mic") {
    const Icon = muted ? MicOffIcon : MicOnIcon;
    return <Icon size={size} />;
  }

  if (button.type === "keyboard") {
    const Icon = locked ? KeyboardOffIcon : KeyboardIcon;
    return <Icon size={size} />;
  }

  const Icon = DEFAULT_ICONS[button.type];
  return Icon ? <Icon size={size} /> : null;
}
