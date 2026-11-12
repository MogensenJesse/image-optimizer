import closeIcon from "../assets/icons/close.svg";
import { useTranslation } from "../i18n";

// Slider colour runs from #d7bb21 (low quality) to #62cd20 (high quality)
const GRADIENT_START = [215, 187, 33];
const GRADIENT_END = [98, 205, 32];

function gradientColor(percentage) {
  const [r, g, b] = GRADIENT_START.map((start, i) =>
    Math.round(start + (GRADIENT_END[i] - start) * (percentage / 100)),
  );
  return `rgb(${r}, ${g}, ${b})`;
}

function FloatingMenu({ settings, onSettingsChange, show, onClose }) {
  const { t } = useTranslation();

  const getQualityLabel = (value) => {
    if (value === 100) return t("menu.quality.lossless");
    if (value >= 90) return t("menu.quality.nearLossless");
    if (value >= 70) return t("menu.quality.excellent");
    if (value >= 50) return t("menu.quality.good");
    if (value >= 30) return t("menu.quality.fair");
    if (value >= 10) return t("menu.quality.poor");
    return t("menu.quality.broken");
  };

  const handleResizeChange = (mode, value) => {
    // Empty, zero and negative input all mean "no size yet"
    const parsed = Number.parseInt(value, 10);
    onSettingsChange({
      ...settings,
      resize: { mode, size: parsed > 0 ? parsed : null },
    });
  };

  const resizeMode = settings.resize.mode;

  return (
    <>
      {/* biome-ignore lint/a11y/useSemanticElements: Overlay needs to be a div for full-screen coverage styling */}
      <div
        className={`floating-menu__overlay ${
          show ? "floating-menu__overlay--active" : ""
        }`}
        onClick={onClose}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === "Escape") {
            e.preventDefault();
            onClose();
          }
        }}
        role="button"
        tabIndex={0}
        aria-label={t("menu.close")}
        inert={!show}
      />

      <div
        className={`floating-menu ${show ? "floating-menu--open" : ""}`}
        inert={!show}
      >
        <div className="floating-menu__panel">
          <div className="floating-menu__item">
            <div className="floating-menu__content floating-menu__content--column">
              <div className="header-row">
                <span>{t("menu.quality")}</span>
                <span className="value">
                  <span className="menu-control--label">
                    {getQualityLabel(settings.quality)}
                  </span>{" "}
                  {settings.quality}%
                </span>
              </div>
              <div
                className="slider-container"
                style={{
                  "--slider-value": `${settings.quality}%`,
                  "--slider-color": gradientColor(settings.quality),
                }}
              >
                <input
                  className="menu-control--range"
                  type="range"
                  min="1"
                  max="100"
                  value={settings.quality}
                  onChange={(e) =>
                    onSettingsChange({
                      ...settings,
                      quality: Number.parseInt(e.target.value, 10),
                    })
                  }
                />
              </div>
            </div>
          </div>

          <div className="divider"></div>

          <div className="floating-menu__item">
            <div className="floating-menu__content">
              <span>{t("menu.resize")}</span>
              <div className="control-group">
                {resizeMode !== "none" && (
                  <div className="input-with-unit">
                    <input
                      type="number"
                      min="1"
                      placeholder={t("menu.resize.placeholder")}
                      value={settings.resize.size || ""}
                      onChange={(e) =>
                        handleResizeChange(resizeMode, e.target.value)
                      }
                      className="menu-control--input"
                    />
                    <span className="unit">px</span>
                  </div>
                )}
                <select
                  value={resizeMode}
                  onChange={(e) =>
                    handleResizeChange(e.target.value, settings.resize.size)
                  }
                  className="menu-control--select"
                >
                  <option value="none">{t("menu.resize.none")}</option>
                  <option value="width">{t("menu.resize.width")}</option>
                  <option value="height">{t("menu.resize.height")}</option>
                  <option value="longest">{t("menu.resize.longest")}</option>
                  <option value="shortest">{t("menu.resize.shortest")}</option>
                </select>
              </div>
            </div>
          </div>

          <div className="divider"></div>

          <div className="floating-menu__item">
            <div className="floating-menu__content">
              <span>{t("menu.convertTo")}</span>
              <select
                value={settings.outputFormat}
                onChange={(e) =>
                  onSettingsChange({
                    ...settings,
                    outputFormat: e.target.value,
                  })
                }
                className="menu-control--select"
              >
                <option value="original">{t("menu.format.original")}</option>
                <option value="jpeg">JPEG</option>
                <option value="png">PNG</option>
                <option value="webp">WEBP</option>
                <option value="avif">AVIF</option>
              </select>
            </div>
          </div>

          <button type="button" onClick={onClose} aria-label={t("menu.close")}>
            <img className="floating-menu__close" src={closeIcon} alt="" />
          </button>
        </div>
      </div>
    </>
  );
}

export default FloatingMenu;
