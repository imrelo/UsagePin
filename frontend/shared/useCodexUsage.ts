import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { initialUsage } from "./codexUsage";
import type { UsageSnapshot } from "./codexUsage";
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

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const stop = await listen<UsageSnapshot>(`${provider}-usage-updated`, (event) => {
          if (!disposed) acceptSnapshot(event.payload);
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
        const snapshot = await invoke<UsageSnapshot>(`get_${provider}_usage`);
        if (!disposed) acceptSnapshot(snapshot);
      } catch {
        if (!disposed) setConnectionError("Could not connect to the UsagePin backend. Restart UsagePin and try again.");
      }
    })();
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => { disposed = true; unlisten?.(); window.clearInterval(timer); };
  }, [acceptSnapshot, provider, setConnectionError]);

  const refreshUsage = useCallback(async () => {
    setConnectionError("");
    try { acceptSnapshot(await invoke<UsageSnapshot>(`refresh_${provider}_usage`)); }
    catch { setConnectionError("Could not check the connection. Restart UsagePin and try again."); }
  }, [acceptSnapshot, provider, setConnectionError]);

  return { usage: usageByProvider[provider], now, connectionError: errors[provider], refreshUsage };
}
