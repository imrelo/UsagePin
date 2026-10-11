import { useEffect, useState } from "react";

const storageKey = "usagepin.usage-shake-enabled";

export function useUsageShakeSetting() {
  const [usageShakeEnabled, setUsageShakeEnabled] = useState(() => window.localStorage.getItem(storageKey) !== "false");

  useEffect(() => {
    window.localStorage.setItem(storageKey, String(usageShakeEnabled));
  }, [usageShakeEnabled]);

  useEffect(() => {
    function syncSetting(event: StorageEvent) {
      if (event.key === storageKey || event.key === null) setUsageShakeEnabled(event.newValue !== "false");
    }
    window.addEventListener("storage", syncSetting);
    return () => window.removeEventListener("storage", syncSetting);
  }, []);

  return { usageShakeEnabled, setUsageShakeEnabled };
}
