export type UsageStatus = "connecting" | "ready" | "missing_session" | "invalid_session" | "unsupported_auth" | "expired_session" | "access_denied" | "rate_limited" | "network_error" | "invalid_response";

export interface UsageWindow {
  id: string;
  remainingPercent: number | null;
  windowSeconds: number | null;
  resetsAt: number | null;
}

export interface UsageSnapshot {
  revision: number;
  status: UsageStatus;
  refreshing: boolean;
  message: string;
  sourcePath: string;
  account: { accountId: string; userId: string | null; email: string | null; plan: string | null } | null;
  windows: UsageWindow[];
  updatedAt: number | null;
  refreshIntervalSeconds: number;
  nextRefreshAtMs: number | null;
}

export const initialUsage: UsageSnapshot = {
  revision: -1, status: "connecting", refreshing: false,
  message: "Checking your existing Codex Windows session…", sourcePath: "",
  account: null, windows: [], updatedAt: null, refreshIntervalSeconds: 60, nextRefreshAtMs: null,
};

export function sameUsageIdentity(previous: UsageSnapshot | null, current: UsageSnapshot) {
  return previous !== null && !!current.account?.userId
    && previous.sourcePath === current.sourcePath
    && previous.account?.accountId === current.account.accountId
    && previous.account?.userId === current.account.userId;
}

export function usageSpikeIds(previous: UsageSnapshot | null, current: UsageSnapshot) {
  if (!previous || previous.status !== "ready" || current.status !== "ready" || current.refreshing
    || current.revision <= previous.revision || !sameUsageIdentity(previous, current)) return [];

  return current.windows.filter((row) => {
    const before = previous.windows.find((window) => window.id === row.id);
    return before !== undefined && before.windowSeconds === row.windowSeconds
      && before.remainingPercent !== null && row.remainingPercent !== null
      && before.remainingPercent - row.remainingPercent > 10;
  }).map((row) => row.id);
}

export function latestUsageDeductions(previous: UsageSnapshot | null, current: UsageSnapshot, latest: Record<string, number>) {
  if (previous && current.revision <= previous.revision) return latest;
  if (!previous || !sameUsageIdentity(previous, current)) return {};
  if (current.status !== "ready" || current.refreshing) return latest;

  const deductions: Record<string, number> = {};
  for (const row of current.windows) {
    const before = previous.windows.find((window) => window.id === row.id);
    if (!before || before.windowSeconds !== row.windowSeconds) continue;
    if (latest[row.id] !== undefined) deductions[row.id] = latest[row.id];
    if (before.remainingPercent !== null && row.remainingPercent !== null
      && before.remainingPercent > row.remainingPercent) {
      deductions[row.id] = before.remainingPercent - row.remainingPercent;
    }
  }
  return deductions;
}

export function isUsageStale(usage: UsageSnapshot, now = Date.now()) {
  return usage.updatedAt !== null && (usage.status !== "ready" || now / 1000 - usage.updatedAt > Math.max(usage.refreshIntervalSeconds * 2, 30));
}

export function usageStatusLabel(usage: UsageSnapshot, now = Date.now()) {
  if (isUsageStale(usage, now)) return "Stale data";
  const labels: Record<UsageStatus, string> = {
    connecting: "Connecting", ready: "Connected", missing_session: "No session",
    invalid_session: "Check session", unsupported_auth: "Unsupported session",
    expired_session: "Session expired", access_denied: "Access denied",
    rate_limited: "Waiting to retry", network_error: "Offline", invalid_response: "Unavailable",
  };
  return labels[usage.status];
}

export function refreshCountdownLabel(usage: UsageSnapshot, now = Date.now()) {
  if (usage.refreshing) return "Refreshing…";
  if (usage.nextRefreshAtMs == null) return "Refresh pending";
  const seconds = Math.max(0, Math.ceil((usage.nextRefreshAtMs - now) / 1000));
  return `Refresh in ${seconds}s`;
}

export function quotaLabel(seconds: number | null) {
  if (seconds === null) return "Quota";
  if (seconds === 604800) return "Weekly";
  if (seconds % 86400 === 0) return `${seconds / 86400}d`;
  if (seconds % 3600 === 0) return `${seconds / 3600}h`;
  if (seconds % 60 === 0) return `${seconds / 60}m`;
  return `${seconds}s`;
}

export function resetLabel(window: UsageWindow, now = Date.now()) {
  if (window.resetsAt === null) return "—";
  if (window.resetsAt * 1000 <= now) return "Checking…";
  const date = new Date(window.resetsAt * 1000);
  if (!Number.isFinite(date.getTime())) return "—";
  return window.windowSeconds !== null && window.windowSeconds >= 86400
    ? date.toLocaleDateString(undefined, { month: "short", day: "numeric" })
    : date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}
