const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("typescript");

const source = ts.transpileModule(fs.readFileSync(path.join(__dirname, "codexUsage.ts"), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
const context = { exports: {} };
vm.runInNewContext(source, context);
const { initialUsage, quotaLabel, isUsageStale, usageStatusLabel, resetLabel, refreshCountdownLabel, usageSpikeIds, latestUsageDeductions } = context.exports;

test("refresh countdown uses the scheduled deadline, rounds up and never goes negative", () => {
  const usage = { ...initialUsage, nextRefreshAtMs: 1060000 };
  assert.equal(refreshCountdownLabel(usage, 1000000), "Refresh in 60s");
  assert.equal(refreshCountdownLabel(usage, 1001000), "Refresh in 59s");
  assert.equal(refreshCountdownLabel(usage, 1059500), "Refresh in 1s");
  assert.equal(refreshCountdownLabel(usage, 1060000), "Refresh in 0s");
  assert.equal(refreshCountdownLabel(usage, 1070000), "Refresh in 0s");
  assert.equal(refreshCountdownLabel({ ...usage, nextRefreshAtMs: 1120000 }, 1000000), "Refresh in 120s");
});

test("refresh countdown distinguishes an active request from an unknown schedule", () => {
  assert.equal(refreshCountdownLabel(initialUsage), "Refresh pending");
  assert.equal(refreshCountdownLabel({ ...initialUsage, refreshing: true }), "Refreshing…");
  assert.equal(refreshCountdownLabel({ ...initialUsage, refreshing: true, nextRefreshAtMs: 1060000 }, 1000000), "Refreshing…");
});

test("quota labels follow real duration including a weekly primary and missing duration", () => {
  assert.equal(quotaLabel(604800), "Weekly");
  assert.equal(quotaLabel(18000), "5h");
  assert.equal(quotaLabel(900), "15m");
  assert.equal(quotaLabel(null), "Quota");
});

test("network failures and overdue successful snapshots mark old data as stale", () => {
  const usage = { ...initialUsage, status: "ready", updatedAt: 1000, refreshIntervalSeconds: 60 };
  assert.equal(isUsageStale(usage, 1100000), false);
  assert.equal(usageStatusLabel(usage, 1121000), "Stale data");
  assert.equal(isUsageStale({ ...usage, status: "network_error" }, 1001000), true);
  assert.equal(usageStatusLabel({ ...initialUsage, status: "missing_session" }), "No session");
});

test("expired reset never implies restored quota and missing reset stays unknown", () => {
  assert.equal(resetLabel({ resetsAt: null }, 1000000), "—");
  assert.equal(resetLabel({ resetsAt: 1000, windowSeconds: 18000 }, 1000000), "Checking…");
});

function snapshot(revision, remaining, overrides = {}) {
  return {
    ...initialUsage, status: "ready", revision, updatedAt: 1000 + revision,
    account: { accountId: "team", userId: "user-a", email: null, plan: null },
    windows: [{ id: "primary", remainingPercent: remaining, windowSeconds: 18000, resetsAt: 2000 }],
    ...overrides,
  };
}

test("usage shake triggers only for a drop greater than 10 percentage points per update", () => {
  const before = snapshot(1, 80);
  assert.deepEqual(Array.from(usageSpikeIds(before, snapshot(2, 69))), ["primary"]);
  for (const remaining of [80, 75, 70, 90, null]) {
    assert.deepEqual(Array.from(usageSpikeIds(before, snapshot(2, remaining))), []);
  }
  assert.deepEqual(Array.from(usageSpikeIds(snapshot(1, null), snapshot(2, 69))), []);
  assert.deepEqual(Array.from(usageSpikeIds(snapshot(2, 75), snapshot(3, 69))), []);
});

test("initial load, refresh notifications, errors and repeated snapshots do not shake", () => {
  const before = snapshot(1, 80);
  const after = snapshot(2, 60);
  assert.deepEqual(Array.from(usageSpikeIds(null, after)), []);
  assert.deepEqual(Array.from(usageSpikeIds(before, { ...after, refreshing: true })), []);
  assert.deepEqual(Array.from(usageSpikeIds(before, { ...after, status: "network_error" })), []);
  assert.deepEqual(Array.from(usageSpikeIds(after, after)), []);
  assert.deepEqual(Array.from(usageSpikeIds(after, before)), []);
});

test("only affected quota rows shake, and different accounts or quota durations do not compare", () => {
  const before = snapshot(1, 80);
  const after = snapshot(2, 60);
  before.windows.push({ id: "secondary", remainingPercent: 90, windowSeconds: 604800, resetsAt: 3000 });
  after.windows.push({ id: "secondary", remainingPercent: 85, windowSeconds: 604800, resetsAt: 3000 });
  assert.deepEqual(Array.from(usageSpikeIds(before, after)), ["primary"]);
  assert.deepEqual(Array.from(usageSpikeIds(before, { ...after, account: { ...after.account, accountId: "other" } })), []);
  assert.deepEqual(Array.from(usageSpikeIds(before, { ...after, sourcePath: "other-session" })), []);
  assert.deepEqual(Array.from(usageSpikeIds(before, snapshot(2, 60, {
    windows: [{ ...after.windows[0], windowSeconds: 604800 }],
  }))), []);
});

test("last deduction includes small drops and stays until the next drop", () => {
  const first = snapshot(1, 80);
  const second = snapshot(2, 75);
  let latest = latestUsageDeductions(first, second, {});
  assert.equal(latest.primary, 5);
  const unchanged = snapshot(3, 75);
  latest = latestUsageDeductions(second, unchanged, latest);
  assert.equal(latest.primary, 5);
  const restored = snapshot(4, 100);
  latest = latestUsageDeductions(unchanged, restored, latest);
  assert.equal(latest.primary, 5);
  latest = latestUsageDeductions(restored, snapshot(5, 97.5), latest);
  assert.equal(latest.primary, 2.5);
});

test("last deductions are independent for each quota row", () => {
  const before = snapshot(1, 80);
  const after = snapshot(2, 75);
  before.windows.push({ id: "secondary", remainingPercent: 90, windowSeconds: 604800, resetsAt: 3000 });
  after.windows.push({ id: "secondary", remainingPercent: 78, windowSeconds: 604800, resetsAt: 3000 });
  const latest = latestUsageDeductions(before, after, {});
  assert.equal(latest.primary, 5);
  assert.equal(latest.secondary, 12);
});

test("last deductions ignore incomplete updates and clear for different accounts or quotas", () => {
  const before = snapshot(1, 80);
  const after = snapshot(2, 60);
  const latest = { primary: 5 };
  assert.deepEqual({ ...latestUsageDeductions(null, after, {}) }, {});
  assert.equal(latestUsageDeductions(before, { ...after, refreshing: true }, latest), latest);
  assert.equal(latestUsageDeductions(before, { ...after, status: "network_error" }, latest), latest);
  assert.equal(latestUsageDeductions(after, after, latest), latest);
  assert.equal(latestUsageDeductions(before, snapshot(2, null), latest).primary, 5);
  assert.deepEqual({ ...latestUsageDeductions(before, { ...after, account: { ...after.account, accountId: "other" } }, latest) }, {});
  assert.deepEqual({ ...latestUsageDeductions(before, { ...after, sourcePath: "other-session" }, latest) }, {});
  assert.deepEqual({ ...latestUsageDeductions(before, snapshot(2, 60, {
    windows: [{ ...after.windows[0], windowSeconds: 604800 }],
  }), latest) }, {});
});

test("different users in the same workspace never compare quotas, even without email", () => {
  const before = snapshot(1, 90);
  const after = snapshot(2, 20, { account: { ...before.account, userId: "user-b" } });
  assert.deepEqual(Array.from(usageSpikeIds(before, after)), []);
  assert.deepEqual({ ...latestUsageDeductions(before, after, { primary: 5 }) }, {});
  assert.deepEqual({ ...latestUsageDeductions(before, { ...after, refreshing: true }, { primary: 5 }) }, {});
  assert.equal(latestUsageDeductions(after, snapshot(3, 17, { account: after.account }), {}).primary, 3);
});

test("the same user keeps quota history when email changes or is missing", () => {
  const before = snapshot(1, 90, { account: { accountId: "team", userId: "user-a", email: "old@example.test", plan: null } });
  const after = snapshot(2, 70);
  assert.deepEqual(Array.from(usageSpikeIds(before, after)), ["primary"]);
  assert.equal(latestUsageDeductions(before, after, {}).primary, 20);
});

test("missing user identity never produces deductions or shakes", () => {
  for (const userId of [null, undefined, ""]) {
    const account = { accountId: "team", userId, email: "same@example.test", plan: null };
    const before = snapshot(1, 90, { account });
    const after = snapshot(2, 20, { account });
    assert.deepEqual(Array.from(usageSpikeIds(before, after)), []);
    assert.deepEqual({ ...latestUsageDeductions(before, after, { primary: 5 }) }, {});
  }
});
