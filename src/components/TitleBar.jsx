// src/components/TitleBar.jsx
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import closeIcon from "../assets/icons/close.svg";
import minimizeIcon from "../assets/icons/minimize.svg";
import settingsIcon from "../assets/icons/settings.svg";

function TitleBar({ onSettingsToggle, hasUpdate }) {
  const appWindow = getCurrentWindow();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion).catch(console.error);
  }, []);

  const handleMinimize = () => {
    appWindow.minimize();
  };

  const handleClose = () => {
    appWindow.close();
  };

  return (
    <div className="title-bar" data-tauri-drag-region>
      <h1 className="title-bar-title" data-tauri-drag-region>
        Image optimizer{" "}
        {version && <span className="title-bar-version">v{version}</span>}
      </h1>
      <div className="window-controls">
        <button
          type="button"
          onClick={onSettingsToggle}
          className="window-control-button window-control-settings"
          title="Settings"
        >
          <img src={settingsIcon} alt="Settings" />
          {hasUpdate && <span className="window-control-badge" />}
        </button>
        <button
          type="button"
          onClick={handleMinimize}
          className="window-control-button"
          title="Minimize"
        >
          <img src={minimizeIcon} alt="Minimize" />
        </button>
        <button
          type="button"
          onClick={handleClose}
          className="window-control-button window-control-close"
          title="Close"
        >
          <img src={closeIcon} alt="Close" />
        </button>
      </div>
    </div>
  );
}

export default TitleBar;
