import { useState, useEffect, useRef } from "react";
import closeIcon from "../assets/icons/close.svg";

function FloatingMenu({ settings, onSettingsChange, disabled, show, onClose }) {
  const [resizeMode, setResizeMode] = useState(settings.resize.mode || "none"); // none, width, height, longest, shortest
  const qualitySliderRef = useRef(null);

  // Calculate the gradient color based on the percentage
  const calculateGradientColor = (percentage) => {
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

  // Update the CSS variables when the quality value changes
  useEffect(() => {
    if (qualitySliderRef.current) {
      const percentage = settings.quality.global;
      const currentColor = calculateGradientColor(percentage);

      qualitySliderRef.current.style.setProperty(
        "--slider-value",
        `${percentage}%`
      );
      qualitySliderRef.current.style.setProperty(
        "--slider-color",
        currentColor
      );
    }
  }, [settings.quality.global]);

  const handleQualityChange = (value) => {
    const qualityValue = parseInt(value);

    // Update the CSS variables directly for immediate visual feedback
    if (qualitySliderRef.current) {
      const currentColor = calculateGradientColor(qualityValue);

      qualitySliderRef.current.style.setProperty(
        "--slider-value",
        `${qualityValue}%`
      );
      qualitySliderRef.current.style.setProperty(
        "--slider-color",
        currentColor
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
      size: value ? parseInt(value) : null,
    };

    onSettingsChange({
      ...settings,
      resize: newResize,
    });
  };

  return (
    <>
      <div
        className={`floating-menu__overlay ${
          show ? "floating-menu__overlay--active" : ""
        }`}
        onClick={onClose}
      />

      <div className={`floating-menu ${show ? "floating-menu--open" : ""}`}>
        <div className="floating-menu__panel">
          <div className="floating-menu__item">
            <div className="floating-menu__content floating-menu__content--column">
              <div className="header-row">
                <span>Quality</span>
                <span className="value">{settings.quality.global}%</span>
              </div>
              <input
                ref={qualitySliderRef}
                className="menu-control--range"
                type="range"
                min="0"
                max="100"
                value={settings.quality.global}
                onChange={(e) => handleQualityChange(e.target.value)}
              />
            </div>
          </div>

          <div className="divider"></div>

          <div className="floating-menu__item">
            <div className="floating-menu__content">
              <span>Resize</span>
              <div className="control-group">
                {resizeMode !== "none" && (
                  <div className="input-with-unit">
                    <input
                      type="number"
                      min="1"
                      placeholder="Size"
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
                  <option value="none">Don't resize</option>
                  <option value="width">Width</option>
                  <option value="height">Height</option>
                  <option value="longest">Longest edge</option>
                  <option value="shortest">Shortest edge</option>
                </select>
              </div>
            </div>
          </div>

          <div className="divider"></div>

          <div className="floating-menu__item">
            <div className="floating-menu__content">
              <span>Convert to</span>
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
                <option value="original">Original</option>
                <option value="jpeg">JPEG</option>
                <option value="png">PNG</option>
                <option value="webp">WEBP</option>
                <option value="avif">AVIF</option>
              </select>
            </div>
          </div>

          <button onClick={onClose} aria-label="Close menu">
            <img className="floating-menu__close" src={closeIcon} alt="Close" />
          </button>
        </div>
      </div>
    </>
  );
}

export default FloatingMenu;
