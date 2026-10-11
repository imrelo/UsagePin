import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppearance } from "../shared/useAppearance";
import { useClickThrough } from "../shared/useClickThrough";
import { useRefreshInterval } from "../shared/useRefreshInterval";
import { useUsageShakeSetting } from "../shared/useUsageShakeSetting";
import { useUsageDeductionSetting } from "../shared/useUsageDeductionSetting";

export function useSettings() {
  const { backgroundOpacity, setBackgroundOpacity, contentOpacity, setContentOpacity, theme, setTheme } = useAppearance();
  const { clickThrough, toggleClickThrough: toggleMode } = useClickThrough();
  const { refreshInterval, setRefreshInterval, refreshIntervalError } = useRefreshInterval();
  const { usageShakeEnabled, setUsageShakeEnabled } = useUsageShakeSetting();
  const { usageDeductionEnabled, setUsageDeductionEnabled } = useUsageDeductionSetting();
  const [refreshIntervalInput, setRefreshIntervalInput] = useState(String(refreshInterval));
  const [settingsClosing, setSettingsClosing] = useState(false);
  const [settingsOpening, setSettingsOpening] = useState(0);
  const [modeError, setModeError] = useState("");
  const [modeErrorVisible, setModeErrorVisible] = useState(false);

  useEffect(() => {
    setRefreshIntervalInput(String(refreshInterval));
  }, [refreshInterval]);

  function changeRefreshInterval(value: string) {
    setRefreshIntervalInput(value);
    const seconds = Number(value);
    if (Number.isSafeInteger(seconds) && seconds >= 1) setRefreshInterval(seconds);
  }

  useEffect(() => {
    if (!settingsClosing) return;
    const duration = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240;
    const timer = window.setTimeout(() => {
      void getCurrentWindow().hide();
    }, duration);
    return () => window.clearTimeout(timer);
  }, [settingsClosing]);

  useEffect(() => {
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setSettingsClosing(true);
    }
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlistenSettingsOpen: (() => void) | undefined;
    let unlistenSettingsClose: (() => void) | undefined;

    async function listenForSettings() {
      const stopOpen = await listen("settings-opened", () => {
        setSettingsClosing(false);
        setSettingsOpening((value) => value + 1);
      });
      if (disposed) stopOpen();
      else unlistenSettingsOpen = stopOpen;

      const stopClose = await listen("settings-close-requested", () => {
        setSettingsClosing(true);
      });
      if (disposed) stopClose();
      else unlistenSettingsClose = stopClose;
    }

    void listenForSettings();
    return () => {
      disposed = true;
      unlistenSettingsOpen?.();
      unlistenSettingsClose?.();
    };
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

  return {
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
    resetRefreshIntervalInput: () => setRefreshIntervalInput(String(refreshInterval)),
    clickThrough,
    modeError,
    modeErrorVisible,
    settingsClosing,
    settingsOpening,
    closeSettings: () => setSettingsClosing(true),
    toggleClickThrough,
  };
}
