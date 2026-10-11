import { useEffect, useState } from "react";

function parseOpacity(saved: string | null) {
  const value = Number(saved);
  return saved !== null && saved.trim() !== "" && Number.isFinite(value) && value >= 0 && value <= 100
    ? value
    : 100;
}

function useStoredOpacity(key: string) {
  const [opacity, setOpacity] = useState(() => parseOpacity(window.localStorage.getItem(key)));

  useEffect(() => {
    window.localStorage.setItem(key, String(opacity));
  }, [key, opacity]);

  useEffect(() => {
    function syncOpacity(event: StorageEvent) {
      if (event.key === key) setOpacity(parseOpacity(event.newValue));
    }
    window.addEventListener("storage", syncOpacity);
    return () => window.removeEventListener("storage", syncOpacity);
  }, [key]);

  return [opacity, setOpacity] as const;
}

export function useAppearance() {
  const [backgroundOpacity, setBackgroundOpacity] = useStoredOpacity("usagepin.background-opacity");
  const [contentOpacity, setContentOpacity] = useStoredOpacity("usagepin.content-opacity");
  const [theme, setStoredTheme] = useState<"dark" | "light">(() => window.localStorage.getItem("usagepin.theme") === "light" ? "light" : "dark");

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    function syncTheme(event: StorageEvent) {
      if (event.key === "usagepin.theme" || event.key === null) setStoredTheme(window.localStorage.getItem("usagepin.theme") === "light" ? "light" : "dark");
    }
    window.addEventListener("storage", syncTheme);
    return () => window.removeEventListener("storage", syncTheme);
  }, []);

  function setTheme(value: "dark" | "light") {
    window.localStorage.setItem("usagepin.theme", value);
    setStoredTheme(value);
  }

  return { backgroundOpacity, setBackgroundOpacity, contentOpacity, setContentOpacity, theme, setTheme };
}
