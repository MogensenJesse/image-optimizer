import { useEffect, useMemo, useRef, useState } from "react";
import closeIcon from "../assets/icons/close.svg";
import { useTranslation } from "../i18n";

function FloatingMenu({
  settings,
  onSettingsChange,
  disabled: _disabled,
  show,
  onClose,
}) {
  const { t } = useTranslation();
  const [resizeMode, setResizeMode] = useState(settings.resize.mode || "none");

  // Memoized function to calculate gradient color based on percentage
  const calculateGradientColor = useMemo(() => {
    return (percentage) => {
      // Start color: #d7bb21 (215, 187, 33)
      const startR = 215,
        startG = 187,
        startB = 33;
      // End color: #62cd20 (98, 205, 32)
      const endR = 98,
        endG = 205,
        endB = 32;

      // Calculate the color at the current percentage
      const r = Math.round(startR + (endR - startR) * (percentage / 100));
      const g = Math.round(startG + (endG - startG) * (percentage / 100));
      const b = Math.round(startB + (endB - startB) * (percentage / 100));

      return `rgb(${r}, ${g}, ${b})`;
    };
  }, []); // Empty dependency array means this is only calculated once

  // Function to get quality label based on slider value
  const getQualityLabel = (value) => {
    if (value === 100) return t("menu.quality.lossless");
    if (value >= 90) return t("menu.quality.nearLossless");
    if (value >= 70) return t("menu.quality.excellent");
    if (value >= 50) return t("menu.quality.good");
    if (value >= 30) return t("menu.quality.fair");
    if (value >= 10) return t("menu.quality.poor");
    return t("menu.quality.broken");
  };

  const sliderContainerRef = useRef(null);

  // Update the CSS variables when the quality value changes
  useEffect(() => {
    if (sliderContainerRef.current) {
      const percentage = settings.quality.global;
      const currentColor = calculateGradientColor(percentage);

      sliderContainerRef.current.style.setProperty(
        "--slider-value",
        `${percentage}%`,
      );
      sliderContainerRef.current.style.setProperty(
        "--slider-color",
        currentColor,
      );
    }
  }, [settings.quality.global, calculateGradientColor]);

  const handleQualityChange = (value) => {
    const qualityValue = parseInt(value, 10);

    // Update the CSS variables directly for immediate visual feedback
    if (sliderContainerRef.current) {
      const currentColor = calculateGradientColor(qualityValue);

      sliderContainerRef.current.style.setProperty(
        "--slider-value",
        `${qualityValue}%`,
      );
      sliderContainerRef.current.style.setProperty(
        "--slider-color",
        currentColor,
      );
    }

    onSettingsChange({
      ...settings,
      quality: {
        ...settings.quality,
        global: qualityValue,
      },
    });
  };

  const handleResizeChange = (mode, value) => {
    const newResize = {
      width: null,
      height: null,
      maintainAspect: true,
      mode: mode,
      size: value ? parseInt(value, 10) : null,
    };

    onSettingsChange({
      ...settings,
      resize: newResize,
    });
  };

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
      />

      <div className={`floating-menu ${show ? "floating-menu--open" : ""}`}>
        <div className="floating-menu__panel">
          <div className="floating-menu__item">
            <div className="floating-menu__content floating-menu__content--column">
              <div className="header-row">
                <span>{t("menu.quality")}</span>
                <span className="value">
                  <span className="menu-control--label">
                    {getQualityLabel(settings.quality.global)}
                  </span>{" "}
                  {settings.quality.global}%
                </span>
              </div>
              <div ref={sliderContainerRef} className="slider-container">
                <input
                  className="menu-control--range"
                  type="range"
                  min="0"
                  max="100"
                  value={settings.quality.global}
                  onChange={(e) => handleQualityChange(e.target.value)}
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
                  onChange={(e) => {
                    setResizeMode(e.target.value);
                    handleResizeChange(e.target.value, settings.resize.size);
                  }}
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
