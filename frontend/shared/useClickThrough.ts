import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useClickThrough() {
  const [clickThrough, setClickThrough] = useState(false);

  useEffect(() => {
    let disposed = false;
    let unlistenMode: (() => void) | undefined;

    async function restoreAndListen() {
      const stopMode = await listen<boolean>(
        "click-through-changed",
        ({ payload }) => setClickThrough(payload),
      );
      if (disposed) stopMode();
      else unlistenMode = stopMode;

      setClickThrough(await invoke<boolean>("get_click_through"));
    }

    void restoreAndListen();
    return () => {
      disposed = true;
      unlistenMode?.();
    };
  }, []);

  async function toggleClickThrough() {
    setClickThrough(await invoke<boolean>("toggle_click_through"));
  }

<<<<<<< HEAD
  async function resetClickThrough() {
    await invoke("reset_click_through");
    setClickThrough(false);
  }

  return { clickThrough, toggleClickThrough, resetClickThrough };
=======
  return { clickThrough, toggleClickThrough };
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
}
