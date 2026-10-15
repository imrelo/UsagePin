import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { MaterialIcon } from "../shared/MaterialIcon";
import { useSettings } from "./useSettings";
import { useCodexUsage } from "../shared/useCodexUsage";
import { isUsageStale, usageStatusLabel } from "../shared/codexUsage";
import { providerNames, readUsageProvider } from "../shared/useUsageProvider";
import "./Settings.css";

function Settings() {
  const [checkingConnection, setCheckingConnection] = useState(false);
  const [supportError, setSupportError] = useState("");
  const resetDialog = useRef<HTMLDialogElement>(null);
  const [activeTab, setActiveTab] = useState<"appearance" | "account">("appearance");
  const {
    autoActivation,
    setAutoActivation,
    activationPending,
    activationError,
    startWithWindows,
    setStartWithWindows,
    startupPending,
    startupError,
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
    usageProvider,
    setUsageProvider,
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
    resetting,
    resetMessage,
    resetError,
    resetSettings,
  } = useSettings();
  const { usage, now, connectionError, refreshUsage } = useCodexUsage(usageProvider);
  const stale = isUsageStale(usage, now);

  useEffect(() => {
    if (settingsClosing) resetDialog.current?.close();
  }, [settingsClosing]);

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
        <div className="settings-scroll">
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
            <span className="settings-label">Start with Windows</span>
            <button className={`mode-toggle${startWithWindows ? " is-enabled" : ""}`} type="button" aria-label="Start with Windows" aria-describedby="windows-startup-hint" aria-pressed={startWithWindows} aria-busy={startupPending} disabled={startupPending || resetting} onClick={() => { void setStartWithWindows(!startWithWindows).catch(() => {}); }}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={!startWithWindows ? "is-current" : ""} aria-hidden={startWithWindows}>Off</span>
                <span className={startWithWindows ? "is-current" : ""} aria-hidden={!startWithWindows}>On</span>
              </span>
            </button>
          </div>
          <p className="settings-hint" id="windows-startup-hint">Open UsagePin automatically when you sign in to Windows. Enabled by default.</p>
          {startupError && <p className="connection-error" role="status">{startupError}</p>}
        </div>
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
        <div className="provider-setting">
          <div className="settings-row">
            <label className="settings-label" htmlFor="usage-provider">Widget displays</label>
            <div className="provider-select">
              <select id="usage-provider" value={usageProvider} onChange={(event) => setUsageProvider(readUsageProvider(event.currentTarget.value))} aria-describedby="usage-provider-hint">
                <option value="codex">Codex</option>
                <option value="antigravity">Antigravity</option>
              </select>
              <MaterialIcon name="expand_less" />
            </div>
          </div>
          <p className="settings-hint" id="usage-provider-hint">Display one service at a time. Your choice is saved automatically.</p>
        </div>
        <div className="account-status">
          <div className="account-status-heading">
            <h3>{providerNames[usageProvider]} connection</h3>
            <span className={`connection-badge${usage.status === "ready" && !stale && !connectionError ? " is-connected" : ""}`} role="status">{connectionError ? "Unavailable" : usageStatusLabel(usage, now)}</span>
          </div>
          <p className="settings-hint">{connectionError || usage.message}</p>
          <dl className="connection-details">
            <div><dt>Account</dt><dd>{usage.account?.email ?? (usage.account ? `Signed in with ${providerNames[usageProvider]}` : "—")}</dd></div>
            {usageProvider === "codex" && <div><dt>Workspace</dt><dd>{usage.account?.accountId ?? "—"}</dd></div>}
            <div><dt>Plan</dt><dd>{usage.account?.plan ?? "—"}</dd></div>
            <div><dt>{usageProvider === "codex" ? "Session file" : "App source"}</dt><dd>{usage.sourcePath || "—"}</dd></div>
            <div><dt>Last updated</dt><dd>{usage.updatedAt === null ? "—" : new Date(usage.updatedAt * 1000).toLocaleString()}{stale ? " (last known data)" : ""}</dd></div>
          </dl>
          <div className="connection-actions">
            <button className="connection-check" type="button" disabled={checkingConnection || usage.refreshing} onClick={async () => {
              setCheckingConnection(true);
              try { await refreshUsage(); } finally { setCheckingConnection(false); }
            }}><MaterialIcon name="refresh" />{checkingConnection || usage.refreshing ? "Checking…" : "Check connection / Refresh"}</button>
          </div>
          <p className="settings-hint">{usageProvider === "codex" ? "Uses your existing Codex sign-in. If the session expires, open Codex and sign in again, then check the connection here." : "Keep Antigravity running and signed in. UsagePin reads the same model quota summary as Models & Usage."}</p>
        </div>
        {usageProvider === "codex" && <div className="theme-setting auto-activation-setting">
          <div className="settings-row">
            <span className="settings-label">Auto-activate after 5h reset</span>
            <button className={`mode-toggle${autoActivation.enabled ? " is-enabled" : ""}`} type="button" aria-label="Auto-activate after 5-hour quota reset" aria-describedby="auto-activation-hint" aria-pressed={autoActivation.enabled} aria-busy={activationPending} disabled={activationPending || resetting} onClick={() => { void setAutoActivation(!autoActivation.enabled).catch(() => {}); }}>
              <span className="mode-dot" aria-hidden="true" />
              <span className="mode-label">
                <span className={!autoActivation.enabled ? "is-current" : ""} aria-hidden={autoActivation.enabled}>Off</span>
                <span className={autoActivation.enabled ? "is-current" : ""} aria-hidden={!autoActivation.enabled}>On</span>
              </span>
            </button>
          </div>
          <p className="settings-hint" id="auto-activation-hint">Automatically start a new 5-hour window after reset. Uses a little quota.</p>
          <p className="settings-hint" role="status">{autoActivation.message}</p>
          {autoActivation.lastAttemptAt !== null && <p className="settings-hint">Last activation attempt: {new Date(autoActivation.lastAttemptAt * 1000).toLocaleString()}</p>}
          {activationError && <p className="connection-error" role="status">{activationError}</p>}
        </div>}
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
        <section className="reset-setting" aria-labelledby="reset-setting-title">
          <h3 id="reset-setting-title">Restore defaults</h3>
          <p className="settings-hint" id="reset-setting-hint">Restore all UsagePin preferences to their default values.</p>
          <button className="reset-button" type="button" disabled={resetting} aria-describedby="reset-setting-hint" aria-haspopup="dialog" onClick={() => resetDialog.current?.showModal()}>
            <MaterialIcon name="refresh" />{resetting ? "Resetting…" : "Reset settings"}
          </button>
          {(resetMessage || resetError) && <p className={resetError ? "connection-error" : "settings-hint"} role="status">{resetError || resetMessage}</p>}
        </section>
        </div>
        <footer className="settings-footer">
          <span className="settings-label"><MaterialIcon name="check" />Changes apply instantly</span>
          <button className="support-button" type="button" title="Support UsagePin on Ko-fi. Opens in your browser." onClick={async () => {
              setSupportError("");
              try {
                await openUrl("https://ko-fi.com/mdbrelo/?hidefeed=true&widget=true&embed=true&preview=true");
              } catch {
                setSupportError("Could not open Ko-fi. Try again.");
              }
            }}>Support UsagePin</button>
          <span className="settings-shortcut"><kbd>Esc</kbd> to close</span>
        </footer>
        {supportError && <p className="connection-error" role="status">{supportError}</p>}
      </section>
      <dialog className="reset-confirmation" ref={resetDialog} aria-labelledby="reset-confirmation-title" aria-describedby="reset-confirmation-description" onKeyDown={(event) => {
        if (event.key === "Escape") event.stopPropagation();
      }}>
        <span className="reset-confirmation-icon"><MaterialIcon name="warning" /></span>
        <h3 id="reset-confirmation-title">Reset all settings?</h3>
        <p id="reset-confirmation-description">All UsagePin preferences will return to their defaults. Your current choices will be replaced.</p>
        <div className="reset-confirmation-actions">
          <button className="reset-cancel" type="button" autoFocus onClick={() => resetDialog.current?.close()}>Cancel</button>
          <button className="reset-button" type="button" disabled={resetting} onClick={() => {
            resetDialog.current?.close();
            void resetSettings();
          }}>Reset settings</button>
        </div>
      </dialog>
    </main>
  );
}

export default Settings;
