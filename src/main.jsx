import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initTheme, setThemePreference } from "./lib/theme";
import { getSettings } from "./store/useSettings";
import "./styles.css";

setThemePreference(getSettings().theme);
initTheme();

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
