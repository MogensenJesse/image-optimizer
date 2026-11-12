// src/components/Toast.jsx
import { useEffect, useState } from "react";

const AUTO_DISMISS_MS = 5000;

function Toast({ message, type = "warning", onClose }) {
  const [dismissing, setDismissing] = useState(false);

  // Runs once per mount; parent re-renders must not restart the timer.
  useEffect(() => {
    const timer = setTimeout(() => setDismissing(true), AUTO_DISMISS_MS);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div
      className={`toast toast--${type}${dismissing ? " toast--dismissing" : ""}`}
      // The entry animation also fires animationend; only the exit unmounts.
      onAnimationEnd={() => dismissing && onClose()}
    >
      <span className="toast__message">{message}</span>
      <button
        type="button"
        className="toast__close"
        onClick={() => setDismissing(true)}
      >
        ×
      </button>
    </div>
  );
}

export default Toast;
