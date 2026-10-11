import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppearance } from "./shared/useAppearance";
import { useClickThrough } from "./shared/useClickThrough";
import { MaterialIcon } from "./shared/MaterialIcon";
import { useCompactView } from "./useCompactView";
import { useCodexUsage } from "./shared/useCodexUsage";
import { providerNames, useUsageProvider } from "./shared/useUsageProvider";
import { useRefreshInterval } from "./shared/useRefreshInterval";
import { useUsageShakeSetting } from "./shared/useUsageShakeSetting";
import { useUsageDeductionSetting } from "./shared/useUsageDeductionSetting";
import { isUsageStale, latestUsageDeductions, quotaLabel, refreshCountdownLabel, resetLabel, sameUsageIdentity, usageSpikeIds, usageStatusLabel } from "./shared/codexUsage";
import type { UsageSnapshot, UsageWindow } from "./shared/codexUsage";
import "./App.css";

function UsagePin() {
  const { usageProvider } = useUsageProvider();
  const { usage, now, connectionError } = useCodexUsage(usageProvider);
  const { usageShakeEnabled } = useUsageShakeSetting();
  const { usageDeductionEnabled } = useUsageDeductionSetting();
  const previousUsage = useRef<UsageSnapshot | null>(null);
  const [usageDeductions, setUsageDeductions] = useState<Record<string, number>>({});
  const usageTracks = useRef(new Map<string, HTMLDivElement>());
  useRefreshInterval();
  const stale = isUsageStale(usage, now);
  const usageRows: UsageWindow[] = usage.windows.length ? usage.windows : usageProvider === "antigravity" ? [
    { id: "gemini-weekly", label: "Gemini", remainingPercent: null, windowSeconds: 604800, resetsAt: null },
    { id: "3p-weekly", label: "Claude/GPT", remainingPercent: null, windowSeconds: 604800, resetsAt: null },
  ] : [
    { id: "primary", remainingPercent: null, windowSeconds: null, resetsAt: null },
    { id: "secondary", remainingPercent: null, windowSeconds: null, resetsAt: null },
  ];
  const [appScale, setAppScale] = useState(() => window.innerWidth / 320);
  const { backgroundOpacity, contentOpacity } = useAppearance();
  const { clickThrough, toggleClickThrough: toggleMode } = useClickThrough();
  const [modeError, setModeError] = useState("");
  const [modeErrorVisible, setModeErrorVisible] = useState(false);
  const { compact, chromeProgress, resizing, toggleCompactView } = useCompactView((message) => {
    setModeError(message);
    setModeErrorVisible(true);
  });

  useEffect(() => {
    const identityChanged = previousUsage.current !== null && !sameUsageIdentity(previousUsage.current, usage);
    if (identityChanged) {
      previousUsage.current = null;
      setUsageDeductions({});
    }
    if (!usageShakeEnabled || identityChanged) {
      for (const track of usageTracks.current.values()) {
        track.getAnimations().filter((animation) => animation.id === "usage-spike-shake").forEach((animation) => animation.cancel());
      }
    }
    if (usage.status !== "ready" || usage.refreshing) return;
    const spikeIds = usageSpikeIds(previousUsage.current, usage);
    const previous = previousUsage.current;
    setUsageDeductions((latest) => latestUsageDeductions(previous, usage, latest));
    previousUsage.current = usage;
    if (!usageShakeEnabled || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    for (const id of spikeIds) {
      const track = usageTracks.current.get(id);
      if (!track) continue;
      track.getAnimations().filter((animation) => animation.id === "usage-spike-shake").forEach((animation) => animation.cancel());
      const animation = track.animate([
        { transform: "translateX(0)" },
        { transform: "translateX(-3px)" },
        { transform: "translateX(3px)" },
        { transform: "translateX(-2px)" },
        { transform: "translateX(2px)" },
        { transform: "translateX(0)" },
      ], { duration: 500, iterations: 6, easing: "ease-in-out" });
      animation.id = "usage-spike-shake";
    }
  }, [usage, usageShakeEnabled]);

  useEffect(() => {
    const updateScale = () => setAppScale(window.innerWidth / 320);
    window.addEventListener("resize", updateScale);
    return () => window.removeEventListener("resize", updateScale);
  }, []);

  async function toggleClickThrough() {
    setModeErrorVisible(false);
    try {
      await toggleMode();
    } catch {
      setModeError("Use the UsagePin tray menu to change mouse mode.");
      setModeErrorVisible(true);
    }
  }

  async function openSettings() {
    setModeErrorVisible(false);
    try {
      await invoke("open_settings");
    } catch {
      setModeError("Could not open Settings. Try again.");
      setModeErrorVisible(true);
    }
  }

  return (
    <main
      className={`app-shell${compact ? " is-compact" : ""}`}
      style={{ "--background-opacity": backgroundOpacity / 100, "--content-opacity": contentOpacity / 100, "--app-scale": appScale, "--chrome-progress": chromeProgress } as CSSProperties}
      onMouseDown={(event) => {
        if (event.button !== 0 || (event.target as Element).closest("button")) return;
        void getCurrentWindow().startDragging();
      }}
    >
      <section className="usage-pin" aria-label={`UsagePin ${providerNames[usageProvider]} usage`}>
        <header className="card-header" id="usage-card-header" aria-hidden={compact} inert={compact}>
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
              <span className="brand-caption">{providerNames[usageProvider].toUpperCase()} USAGE</span>
            </div>
          </div>
          <button
            className="settings-button"
            type="button"
            aria-label="Settings"
            aria-haspopup="dialog"
            title="Settings"
            onClick={openSettings}
          >
            <MaterialIcon name="settings" />
          </button>
        </header>

        <div className="card-divider">
          <button
            className="compact-toggle"
            type="button"
            aria-label={compact ? "Show header and footer" : "Hide header and footer"}
            aria-expanded={!compact}
            aria-controls="usage-card-header usage-card-footer"
            title={compact ? "Show header and footer" : "Hide header and footer"}
            disabled={resizing}
            onClick={() => {
              setModeErrorVisible(false);
              void toggleCompactView();
            }}
          >
            <MaterialIcon name="expand_less" />
          </button>
        </div>

        <section className={`usage-list${stale ? " is-stale" : ""}${usageDeductionEnabled ? "" : " hide-deductions"}`} aria-label={stale ? "Usage remaining, last known data" : "Usage remaining"} style={{ "--usage-row-count": usageRows.length, "--usage-label-width": usageProvider === "antigravity" ? "72px" : "46px" } as CSSProperties} title={connectionError || usage.message}>
          {usageRows.map((row) => (
            <article className="usage-row" key={row.id}>
              <div className="row-main">
                <span className="usage-label" title={[row.label, quotaLabel(row.windowSeconds)].filter(Boolean).join(" · ")}>{row.label || quotaLabel(row.windowSeconds)}</span>
                <div
                  className="usage-track"
                  ref={(element) => {
                    if (element) usageTracks.current.set(row.id, element);
                    else usageTracks.current.delete(row.id);
                  }}
                  role="progressbar"
                  aria-label={`${row.label ? `${row.label} ${quotaLabel(row.windowSeconds)}` : quotaLabel(row.windowSeconds)} usage remaining${stale ? ", last known data" : ""}`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={row.remainingPercent ?? undefined}
                  aria-valuetext={row.remainingPercent === null ? "No data" : undefined}
                >
                  <span style={{ width: `${row.remainingPercent ?? 0}%` }} />
                </div>
              </div>
              <div className="row-value">
                <strong>{row.remainingPercent === null ? "—" : `${Math.round(row.remainingPercent)}%`}</strong>
                {usageDeductionEnabled && (
                  <span className="row-deduction" title="Last usage deduction" aria-hidden={row.remainingPercent === null || usageDeductions[row.id] === undefined} aria-label={usageDeductions[row.id] === undefined ? undefined : `Last usage deduction: ${usageDeductions[row.id].toLocaleString(undefined, { maximumFractionDigits: 2 })} percentage points`}>
                    {row.remainingPercent !== null && usageDeductions[row.id] !== undefined ? `-${usageDeductions[row.id].toLocaleString(undefined, { maximumFractionDigits: 2 })}` : ""}
                  </span>
                )}
                <span title={row.resetsAt === null ? "Reset time unavailable" : new Date(row.resetsAt * 1000).toLocaleString()}>{resetLabel(row, now)}</span>
              </div>
            </article>
          ))}
        </section>

        <footer className="card-footer" id="usage-card-footer" aria-hidden={compact} inert={compact}>
          <span className={`demo-label usage-status${usage.status === "ready" && !stale && !connectionError ? " is-connected" : ""}`} title={[usage.account?.email, connectionError || usage.message].filter(Boolean).join("\n")} role="status"><span aria-hidden="true" />{usage.account?.email || (connectionError ? "UNAVAILABLE" : usageStatusLabel(usage, now).toUpperCase())}</span>
          <span className="footer-hint">{refreshCountdownLabel(usage, now)}</span>
        </footer>
        <p className={`mode-error${modeErrorVisible ? " is-visible" : ""}`} role="status" aria-hidden={!modeErrorVisible}>{modeError}</p>
      </section>
    </main>
  );
}

export default UsagePin;
