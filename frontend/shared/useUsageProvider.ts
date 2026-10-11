import { useEffect, useState } from "react";

export type UsageProvider = "codex" | "antigravity";
export const providerNames: Record<UsageProvider, string> = { codex: "Codex", antigravity: "Antigravity" };
const storageKey = "usagepin.usage-provider";

export function readUsageProvider(saved: string | null): UsageProvider {
  return saved === "antigravity" ? "antigravity" : "codex";
}

export function useUsageProvider() {
  const [usageProvider, setUsageProvider] = useState(() => readUsageProvider(window.localStorage.getItem(storageKey)));
  useEffect(() => {
    window.localStorage.setItem(storageKey, usageProvider);
  }, [usageProvider]);
  useEffect(() => {
    function syncProvider(event: StorageEvent) {
      if (event.key === storageKey || event.key === null) setUsageProvider(readUsageProvider(event.newValue));
    }
    window.addEventListener("storage", syncProvider);
    return () => window.removeEventListener("storage", syncProvider);
  }, []);
  return { usageProvider, setUsageProvider };
}
