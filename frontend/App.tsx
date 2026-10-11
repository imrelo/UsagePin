import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type SavedPosition = { x: number; y: number };

const positionKey = "usagepin.window-position";

const usageRows = [
  { label: "5h", remaining: 100, reset: "11:18 PM" },
  { label: "Weekly", remaining: 95, reset: "Oct 14" },
];

function App() {
  const settingsDialog = useRef<HTMLDialogElement>(null);
  const [clickThrough, setClickThrough] = useState(false);
  const [modeError, setModeError] = useState("");
  const [modeErrorVisible, setModeErrorVisible] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlistenMoved: (() => void) | undefined;
    let unlistenMode: (() => void) | undefined;

    async function restoreAndListen() {
      const storedPosition = window.localStorage.getItem(positionKey);
      if (storedPosition) {
        try {
          const position = JSON.parse(storedPosition) as SavedPosition;
          if (Number.isFinite(position.x) && Number.isFinite(position.y)) {
            await appWindow.setPosition(
              new PhysicalPosition(position.x, position.y),
            );
          }
        } catch {
          window.localStorage.removeItem(positionKey);
        }
      }

      const stopMoved = await appWindow.onMoved(({ payload }) => {
        window.localStorage.setItem(
          positionKey,
          JSON.stringify({ x: payload.x, y: payload.y }),
        );
      });
      if (disposed) stopMoved();
      else unlistenMoved = stopMoved;

      const stopMode = await listen<boolean>(
        "click-through-changed",
        ({ payload }) => setClickThrough(payload),
      );
      if (disposed) stopMode();
      else unlistenMode = stopMode;
    }

    void restoreAndListen();
    return () => {
      disposed = true;
      unlistenMoved?.();
      unlistenMode?.();
    };
  }, []);

  async function toggleClickThrough() {
    setModeErrorVisible(false);
    try {
      setClickThrough(await invoke<boolean>("toggle_click_through"));
    } catch {
      setModeError("Use the UsagePin tray menu to change mouse mode.");
      setModeErrorVisible(true);
    }
  }

  return (
    <main
      className="app-shell"
      onMouseDown={(event) => {
        if (event.button !== 0 || (event.target as Element).closest("button, dialog")) return;
        void getCurrentWindow().startDragging();
      }}
    >
      <section className="usage-pin" aria-label="UsagePin Codex usage">
        <header className="card-header">
          <div className="brand-lockup">
            <button
              className={`brand-mark${clickThrough ? " is-enabled" : ""}`}
              type="button"
              aria-label={clickThrough ? "Use the tray menu to turn interaction back on" : "Enable click-through"}
              aria-pressed={clickThrough}
              title={clickThrough ? "Use the tray menu to turn interaction back on" : "Enable click-through"}
              onClick={toggleClickThrough}
            />
            <div className="brand-copy">
              <span className="brand-name">UsagePin</span>
              <span className="brand-caption">CODEX USAGE</span>
            </div>
          </div>
          <button
            className="settings-button"
            type="button"
            aria-label="Settings"
            aria-haspopup="dialog"
            title="Settings"
            onClick={() => settingsDialog.current?.showModal()}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
              <path d="m9.5 3-.6 2.4-1.6.9L5 5.6 2.5 9.9l1.8 1.7v1.8l-1.8 1.7L5 19.4l2.3-.7 1.6.9.6 2.4h5l.6-2.4 1.6-.9 2.3.7 2.5-4.3-1.8-1.7v-1.8l1.8-1.7L19 5.6l-2.3.7-1.6-.9-.6-2.4z" />
              <circle cx="12" cy="12.5" r="3" />
            </svg>
          </button>
        </header>

        <div className="card-divider" />

        <section className="usage-list" aria-label="Usage remaining">
          {usageRows.map((row) => (
            <article className="usage-row" key={row.label}>
              <div className="row-main">
                <span className="usage-label">{row.label}</span>
                <div
                  className="usage-track"
                  role="progressbar"
                  aria-label={`${row.label} usage remaining`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={row.remaining}
                >
                  <span style={{ width: `${row.remaining}%` }} />
                </div>
              </div>
              <div className="row-value">
                <strong>{row.remaining}%</strong>
                <span>{row.reset}</span>
              </div>
            </article>
          ))}
        </section>

        <footer className="card-footer">
          <span className="demo-label"><span aria-hidden="true" />SAMPLE DATA</span>
          <span className="footer-hint">Toggle input from tray</span>
        </footer>
        <p className={`mode-error${modeErrorVisible ? " is-visible" : ""}`} role="status" aria-hidden={!modeErrorVisible}>{modeError}</p>
        <dialog className="settings-dialog" ref={settingsDialog} aria-labelledby="settings-title">
          <header className="settings-header">
            <h2 id="settings-title">Settings</h2>
            <button className="settings-button" type="button" aria-label="Close settings" autoFocus onClick={() => settingsDialog.current?.close()}>
              <span aria-hidden="true">×</span>
            </button>
          </header>
          <div className="settings-row">
            <span>Mouse input</span>
            <button className={`mode-toggle${clickThrough ? " is-enabled" : ""}`} type="button" aria-pressed={clickThrough} onClick={toggleClickThrough}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={!clickThrough ? "is-current" : ""} aria-hidden={clickThrough}>INTERACTIVE</span>
                <span className={clickThrough ? "is-current" : ""} aria-hidden={!clickThrough}>THROUGH</span>
              </span>
            </button>
          </div>
          <p className="settings-hint">Click-through lets clicks pass to the window underneath. Use the tray menu to turn interaction back on.</p>
          <p className={`settings-error${modeErrorVisible ? " is-visible" : ""}`} role="status" aria-hidden={!modeErrorVisible}>{modeError}</p>
        </dialog>
      </section>
    </main>
  );
}

export default App;
