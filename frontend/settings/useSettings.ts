import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppearance } from "../shared/useAppearance";
import { useClickThrough } from "../shared/useClickThrough";
import { useRefreshInterval } from "../shared/useRefreshInterval";
import { useUsageShakeSetting } from "../shared/useUsageShakeSetting";
import { useUsageDeductionSetting } from "../shared/useUsageDeductionSetting";
import { useUsageProvider } from "../shared/useUsageProvider";
import { useStartWithWindows } from "../shared/useStartWithWindows";
import { useCodexAutoActivation } from "../shared/useCodexAutoActivation";

export function useSettings() {
  const { backgroundOpacity, setBackgroundOpacity, contentOpacity, setContentOpacity, theme, setTheme } = useAppearance();
  const { clickThrough, toggleClickThrough: toggleMode, resetClickThrough } = useClickThrough();
  const { refreshInterval, setRefreshInterval, refreshIntervalError, resetRefreshInterval } = useRefreshInterval();
  const { usageShakeEnabled, setUsageShakeEnabled } = useUsageShakeSetting();
  const { usageDeductionEnabled, setUsageDeductionEnabled } = useUsageDeductionSetting();
  const { usageProvider, setUsageProvider } = useUsageProvider();
  const { startWithWindows, setStartWithWindows, startupPending, startupError } = useStartWithWindows();
  const { autoActivation, setAutoActivation, activationPending, activationError } = useCodexAutoActivation();
  const [refreshIntervalInput, setRefreshIntervalInput] = useState(String(refreshInterval));
  const [settingsClosing, setSettingsClosing] = useState(false);
  const [settingsOpening, setSettingsOpening] = useState(0);
  const [modeError, setModeError] = useState("");
  const [modeErrorVisible, setModeErrorVisible] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [resetMessage, setResetMessage] = useState("");
  const [resetError, setResetError] = useState("");

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

  async function resetSettings() {
    setResetting(true);
    setResetMessage("");
    setResetError("");
    setModeErrorVisible(false);
    try {
      setTheme("dark");
      setBackgroundOpacity(100);
      setContentOpacity(100);
      setUsageShakeEnabled(true);
      setUsageDeductionEnabled(true);
      setUsageProvider("codex");
      const [, seconds] = await Promise.all([resetClickThrough(), resetRefreshInterval(), setStartWithWindows(true), setAutoActivation(true)]);
      setRefreshIntervalInput(String(seconds));
      setResetMessage("Settings reset to defaults.");
    } catch {
      setResetError("Some settings could not be reset. Try again.");
    } finally {
      setResetting(false);
    }
  }

  return {
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
    resetRefreshIntervalInput: () => setRefreshIntervalInput(String(refreshInterval)),
    clickThrough,
    modeError,
    modeErrorVisible,
    settingsClosing,
    settingsOpening,
    closeSettings: () => setSettingsClosing(true),
    toggleClickThrough,
    resetting,
    resetMessage,
    resetError,
    resetSettings,
  };
}
