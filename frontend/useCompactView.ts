import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

export async function animateCompactView(compact: boolean, onProgress: (progress: number) => void) {
  const appWindow = getCurrentWindow();
  const originalSize = await appWindow.innerSize();
  const startProgress = compact ? 1 : 0;
  const duration = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240;
  await invoke("set_compact_view", { compact });

  try {
    const started = performance.now();
    let elapsed = 0;
    let lastHeight = originalSize.height;
    do {
      const timestamp = duration === 0 ? started : await new Promise<number>(requestAnimationFrame);
      elapsed = duration === 0 ? 1 : Math.min(1, (timestamp - started) / duration);
      const eased = 1 - Math.pow(1 - elapsed, 3);
      const progress = compact ? 1 - eased : eased;
      const height = Math.max(1, Math.round(originalSize.width * (98 + 42 * progress) / 320));
      if (height !== lastHeight) {
        await appWindow.setSize(new PhysicalSize(originalSize.width, height));
        lastHeight = height;
      }
      onProgress(progress);
    } while (elapsed < 1);
  } catch (error) {
    await invoke("set_compact_view", { compact: !compact }).catch(() => {});
    await appWindow.setSize(originalSize).catch(() => {});
    onProgress(startProgress);
    throw error;
  }
}

export function useCompactView(onError: (message: string) => void) {
  const [compact, setCompact] = useState(false);
  const [chromeProgress, setChromeProgress] = useState(1);
  const [resizing, setResizing] = useState(false);
  const resizingRef = useRef(false);

  async function toggleCompactView() {
    if (resizingRef.current) return;
    resizingRef.current = true;
    setResizing(true);
    setCompact(!compact);
    try {
      await animateCompactView(!compact, setChromeProgress);
    } catch {
      setCompact(compact);
      onError("Could not resize UsagePin. Try again.");
    } finally {
      resizingRef.current = false;
      setResizing(false);
    }
  }

  return { compact, chromeProgress, resizing, toggleCompactView };
}
