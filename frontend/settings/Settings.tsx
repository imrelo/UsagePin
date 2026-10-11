import { useState } from "react";
import type { CSSProperties } from "react";
import { MaterialIcon } from "../shared/MaterialIcon";
import { useSettings } from "./useSettings";
import { useCodexUsage } from "../shared/useCodexUsage";
import { isUsageStale, usageStatusLabel } from "../shared/codexUsage";
import "./Settings.css";

function Settings() {
  const { usage, now, connectionError, refreshUsage } = useCodexUsage();
  const [checkingConnection, setCheckingConnection] = useState(false);
  const stale = isUsageStale(usage, now);
  const [activeTab, setActiveTab] = useState<"appearance" | "account">("appearance");
  const {
    theme,
    setTheme,
    backgroundOpacity,
    setBackgroundOpacity,
    contentOpacity,
    setContentOpacity,
    usageShakeEnabled,
    setUsageShakeEnabled,
    usageDeductionEnabled,
    setUsageDeductionEnabled,
    refreshIntervalInput,
    refreshIntervalError,
    changeRefreshInterval,
    resetRefreshIntervalInput,
    clickThrough,
    modeError,
    modeErrorVisible,
    settingsClosing,
    settingsOpening,
    closeSettings,
    toggleClickThrough,
  } = useSettings();

  return (
    <main
      className={`settings-page${settingsClosing ? " is-closing" : ""}`}
    >
      <section className="settings-panel" key={settingsOpening} aria-labelledby="settings-title">
        <header className="settings-header" data-tauri-drag-region>
          <div className="settings-heading" data-tauri-drag-region>
            <h2 id="settings-title" data-tauri-drag-region>Settings</h2>
          </div>
          <button className="settings-button" type="button" aria-label="Close settings" onClick={closeSettings}>
            <MaterialIcon name="close" />
          </button>
        </header>
        <div
          className="settings-tabs"
          data-active-tab={activeTab}
          role="tablist"
          aria-label="Settings sections"
          onKeyDown={(event) => {
            let nextTab: typeof activeTab;
            if (event.key === "ArrowLeft" || event.key === "ArrowRight") nextTab = activeTab === "appearance" ? "account" : "appearance";
            else if (event.key === "Home") nextTab = "appearance";
            else if (event.key === "End") nextTab = "account";
            else return;
            event.preventDefault();
            setActiveTab(nextTab);
            event.currentTarget.querySelector<HTMLButtonElement>(`#settings-tab-${nextTab}`)?.focus();
          }}
        >
          <button className="settings-tab" id="settings-tab-appearance" type="button" role="tab" aria-selected={activeTab === "appearance"} aria-controls="settings-appearance" tabIndex={activeTab === "appearance" ? 0 : -1} onClick={() => setActiveTab("appearance")}>Appearance</button>
          <button className="settings-tab" id="settings-tab-account" type="button" role="tab" aria-selected={activeTab === "account"} aria-controls="settings-account" tabIndex={activeTab === "account" ? 0 : -1} onClick={() => setActiveTab("account")}>Account connection</button>
        </div>
        <div className="settings-content" id="settings-appearance" role="tabpanel" aria-labelledby="settings-tab-appearance" hidden={activeTab !== "appearance"}>
        <div className="theme-setting">
          <div className="settings-row">
            <span className="settings-label" id="theme-label"><MaterialIcon name="light_mode" />Theme</span>
            <button className={`mode-toggle${theme === "light" ? " is-enabled" : ""}`} type="button" aria-label="Light mode" aria-describedby="theme-label" aria-pressed={theme === "light"} onClick={() => setTheme(theme === "light" ? "dark" : "light")}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={theme === "dark" ? "is-current" : ""} aria-hidden={theme !== "dark"}>Dark</span>
                <span className={theme === "light" ? "is-current" : ""} aria-hidden={theme !== "light"}>Light</span>
              </span>
            </button>
          </div>
        </div>
        <div className="opacity-controls" role="group" aria-label="Appearance">
        <div className="opacity-setting" style={{ "--slider-progress": `${backgroundOpacity}%` } as CSSProperties}>
          <div className="settings-row">
            <label className="settings-label" htmlFor="background-opacity"><MaterialIcon name="opacity" />Background opacity</label>
            <output htmlFor="background-opacity">{backgroundOpacity}%</output>
          </div>
          <input
            className="opacity-slider"
            id="background-opacity"
            type="range"
            min="0"
            max="100"
            step="1"
            value={backgroundOpacity}
            aria-valuetext={`${backgroundOpacity}%`}
            onChange={(event) => setBackgroundOpacity(Number(event.currentTarget.value))}
          />
        </div>
        <div className="opacity-setting" style={{ "--slider-progress": `${contentOpacity}%` } as CSSProperties}>
          <div className="settings-row">
            <label className="settings-label" htmlFor="content-opacity"><MaterialIcon name="text_fields" />Content opacity</label>
            <output htmlFor="content-opacity">{contentOpacity}%</output>
          </div>
          <input
            className="opacity-slider"
            id="content-opacity"
            type="range"
            min="0"
            max="100"
            step="1"
            value={contentOpacity}
            aria-valuetext={`${contentOpacity}%`}
            onChange={(event) => setContentOpacity(Number(event.currentTarget.value))}
          />
        </div>
        </div>
        <div className="mouse-setting">
        <div className="settings-row">
          <span className="settings-label" id="mouse-input-label"><MaterialIcon name="mouse" />Mouse input</span>
          <button className={`mode-toggle${clickThrough ? " is-enabled" : ""}`} type="button" aria-label="Click-through" aria-describedby="mouse-input-label mouse-input-hint" aria-pressed={clickThrough} onClick={toggleClickThrough}>
            <span className="mode-dot" aria-hidden="true" />
            <span className="mode-label">
              <span className={!clickThrough ? "is-current" : ""} aria-hidden={clickThrough}>Interactive</span>
              <span className={clickThrough ? "is-current" : ""} aria-hidden={!clickThrough}>Click-through</span>
            </span>
          </button>
        </div>
        <p className="settings-hint" id="mouse-input-hint">Click-through lets clicks pass to the window underneath. Use the tray menu to turn interaction back on.</p>
        <p className={`settings-error${modeErrorVisible ? " is-visible" : ""}`} role="status" aria-hidden={!modeErrorVisible}>{modeError}</p>
        </div>
        <div className="mouse-setting">
          <div className="settings-row">
            <span className="settings-label" id="usage-shake-label">Usage spike animation</span>
            <button className={`mode-toggle${usageShakeEnabled ? " is-enabled" : ""}`} type="button" aria-label="Usage spike animation" aria-describedby="usage-shake-hint" aria-pressed={usageShakeEnabled} onClick={() => setUsageShakeEnabled(!usageShakeEnabled)}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={!usageShakeEnabled ? "is-current" : ""} aria-hidden={usageShakeEnabled}>Off</span>
                <span className={usageShakeEnabled ? "is-current" : ""} aria-hidden={!usageShakeEnabled}>On</span>
              </span>
            </button>
          </div>
          <p className="settings-hint" id="usage-shake-hint">Shake the usage bar when consumption increases by more than 10 percentage points in one update. Enabled by default.</p>
        </div>
        <div className="mouse-setting">
          <div className="settings-row">
            <span className="settings-label">Last usage deduction</span>
            <button className={`mode-toggle${usageDeductionEnabled ? " is-enabled" : ""}`} type="button" aria-label="Last usage deduction" aria-describedby="usage-deduction-hint" aria-pressed={usageDeductionEnabled} onClick={() => setUsageDeductionEnabled(!usageDeductionEnabled)}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={!usageDeductionEnabled ? "is-current" : ""} aria-hidden={usageDeductionEnabled}>Off</span>
                <span className={usageDeductionEnabled ? "is-current" : ""} aria-hidden={!usageDeductionEnabled}>On</span>
              </span>
            </button>
          </div>
          <p className="settings-hint" id="usage-deduction-hint">Show the last usage deduction, such as -5, between the remaining percentage and reset time. Enabled by default.</p>
        </div>
        </div>
        <div className="settings-content" id="settings-account" role="tabpanel" aria-labelledby="settings-tab-account" hidden={activeTab !== "account"}>
        <div className="account-status">
          <div className="account-status-heading">
            <h3>Codex Windows session</h3>
            <span className={`connection-badge${usage.status === "ready" && !stale && !connectionError ? " is-connected" : ""}`} role="status">{connectionError ? "Unavailable" : usageStatusLabel(usage, now)}</span>
          </div>
          <p className="settings-hint">{connectionError || usage.message}</p>
          <dl className="connection-details">
            <div><dt>Account</dt><dd>{usage.account?.email ?? (usage.account ? "Signed in with ChatGPT" : "—")}</dd></div>
            <div><dt>Workspace</dt><dd>{usage.account?.accountId ?? "—"}</dd></div>
            <div><dt>Plan</dt><dd>{usage.account?.plan ?? "—"}</dd></div>
            <div><dt>Session file</dt><dd>{usage.sourcePath || "—"}</dd></div>
            <div><dt>Last updated</dt><dd>{usage.updatedAt === null ? "—" : new Date(usage.updatedAt * 1000).toLocaleString()}{stale ? " (last known data)" : ""}</dd></div>
          </dl>
          <div className="connection-actions">
            <button className="connection-check" type="button" disabled={checkingConnection || usage.refreshing} onClick={async () => {
              setCheckingConnection(true);
              try { await refreshUsage(); } finally { setCheckingConnection(false); }
            }}><MaterialIcon name="refresh" />{checkingConnection || usage.refreshing ? "Checking…" : "Check connection / Refresh"}</button>
          </div>
          <p className="settings-hint">Uses your existing Codex sign-in. If the session expires, open Codex and sign in again, then check the connection here.</p>
        </div>
        <div className="refresh-setting">
          <div className="settings-row">
            <label className="settings-label" htmlFor="refresh-interval"><MaterialIcon name="refresh" />API refresh interval</label>
            <div className="refresh-input">
              <input
                id="refresh-interval"
                type="number"
                min="1"
                max={Number.MAX_SAFE_INTEGER}
                step="1"
                required
                aria-describedby="refresh-interval-hint"
                value={refreshIntervalInput}
                onChange={(event) => changeRefreshInterval(event.currentTarget.value)}
                onBlur={resetRefreshIntervalInput}
              />
              <span>seconds</span>
            </div>
          </div>
          <p className="settings-hint" id="refresh-interval-hint">Whole seconds, minimum 1. UsagePin refreshes automatically; temporary errors may extend the wait.</p>
          {refreshIntervalError && <p className="connection-error" role="status">{refreshIntervalError}</p>}
        </div>
        </div>
        <footer className="settings-footer">
          <span className="settings-label"><MaterialIcon name="check" />Changes apply instantly</span>
          <span className="settings-shortcut"><kbd>Esc</kbd> to close</span>
        </footer>
      </section>
    </main>
  );
}

export default Settings;
