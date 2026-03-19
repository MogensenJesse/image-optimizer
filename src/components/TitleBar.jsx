// src/components/TitleBar.jsx
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import closeIcon from "../assets/icons/close.svg";
import minimizeIcon from "../assets/icons/minimize.svg";
import settingsIcon from "../assets/icons/settings.svg";
import { useTranslation } from "../i18n";

function TitleBar({ onSettingsToggle, hasUpdate }) {
  const appWindow = getCurrentWindow();
  const [version, setVersion] = useState("");
  const { t } = useTranslation();

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
        {t("titlebar.title")}{" "}
        {version && <span className="title-bar-version">v{version}</span>}
      </h1>
      <div className="window-controls">
        <button
          type="button"
          onClick={onSettingsToggle}
          className="window-control-button window-control-settings"
          title={t("titlebar.settings")}
        >
          <img src={settingsIcon} alt={t("titlebar.settings")} />
          {hasUpdate && <span className="window-control-badge" />}
        </button>
        <button
          type="button"
          onClick={handleMinimize}
          className="window-control-button"
          title={t("titlebar.minimize")}
        >
          <img src={minimizeIcon} alt={t("titlebar.minimize")} />
        </button>
        <button
          type="button"
          onClick={handleClose}
          className="window-control-button window-control-close"
          title={t("titlebar.close")}
        >
          <img src={closeIcon} alt={t("titlebar.close")} />
        </button>
      </div>
    </div>
  );
}

export default TitleBar;
