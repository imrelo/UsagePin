import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { initialUsage } from "./codexUsage";
import type { UsageSnapshot } from "./codexUsage";

export function useCodexUsage() {
  const [usage, setUsage] = useState(initialUsage);
  const [connectionError, setConnectionError] = useState("");
  const [now, setNow] = useState(Date.now);
  const acceptSnapshot = useCallback((snapshot: UsageSnapshot) => {
    setUsage((previous) => snapshot.revision >= previous.revision ? snapshot : previous);
    setConnectionError("");
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const stop = await listen<UsageSnapshot>("codex-usage-updated", (event) => {
          if (!disposed) acceptSnapshot(event.payload);
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
        const snapshot = await invoke<UsageSnapshot>("get_codex_usage");
        if (!disposed) acceptSnapshot(snapshot);
      } catch {
        if (!disposed) setConnectionError("Could not connect to the UsagePin backend. Restart UsagePin and try again.");
      }
    })();
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => { disposed = true; unlisten?.(); window.clearInterval(timer); };
  }, [acceptSnapshot]);

  const refreshUsage = useCallback(async () => {
    setConnectionError("");
    try { acceptSnapshot(await invoke<UsageSnapshot>("refresh_codex_usage")); }
    catch { setConnectionError("Could not check the connection. Restart UsagePin and try again."); }
  }, [acceptSnapshot]);

  return { usage, now, connectionError, refreshUsage };
}
