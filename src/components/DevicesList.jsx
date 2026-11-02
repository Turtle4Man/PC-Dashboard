import { useVitals } from "../store/useVitals";
import {
  BatteryIcon,
  BatteryNaIcon,
  DevicesIcon,
  GamepadIcon,
  HeadphonesIcon,
  KeyboardIcon,
  MouseIcon,
} from "./icons";

function deviceIcon(name) {
  const lower = (name || "").toLowerCase();
  const iconSize = 25;

  if (/(airpod|headphone|headset|casque|earbud)/.test(lower)) {
    return <HeadphonesIcon size={iconSize} />;
  }
  if (/(souris|mouse|mice)/.test(lower)) {
    return <MouseIcon size={iconSize} />;
  }
  if (/(clavier|keyboard|\btkl\b|kbd)/.test(lower)) {
    return <KeyboardIcon size={iconSize} />;
  }
  if (/(manette|controller|xbox|dual)/.test(lower)) {
    return <GamepadIcon size={22} />;
  }
  return <DevicesIcon size={iconSize} />;
}

export default function DevicesList() {
  const { vitals } = useVitals();
  const devices = [...(vitals?.devices ?? [])].sort((a, b) =>
    a.name.localeCompare(b.name, "fr", { sensitivity: "base" }),
  );
  const iconSize = 35;

  return (
    <div className="devices">
      {!vitals && <p className="muted">…</p>}
      {vitals && devices.length === 0 && <p className="muted">—</p>}
      {devices.map((device, index) => {
        const hasLevel = typeof device.level === "number";
        const hasState = device.state === "high" || device.state === "low";
        const low = hasLevel ? device.level < 20 : device.state === "low";
        return (
          <div className="device" key={index}>
            <span className="device__icon" title={device.name}>{deviceIcon(device.name)}</span>
            <span className="device__name" title={device.name}>{device.name}</span>
            <span
              className={`device__batt level-${hasLevel || hasState ? (low ? "bad" : "ok") : "na"}`}
            >
              {hasLevel ? (
                <span className="battvalue">
                  <BatteryIcon size={iconSize} />
                  <span className="battvalue__num">
                    {device.level}
                    <small>%</small>
                  </span>
                </span>
              ) : hasState ? (
                <span
                  className="device-state"
                  title={low ? "Batterie faible" : "Batterie haute"}
                >
                  <BatteryIcon size={iconSize} />
                </span>
              ) : (
                <BatteryNaIcon size={iconSize} />
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
}
