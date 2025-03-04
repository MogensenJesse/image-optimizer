import { getCurrentWindow } from '@tauri-apps/api/window';
import { useState, useEffect } from 'react';
import closeIcon from '../assets/icons/close.svg';
import minimizeIcon from '../assets/icons/minimize.svg';

function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    // Listen to window resized event to update the maximize/restore button state
    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then(setIsMaximized);
    });

    // Check initial state
    appWindow.isMaximized().then(setIsMaximized);

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  const handleMinimize = () => {
    appWindow.minimize();
  };

  const handleClose = () => {
    appWindow.close();
  };

  return (
    <div className="title-bar">
      <h1 className="title-bar-title">Image optimizer</h1>
      <div className="window-controls">
        <button 
          onClick={handleMinimize} 
          className="window-control-button"
          title="Minimize"
        >
          <img src={minimizeIcon} alt="Minimize" />
        </button>
        <button 
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