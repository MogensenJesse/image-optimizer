import { platform } from "@tauri-apps/plugin-os";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./assets/styles/main.scss";
import { I18nProvider } from "./i18n";

// Linux has no window vibrancy, so the CSS needs an opaque background there
if (platform() === "linux") {
  document.body.classList.add("platform-linux");
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <I18nProvider>
      <App />
    </I18nProvider>
  </React.StrictMode>,
);
