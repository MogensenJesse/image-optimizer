import { useState } from 'react';
import './FloatingMenu.scss';

const ResizeControls = ({ settings, onSettingsChange }) => {
  const [resizeMode, setResizeMode] = useState('none'); // none, width, height, longest, shortest

  const handleResizeChange = (mode, value) => {
    const newResize = {
      width: null,
      height: null,
      maintainAspect: true,
      mode: mode,
      size: value ? parseInt(value) : null
    };

    onSettingsChange({
      ...settings,
      resize: newResize
    });
  };

  return (
    <div className="resize-controls">
      <select 
        value={resizeMode}
        onChange={(e) => {
          setResizeMode(e.target.value);
          handleResizeChange(e.target.value, settings.resize.size);
        }}
      >
        <option value="none">No resize</option>
        <option value="width">Width</option>
        <option value="height">Height</option>
        <option value="longest">Longest edge</option>
        <option value="shortest">Shortest edge</option>
      </select>
      
      {resizeMode !== 'none' && (
        <input 
          type="number"
          min="1"
          placeholder={`Target ${resizeMode}`}
          value={settings.resize.size || ''}
          onChange={(e) => handleResizeChange(resizeMode, e.target.value)}
        />
      )}
    </div>
  );
};

function FloatingMenu({ settings, onSettingsChange }) {
  const [isOpen, setIsOpen] = useState(false);
  
  const handleQualityChange = (value) => {
    console.log('Quality changed to:', value); // Debug log
    onSettingsChange({
      ...settings,
      quality: {
        ...settings.quality,
        global: parseInt(value)
      }
    });
  };
  
  const handleClose = () => {
    setIsOpen(false);
  };

  return (
    <>
      <div 
        className={`menu-overlay ${isOpen ? 'menu-overlay--active' : ''}`}
        onClick={() => setIsOpen(false)}
      />

      <div className={`floating-menu ${isOpen ? 'floating-menu--open' : ''}`}>
        <button 
          className="fab"
          onClick={() => setIsOpen(!isOpen)}
          aria-label="Settings"
        >
          <svg className="fab__icon" viewBox="0 0 24 24">
            <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"/>
          </svg>
        </button>

        <div className="menu-items">
          <div className="menu-item">
            <button className="menu-item__button">
              Quality ({settings.quality.global}%)
              <input 
                className="form-control--range"
                type="range" 
                min="0" 
                max="100" 
                value={settings.quality.global}
                onChange={(e) => handleQualityChange(e.target.value)}
              />
            </button>
          </div>

          <div className="menu-item">
            <button className="menu-item__button">
              Resize
              <ResizeControls 
                settings={settings} 
                onSettingsChange={onSettingsChange} 
              />
            </button>
          </div>

          <div className="menu-item">
            <button className="menu-item__button">
              Format
              <select 
                value={settings.outputFormat}
                onChange={(e) => onSettingsChange({
                  ...settings,
                  outputFormat: e.target.value
                })}
              >
                <option value="original">Original</option>
                <option value="jpeg">JPEG</option>
                <option value="png">PNG</option>
                <option value="webp">WebP</option>
                <option value="avif">AVIF</option>
              </select>
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

export default FloatingMenu; 