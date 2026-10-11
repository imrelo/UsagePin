const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("typescript");

function loadModule(filename, modules, globals = {}) {
  const source = ts.transpileModule(fs.readFileSync(path.join(__dirname, filename), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX },
  }).outputText;
  const context = { exports: {}, require(name) {
    if (!(name in modules)) throw new Error(`Unexpected import: ${name}`);
    return modules[name];
  }, ...globals };
  vm.runInNewContext(source, context, { filename });
  return context.exports;
}

const usageModule = loadModule("shared/codexUsage.ts", {});

function snapshot(revision, remaining, userId = "user-a", overrides = {}) {
  return {
    ...usageModule.initialUsage, status: "ready", revision, updatedAt: 1000,
    account: { accountId: "team", userId, email: null, plan: null },
    windows: [{ id: "primary", remainingPercent: remaining, windowSeconds: 18000, resetsAt: 2000 }],
    ...overrides,
  };
}

function widget() {
  const hooks = [];
  const tracks = new Map();
<<<<<<< HEAD
  let cursor, effects, dirty, currentUsage, tree, provider = "codex";
=======
  let cursor, effects, dirty, currentUsage, tree;
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
  const react = {
    useRef(initial) { return hooks[cursor++] ??= { current: initial }; },
    useState(initial) {
      const state = hooks[cursor++] ??= { value: typeof initial === "function" ? initial() : initial };
      return [state.value, (update) => {
        const next = typeof update === "function" ? update(state.value) : update;
        if (!Object.is(next, state.value)) { state.value = next; dirty = true; }
      }];
    },
    useEffect(callback, dependencies) {
      const index = cursor++;
      if (!hooks[index] || dependencies.some((value, i) => !Object.is(value, hooks[index][i]))) effects.push(callback);
      hooks[index] = dependencies;
    },
  };
  const jsx = (type, props, key) => ({ type, props, key });
  const component = loadModule("UsagePin.tsx", {
    react,
    "react/jsx-runtime": { jsx, jsxs: jsx },
    "@tauri-apps/api/core": {},
    "@tauri-apps/api/window": {},
    "./shared/useAppearance": { useAppearance: () => ({ backgroundOpacity: 100, contentOpacity: 100 }) },
    "./shared/useClickThrough": { useClickThrough: () => ({ clickThrough: false }) },
    "./shared/MaterialIcon": { MaterialIcon: () => null },
    "./useCompactView": { useCompactView: () => ({ compact: false, chromeProgress: 1, resizing: false }) },
    "./shared/useCodexUsage": { useCodexUsage: () => ({ usage: currentUsage, now: 1000000, connectionError: "" }) },
<<<<<<< HEAD
    "./shared/useUsageProvider": { useUsageProvider: () => ({ usageProvider: provider }), providerNames: { codex: "Codex", antigravity: "Antigravity" } },
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
    "./shared/useRefreshInterval": { useRefreshInterval() {} },
    "./shared/useUsageShakeSetting": { useUsageShakeSetting: () => ({ usageShakeEnabled: true }) },
    "./shared/useUsageDeductionSetting": { useUsageDeductionSetting: () => ({ usageDeductionEnabled: true }) },
    "./shared/codexUsage": usageModule,
    "./App.css": {},
  }, { window: { innerWidth: 320, matchMedia: () => ({ matches: false }), addEventListener() {}, removeEventListener() {} } }).default;

  function walk(node, visit, rowId) {
    if (Array.isArray(node)) { node.forEach((child) => walk(child, visit, rowId)); return; }
    if (!node || typeof node !== "object") return;
    const id = node.type === "article" ? node.key : rowId;
    visit(node, id);
    walk(node.props?.children, visit, id);
  }

  return {
<<<<<<< HEAD
    selectProvider(value) { provider = value; },
    inspect() {
      const labels = [], captions = [];
      walk(tree, (node) => {
        if (node.props?.className === "usage-label") labels.push(node.props.children);
        if (node.props?.className === "brand-caption") captions.push([node.props.children].flat().join(""));
      });
      return { labels, captions };
    },
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
    render(usage) {
      currentUsage = usage;
      let renders = 0;
      do {
        assert.ok(++renders < 10, "widget did not settle");
        cursor = 0; effects = []; dirty = false;
        tree = component();
        walk(tree, (node, id) => {
          if (typeof node.props?.ref !== "function") return;
          if (!tracks.has(id)) {
            const animations = [];
            tracks.set(id, {
              getAnimations: () => animations.filter((animation) => !animation.cancelled),
              animate() {
                const animation = { cancelled: false, cancel() { this.cancelled = true; } };
                animations.push(animation);
                return animation;
              },
            });
          }
          node.props.ref(tracks.get(id));
        });
        effects.forEach((effect) => effect());
      } while (dirty);
      const deductions = [];
      walk(tree, (node) => {
        if (node.props?.className === "row-deduction" && !node.props["aria-hidden"]) deductions.push(node.props.children);
      });
      return { deductions, animations: [...tracks.values()].flatMap((track) => track.getAnimations()).length };
    },
  };
}

