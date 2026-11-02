import ActionGrid from "../components/ActionGrid";
import DevicesList from "../components/DevicesList";
import Clock from "../components/Clock";
import MediaPill from "../components/MediaPill";
import StatsList from "../components/StatsList";
import { useSettings } from "../store/useSettings";

export default function HomeScreen() {
  const { settings } = useSettings();

  return (
    <div className="home">
      {/* Empty placeholders keep the center column aligned when a side
          panel is hidden. */}
      {settings.showStats ? <StatsList /> : <div className="stats" aria-hidden="true" />}

      <div className="center">
        {settings.showClock && <Clock />}
        <MediaPill />
        {settings.showTiles && <ActionGrid />}
      </div>

      {settings.showDevices ? (
        <DevicesList />
      ) : (
        <div className="devices" aria-hidden="true" />
      )}
    </div>
  );
}
