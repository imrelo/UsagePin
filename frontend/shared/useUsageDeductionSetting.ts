import { useEffect, useState } from "react";

const storageKey = "usagepin.usage-deduction-enabled";

export function useUsageDeductionSetting() {
  const [usageDeductionEnabled, setUsageDeductionEnabled] = useState(() => window.localStorage.getItem(storageKey) !== "false");

  useEffect(() => {
    window.localStorage.setItem(storageKey, String(usageDeductionEnabled));
  }, [usageDeductionEnabled]);

  useEffect(() => {
    function syncSetting(event: StorageEvent) {
      if (event.key === storageKey || event.key === null) setUsageDeductionEnabled(event.newValue !== "false");
    }
    window.addEventListener("storage", syncSetting);
    return () => window.removeEventListener("storage", syncSetting);
  }, []);

  return { usageDeductionEnabled, setUsageDeductionEnabled };
}
