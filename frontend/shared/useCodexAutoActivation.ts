import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface AutoActivationSnapshot {
  revision: number;
  enabled: boolean;
  message: string;
  lastAttemptAt: number | null;
}

export function useCodexAutoActivation() {
  const [activation, setActivation] = useState<AutoActivationSnapshot>({
    revision: -1, enabled: true, message: "Waiting for the next 5-hour quota reset.", lastAttemptAt: null,
  });
  const [activationPending, setPending] = useState(true);
  const [activationError, setError] = useState("");

  function accept(snapshot: AutoActivationSnapshot) {
    setActivation((previous) => snapshot.revision >= previous.revision ? snapshot : previous);
  }

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const stop = await listen<AutoActivationSnapshot>("codex-auto-activation-updated", (event) => {
          if (!disposed) accept(event.payload);
        });
        if (disposed) { stop(); return; }
        unlisten = stop;
        const snapshot = await invoke<AutoActivationSnapshot>("get_codex_auto_activation");
        if (!disposed) accept(snapshot);
      } catch {
        if (!disposed) setError("Could not read automatic activation settings. Try changing the setting again.");
      } finally {
        if (!disposed) setPending(false);
      }
    })();
    return () => { disposed = true; unlisten?.(); };
  }, []);

  async function setAutoActivation(enabled: boolean) {
    setPending(true);
    setError("");
    try {
      accept(await invoke<AutoActivationSnapshot>("set_codex_auto_activation", { enabled }));
    } catch {
      setError("Could not save automatic activation settings. Please try again.");
      throw new Error("Could not save automatic activation settings.");
    } finally {
      setPending(false);
    }
  }

  return { autoActivation: activation, setAutoActivation, activationPending, activationError };
}
