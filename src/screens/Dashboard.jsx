import { useState } from "react";
import BottomNav from "../components/BottomNav";
import HardwareScreen from "./HardwareScreen";
import HomeScreen from "./HomeScreen";
import SettingsScreen from "./SettingsScreen";

export default function Dashboard() {
  const [tab, setTab] = useState("home");

  return (
    <div className="dashboard">
      <main className="dashboard__main">
        {tab === "home" && <HomeScreen />}
        {tab === "settings" && <SettingsScreen />}
        {tab === "hardware" && <HardwareScreen />}
      </main>
      <BottomNav tab={tab} onSelect={setTab} />
    </div>
  );
}
