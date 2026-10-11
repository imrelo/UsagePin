import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const storageKey = "usagepin.refresh-interval-seconds";

function readRefreshInterval(saved: string | null) {
  const seconds = Number(saved);
  return Number.isSafeInteger(seconds) && seconds >= 1 ? seconds : 60;
}

export function useRefreshInterval() {
  const [refreshInterval, setRefreshInterval] = useState(() => readRefreshInterval(window.localStorage.getItem(storageKey)));
  const [refreshIntervalError, setRefreshIntervalError] = useState("");

  useEffect(() => {
    window.localStorage.setItem(storageKey, String(refreshInterval));
    let disposed = false;
    void invoke("set_usage_refresh_interval", { seconds: refreshInterval }).then(() => {
      if (!disposed) setRefreshIntervalError("");
    }).catch(() => {
      if (!disposed) setRefreshIntervalError("The interval was saved, but could not be applied. Restart UsagePin to retry.");
    });
    return () => { disposed = true; };
  }, [refreshInterval]);

  useEffect(() => {
    function syncRefreshInterval(event: StorageEvent) {
      if (event.key === storageKey || event.key === null) {
        setRefreshInterval(readRefreshInterval(event.newValue));
      }
    }
    window.addEventListener("storage", syncRefreshInterval);
    return () => window.removeEventListener("storage", syncRefreshInterval);
  }, []);

  return { refreshInterval, setRefreshInterval, refreshIntervalError };
}