test("widget clears deductions and cancels an active shake when the user changes", () => {
  const view = widget();
  view.render(snapshot(1, 90));
  assert.deepEqual(view.render(snapshot(2, 70)), { deductions: ["-20"], animations: 1 });
  assert.deepEqual(view.render(snapshot(3, 20, "user-b")), { deductions: [], animations: 0 });
  assert.deepEqual(view.render(snapshot(4, 17, "user-b")), { deductions: ["-3"], animations: 0 });
});

<<<<<<< HEAD
test("widget displays only the chosen provider and distinct Antigravity groups", () => {
  const view = widget();
  view.render(snapshot(1, 90));
  assert.deepEqual(view.inspect(), { labels: ["5h"], captions: ["CODEX USAGE"] });
  view.selectProvider("antigravity");
  view.render(snapshot(1, 100, "google-user", { sourcePath: "antigravity", windows: [
    { id: "gemini-weekly", label: "Gemini", remainingPercent: 100, windowSeconds: 604800, resetsAt: 2000 },
    { id: "3p-weekly", label: "Claude/GPT", remainingPercent: 25, windowSeconds: 604800, resetsAt: 2000 },
  ] }));
  assert.deepEqual(view.inspect(), { labels: ["Gemini", "Claude/GPT"], captions: ["ANTIGRAVITY USAGE"] });
  view.render(snapshot(2, null, null, { status: "app_not_running", account: null, windows: [] }));
  assert.deepEqual(view.inspect().labels, ["Gemini", "Claude/GPT"]);
});

=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
test("missing-session followed by ready starts new history for the same or a different user", () => {
  for (const userId of ["user-a", "user-b"]) {
    const view = widget();
    view.render(snapshot(1, 90));
    view.render(snapshot(2, 70));
    assert.deepEqual(view.render(snapshot(3, null, "user-a", {
      status: "missing_session", account: null, windows: [],
    })), { deductions: [], animations: 0 });
    assert.deepEqual(view.render(snapshot(4, 20, userId)), { deductions: [], animations: 0 });
    assert.deepEqual(view.render(snapshot(5, 17, userId)), { deductions: ["-3"], animations: 0 });
  }
});

test("refresh notifications and network errors preserve the same user's history", () => {
  const view = widget();
  view.render(snapshot(1, 90));
  assert.deepEqual(view.render(snapshot(2, 85)), { deductions: ["-5"], animations: 0 });
  view.render(snapshot(3, 85, "user-a", { refreshing: true }));
  view.render(snapshot(4, 85, "user-a", { status: "network_error" }));
  assert.deepEqual(view.render(snapshot(5, 82)), { deductions: ["-3"], animations: 0 });
});
