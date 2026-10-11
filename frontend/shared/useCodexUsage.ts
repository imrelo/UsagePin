import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { initialUsage } from "./codexUsage";
import type { UsageSnapshot } from "./codexUsage";
<<<<<<< HEAD
import type { UsageProvider } from "./useUsageProvider";

export function useCodexUsage(provider: UsageProvider = "codex") {
  const [usageByProvider, setUsage] = useState<Record<UsageProvider, UsageSnapshot>>({
    codex: initialUsage,
    antigravity: { ...initialUsage, message: "Checking the running Antigravity app…" },
  });
  const [errors, setErrors] = useState<Record<UsageProvider, string>>({ codex: "", antigravity: "" });
  const setConnectionError = useCallback((error: string) => {
    setErrors((previous) => ({ ...previous, [provider]: error }));
  }, [provider]);
  const [now, setNow] = useState(Date.now);
  const acceptSnapshot = useCallback((snapshot: UsageSnapshot) => {
    setUsage((previous) => snapshot.revision >= previous[provider].revision ? { ...previous, [provider]: snapshot } : previous);
    setConnectionError("");
  }, [provider, setConnectionError]);
=======

export function useCodexUsage() {
  const [usage, setUsage] = useState(initialUsage);
  const [connectionError, setConnectionError] = useState("");
  const [now, setNow] = useState(Date.now);
  const acceptSnapshot = useCallback((snapshot: UsageSnapshot) => {
    setUsage((previous) => snapshot.revision >= previous.revision ? snapshot : previous);
    setConnectionError("");
  }, []);
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
<<<<<<< HEAD
        const stop = await listen<UsageSnapshot>(`${provider}-usage-updated`, (event) => {
=======
        const stop = await listen<UsageSnapshot>("codex-usage-updated", (event) => {
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
          if (!disposed) acceptSnapshot(event.payload);
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
<<<<<<< HEAD
        const snapshot = await invoke<UsageSnapshot>(`get_${provider}_usage`);
=======
        const snapshot = await invoke<UsageSnapshot>("get_codex_usage");
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
        if (!disposed) acceptSnapshot(snapshot);
      } catch {
        if (!disposed) setConnectionError("Could not connect to the UsagePin backend. Restart UsagePin and try again.");
      }
    })();
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => { disposed = true; unlisten?.(); window.clearInterval(timer); };
<<<<<<< HEAD
  }, [acceptSnapshot, provider, setConnectionError]);

  const refreshUsage = useCallback(async () => {
    setConnectionError("");
    try { acceptSnapshot(await invoke<UsageSnapshot>(`refresh_${provider}_usage`)); }
    catch { setConnectionError("Could not check the connection. Restart UsagePin and try again."); }
  }, [acceptSnapshot, provider, setConnectionError]);

  return { usage: usageByProvider[provider], now, connectionError: errors[provider], refreshUsage };
=======
  }, [acceptSnapshot]);

  const refreshUsage = useCallback(async () => {
    setConnectionError("");
    try { acceptSnapshot(await invoke<UsageSnapshot>("refresh_codex_usage")); }
    catch { setConnectionError("Could not check the connection. Restart UsagePin and try again."); }
  }, [acceptSnapshot]);

  return { usage, now, connectionError, refreshUsage };
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
}
