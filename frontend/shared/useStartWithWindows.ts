import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useStartWithWindows() {
  const [startWithWindows, setEnabled] = useState(true);
  const [startupPending, setPending] = useState(true);
  const [startupError, setError] = useState("");

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    async function load() {
      try {
        const stop = await listen<boolean>("start-with-windows-changed", (event) => {
          if (!disposed) { setEnabled(event.payload); setError(""); }
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
        const enabled = await invoke<boolean>("get_start_with_windows");
        if (!disposed) setEnabled(enabled);
      } catch {
        if (!disposed) setError("Could not read Windows startup settings. Try changing the setting again.");
      } finally {
        if (!disposed) setPending(false);
      }
    }
    void load();
    return () => { disposed = true; unlisten?.(); };
  }, []);

  async function setStartWithWindows(enabled: boolean) {
    setPending(true);
    setError("");
    try {
      setEnabled(await invoke<boolean>("set_start_with_windows", { enabled }));
    } catch {
      setError("Could not change Windows startup settings. Please try again.");
      throw new Error("Could not change Windows startup settings.");
    } finally {
      setPending(false);
    }
  }

  return { startWithWindows, setStartWithWindows, startupPending, startupError };
}
