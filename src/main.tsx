import React from "react";
import ReactDOM from "react-dom/client";
import App from "@/App";
import { logError } from "@/services/logger";
import "uno.css";
import "@/styles/theme.css";

declare global {
  interface Window {
    __rtoolGlobalErrorHandlersInstalled?: boolean;
  }
}

if (typeof window !== "undefined" && !window.__rtoolGlobalErrorHandlersInstalled) {
  window.addEventListener("error", (event) => {
    logError("window.onerror", "unhandled_error", {
      message: event.message,
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
      stack: event.error instanceof Error ? event.error.stack : undefined,
    });
  });

  window.addEventListener("unhandledrejection", (event) => {
    const reason = event.reason;
    const message = reason instanceof Error ? reason.message : String(reason);
    logError("window.unhandledrejection", "unhandled_promise_rejection", {
      message,
      stack: reason instanceof Error ? reason.stack : undefined,
    });
  });

  window.__rtoolGlobalErrorHandlersInstalled = true;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
