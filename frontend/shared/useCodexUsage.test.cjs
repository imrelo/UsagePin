const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const path = require("node:path");
const { test } = require("node:test");
const ts = require("typescript");

function load(name, modules, globals = {}) {
  const source = ts.transpileModule(fs.readFileSync(path.join(__dirname, name), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  const context = { exports: {}, require: (name) => modules[name], ...globals };
  vm.runInNewContext(source, context);
  return context.exports;
}
const usageModule = load("codexUsage.ts", {});
function ready(revision, id) {
  return { ...usageModule.initialUsage, revision, status: "ready", windows: [{ id }], updatedAt: 1000 };
}

function view() {
  const hooks = [], listeners = new Map(), pending = new Map(), calls = [];
  let cursor, effects, currentProvider = "codex", value;
  const same = (a, b) => a && b.every((item, index) => Object.is(item, a[index]));
  const react = {
    useState(initial) {
      const state = hooks[cursor++] ??= { value: typeof initial === "function" ? initial() : initial };
      return [state.value, (update) => { state.value = typeof update === "function" ? update(state.value) : update; }];
    },
    useCallback(callback, dependencies) {
      const index = cursor++;
      if (!same(hooks[index]?.dependencies, dependencies)) hooks[index] = { callback, dependencies };
      return hooks[index].callback;
    },
    useEffect(callback, dependencies) {
      const index = cursor++;
      const previous = hooks[index];
      if (!same(previous?.dependencies, dependencies)) {
        effects.push(() => { previous?.cleanup?.(); hooks[index].cleanup = callback(); });
        hooks[index] = { dependencies };
      }
    },
  };
  const { useCodexUsage } = load("useCodexUsage.ts", {
    react, "./codexUsage": usageModule,
    "@tauri-apps/api/core": { invoke(command) {
      calls.push(command);
      return new Promise((resolve, reject) => pending.set(command, { resolve, reject }));
    } },
    "@tauri-apps/api/event": { async listen(event, callback) {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event).add(callback);
      return () => listeners.get(event).delete(callback);
    } },
  }, { window: { setInterval() { return 1; }, clearInterval() {} } });
  function render(provider = currentProvider) {
    currentProvider = provider; cursor = 0; effects = [];
    value = useCodexUsage(provider);
    effects.forEach((effect) => effect());
    return value;
  }
  return {
    calls, render,
    async settle() { await new Promise(setImmediate); return render(); },
    resolve(command, snapshot) { assert.ok(pending.has(command), command); pending.get(command).resolve(snapshot); },
    emit(event, snapshot) { for (const listener of listeners.get(event) ?? []) listener({ payload: snapshot }); },
    listenerCount(event) { return listeners.get(event)?.size ?? 0; },
  };
}

test("switching providers isolates revisions and ignores late requests from the old provider", async () => {
  const hook = view();
  hook.render(); await hook.settle();
  hook.resolve("get_codex_usage", ready(100, "codex-weekly"));
  const codex = await hook.settle();
  assert.equal(codex.usage.windows[0].id, "codex-weekly");
  const oldRefresh = codex.refreshUsage();
  const changing = hook.render("antigravity");
  assert.equal(changing.usage.windows.length, 0);
  await hook.settle();
  assert.equal(hook.listenerCount("codex-usage-updated"), 0);
  assert.equal(hook.listenerCount("antigravity-usage-updated"), 1);
  hook.resolve("get_antigravity_usage", ready(1, "gemini-weekly"));
  assert.equal((await hook.settle()).usage.windows[0].id, "gemini-weekly");
  hook.resolve("refresh_codex_usage", ready(101, "old-codex-response"));
  await oldRefresh;
  assert.equal(hook.render().usage.windows[0].id, "gemini-weekly");
  hook.emit("antigravity-usage-updated", ready(2, "3p-weekly"));
  assert.equal(hook.render().usage.windows[0].id, "3p-weekly");
  hook.emit("antigravity-usage-updated", ready(1, "outdated-quota"));
  assert.equal(hook.render().usage.windows[0].id, "3p-weekly");
});

test("late initial Codex response cannot appear after switching to Antigravity", async () => {
  const hook = view();
  hook.render(); await hook.settle();
  hook.render("antigravity"); await hook.settle();
  hook.resolve("get_codex_usage", ready(200, "late-codex"));
  hook.resolve("get_antigravity_usage", ready(1, "gemini-weekly"));
  assert.equal((await hook.settle()).usage.windows[0].id, "gemini-weekly");
  assert.deepEqual(hook.calls, ["get_codex_usage", "get_antigravity_usage"]);
});
