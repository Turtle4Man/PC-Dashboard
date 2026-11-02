import { useEffect, useState } from "react";
import { useSettings } from "../store/useSettings";

function currentTime(hourFormat) {
  const now = new Date();
  const minutes = String(now.getMinutes()).padStart(2, "0");

  if (hourFormat === "12") {
    const hours = now.getHours() % 12 || 12;
    const suffix = now.getHours() < 12 ? "AM" : "PM";
    return `${hours}:${minutes} ${suffix}`;
  }

  const hours = String(now.getHours()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

export default function Clock() {
  const { settings } = useSettings();
  const [time, setTime] = useState(() => currentTime(settings.hourFormat));

  useEffect(() => {
    setTime(currentTime(settings.hourFormat));
    const timer = setInterval(
      () => setTime(currentTime(settings.hourFormat)),
      1000,
    );
    return () => clearInterval(timer);
  }, [settings.hourFormat]);

  return <div className="clock">{time}</div>;
}
