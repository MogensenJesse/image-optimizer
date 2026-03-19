// src/components/Toast.jsx

function Toast({ message, type = "warning", onClose }) {
  return (
    <div className={`toast toast--${type}`}>
      <span className="toast__message">{message}</span>
      <button type="button" className="toast__close" onClick={onClose}>
        ×
      </button>
    </div>
  );
}

export default Toast;
