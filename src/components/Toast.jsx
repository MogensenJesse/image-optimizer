// src/components/Toast.jsx
import { useCallback, useEffect, useRef, useState } from "react";

const AUTO_DISMISS_MS = 5000;
const EXIT_ANIMATION_MS = 250;

function Toast({ message, type = "warning", onClose }) {
  const [dismissing, setDismissing] = useState(false);
  const timerRef = useRef(null);

  const handleDismiss = useCallback(() => {
    if (dismissing) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    setDismissing(true);
    timerRef.current = setTimeout(onClose, EXIT_ANIMATION_MS);
  }, [onClose, dismissing]);

  useEffect(() => {
    timerRef.current = setTimeout(handleDismiss, AUTO_DISMISS_MS);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [handleDismiss]);

  return (
    <div
      className={`toast toast--${type}${dismissing ? " toast--dismissing" : ""}`}
    >
      <span className="toast__message">{message}</span>
      <button type="button" className="toast__close" onClick={handleDismiss}>
        ×
      </button>
    </div>
  );
}

export default Toast;
