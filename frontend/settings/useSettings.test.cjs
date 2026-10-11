const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("typescript");

function environment(saved = {}) {
  return { storage: new Map(Object.entries(saved)), clickThrough: false, interval: 60, fail: "", windows: [] };
}

function settings(env) {
  const hooks = [];
  const listeners = new Map();
  const modules = new Map();
  const currentWindow = { listeners, events: new Map() };
  env.windows.push(currentWindow);
  let cursor, effects, dirty, value;
  const react = {
    useState(initial) {
      const state = hooks[cursor++] ??= { value: typeof initial === "function" ? initial() : initial };
      return [state.value, (update) => {
        const next = typeof update === "function" ? update(state.value) : update;
        if (!Object.is(next, state.value)) { state.value = next; dirty = true; }
      }];
    },
    useEffect(callback, dependencies) {
      const index = cursor++;
      const previous = hooks[index];
      if (!previous || dependencies.some((item, i) => !Object.is(item, previous.dependencies[i]))) {
        effects.push(() => { previous?.cleanup?.(); hooks[index].cleanup = callback(); });
        hooks[index] = { dependencies };
      }
    },
  };
  const globals = {
    document: { documentElement: { dataset: {} } },
    window: {
      localStorage: {
        getItem: (key) => env.storage.get(key) ?? null,
        setItem(key, newValue) {
          if (env.storage.get(key) === newValue) return;
          env.storage.set(key, newValue);
          for (const other of env.windows) {
            if (other !== currentWindow) for (const listener of other.listeners.get("storage") ?? []) listener({ key, newValue });
          }
        },
      },
      addEventListener(name, listener) {
        if (!listeners.has(name)) listeners.set(name, new Set());
        listeners.get(name).add(listener);
      },
      removeEventListener(name, listener) { listeners.get(name)?.delete(listener); },
    },
  };
  async function invoke(command, args) {
    if (command === env.fail) throw new Error("Backend unavailable");
    if (command === "get_click_through") return env.clickThrough;
    if (command === "reset_click_through") {
      env.clickThrough = false;
      for (const other of env.windows) for (const listener of other.events.get("click-through-changed") ?? []) listener({ payload: false });
      return;
    }
    if (command === "toggle_click_through") return env.clickThrough = !env.clickThrough;
    if (command === "set_usage_refresh_interval") { env.interval = args.seconds; return; }
    throw new Error(`Unexpected command: ${command}`);
  }
  function load(filename) {
    if (modules.has(filename)) return modules.get(filename);
    const source = ts.transpileModule(fs.readFileSync(filename, "utf8"), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    }).outputText;
    const context = { exports: {}, ...globals, require(name) {
      if (name === "react") return react;
      if (name === "@tauri-apps/api/core") return { invoke };
      if (name === "@tauri-apps/api/event") return { async listen(event, listener) {
        if (!currentWindow.events.has(event)) currentWindow.events.set(event, new Set());
        currentWindow.events.get(event).add(listener);
        return () => currentWindow.events.get(event).delete(listener);
      } };
      if (name === "@tauri-apps/api/window") return { getCurrentWindow: () => ({ hide() {} }) };
      return load(path.resolve(path.dirname(filename), `${name}.ts`));
    } };
    vm.runInNewContext(source, context, { filename });
    modules.set(filename, context.exports);
    return context.exports;
  }
  const { useSettings } = load(path.join(__dirname, "useSettings.ts"));
  function render() {
    let renders = 0;
    do {
      assert.ok(++renders < 20, "Settings did not settle");
      cursor = 0; effects = []; dirty = false;
      value = useSettings();
      effects.forEach((effect) => effect());
    } while (dirty);
    return value;
  }
  return {
    render,
    async settle() {
      render();
      await new Promise(setImmediate);
      return render();
    },
  };
}

function preferences(value) {
  return {
    theme: value.theme, backgroundOpacity: value.backgroundOpacity, contentOpacity: value.contentOpacity,
    usageShakeEnabled: value.usageShakeEnabled, usageDeductionEnabled: value.usageDeductionEnabled,
    usageProvider: value.usageProvider,
    refreshIntervalInput: value.refreshIntervalInput, clickThrough: value.clickThrough,
  };
}

test("provider selection synchronizes to the widget and survives reopening", async () => {
  const env = environment();
  const view = settings(env), other = settings(env);
  await view.settle(); await other.settle();
  view.render().setUsageProvider("antigravity");
  assert.equal((await view.settle()).usageProvider, "antigravity");
  assert.equal((await other.settle()).usageProvider, "antigravity");
  assert.equal((await settings(env).settle()).usageProvider, "antigravity");
  assert.equal(env.storage.get("usagepin.usage-provider"), "antigravity");
});

test("Reset restores every preference, synchronizes another window, and survives reopening", async () => {
  const defaults = preferences(await settings(environment()).settle());
  const env = environment({
    "usagepin.theme": "light", "usagepin.background-opacity": "25", "usagepin.content-opacity": "45",
    "usagepin.usage-shake-enabled": "false", "usagepin.usage-deduction-enabled": "false",
    "usagepin.refresh-interval-seconds": "90", "unrelated-data": "keep",
    "usagepin.usage-provider": "antigravity",
  });
  env.clickThrough = true;
  const view = settings(env);
  const other = settings(env);
  await view.settle();
  await other.settle();
  const pending = view.render().resetSettings();
  assert.equal(view.render().resetting, true);
  await pending;
  const result = await view.settle();
  assert.deepEqual(preferences(result), defaults);
  assert.equal(result.resetting, false);
  assert.equal(result.resetError, "");
  assert.ok(result.resetMessage);
  assert.equal(env.interval, 60);
  assert.equal(env.clickThrough, false);
  assert.deepEqual(preferences(await other.settle()), defaults);
  assert.deepEqual(preferences(await settings(env).settle()), defaults);
  assert.equal(env.storage.get("unrelated-data"), "keep");
});

test("Reset clears invalid interval input even at defaults and sets mouse mode despite stale UI state", async () => {
  const env = environment();
  const view = settings(env);
  await view.settle();
  view.render().changeRefreshInterval("");
  assert.equal(view.render().refreshIntervalInput, "");
  env.clickThrough = true;
  await view.render().resetSettings();
  const result = await view.settle();
  assert.equal(result.refreshIntervalInput, "60");
  assert.equal(result.clickThrough, false);
  assert.equal(env.clickThrough, false);
  await result.resetSettings();
  assert.equal((await view.settle()).resetError, "");
  assert.equal(env.clickThrough, false);
});

test("Backend reset failures show an error without success and allow a successful retry", async () => {
  for (const command of ["reset_click_through", "set_usage_refresh_interval"]) {
    const env = environment({ "usagepin.refresh-interval-seconds": "90" });
    env.clickThrough = true;
    const view = settings(env);
    await view.settle();
    env.fail = command;
    await view.render().resetSettings();
    const failed = await view.settle();
    assert.ok(failed.resetError);
    assert.equal(failed.resetMessage, "");
    assert.equal(failed.resetting, false);
    env.fail = "";
    await failed.resetSettings();
    const result = await view.settle();
    assert.equal(result.resetError, "");
    assert.ok(result.resetMessage);
    assert.equal(result.clickThrough, false);
    assert.equal(result.refreshIntervalInput, "60");
  }
});
