const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("typescript");

function view(overrides = {}) {
  const hooks = [];
  let cursor = 0;
  let effects = [];
  const env = { resets: 0, openedUrls: [] };
  const dialog = {
    open: false,
    showModal() { this.open = true; },
    close() { this.open = false; },
  };
  const preferences = {
    theme: "dark", backgroundOpacity: 100, contentOpacity: 100,
    startWithWindows: true, startupPending: false, startupError: "",
    autoActivation: { enabled: true, message: "Waiting for the next 5-hour quota reset.", lastAttemptAt: null },
    activationPending: false, activationError: "",
    usageShakeEnabled: true, usageDeductionEnabled: true, usageProvider: "codex",
    refreshIntervalInput: "60", refreshIntervalError: "", clickThrough: false,
    modeError: "", modeErrorVisible: false, settingsClosing: false, settingsOpening: 0,
    resetting: false, resetMessage: "", resetError: "",
    resetSettings: async () => { env.resets++; },
    ...overrides,
  };
  const react = {
    useState(initial) {
      const index = cursor++;
      hooks[index] ??= { value: initial };
      return [hooks[index].value, (value) => { hooks[index].value = value; }];
    },
    useRef(initial) { return hooks[cursor++] ??= { current: initial }; },
    useEffect(callback) { effects.push(callback); },
  };
  const jsx = (type, props, key) => ({ type, props, key });
  const modules = {
    react,
    "react/jsx-runtime": { jsx, jsxs: jsx },
    "@tauri-apps/plugin-opener": { openUrl: async (url) => { env.openedUrls.push(url); } },
    "../shared/MaterialIcon": { MaterialIcon: () => null },
    "./useSettings": { useSettings: () => preferences },
    "../shared/useCodexUsage": { useCodexUsage: () => ({
      usage: { status: "ready", refreshing: false, message: "Connected", account: null, sourcePath: "", updatedAt: null },
      now: 0, connectionError: "", refreshUsage() {},
    }) },
    "../shared/codexUsage": { isUsageStale: () => false, usageStatusLabel: () => "Connected" },
    "../shared/useUsageProvider": { providerNames: { codex: "Codex", antigravity: "Antigravity" }, readUsageProvider: (value) => value },
    "./Settings.css": {},
  };
  const context = { exports: {}, require(name) {
    assert.ok(name in modules, `Unexpected import ${name}`);
    return modules[name];
  } };
  const source = ts.transpileModule(fs.readFileSync(path.join(__dirname, "Settings.tsx"), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX },
  }).outputText;
  vm.runInNewContext(source, context);
  function nodes(tree, predicate) {
    if (Array.isArray(tree)) return tree.flatMap((child) => nodes(child, predicate));
    if (!tree || typeof tree !== "object") return [];
    return [...(predicate(tree) ? [tree] : []), ...nodes(tree.props?.children, predicate)];
  }
  cursor = 0;
  const tree = context.exports.default();
  const popup = nodes(tree, (node) => node.type === "dialog")[0];
  popup.props.ref.current = dialog;
  effects.forEach((effect) => effect());
  return {
    env, dialog, tree, popup,
    find: (predicate, root = tree) => nodes(root, predicate),
  };
}

test("Reset only runs after confirmation; opening and cancelling leave preferences alone", async () => {
  const ui = view();
  const [trigger, confirm] = ui.find((node) => node.type === "button" && node.props.className === "reset-button");
  const cancel = ui.find((node) => node.props?.className === "reset-cancel")[0];
  assert.equal(ui.dialog.open, false);
  assert.equal(cancel.props.autoFocus, true);
  trigger.props.onClick();
  assert.equal(ui.dialog.open, true);
  assert.equal(ui.env.resets, 0);
  cancel.props.onClick();
  assert.equal(ui.dialog.open, false);
  assert.equal(ui.env.resets, 0);
  trigger.props.onClick();
  await confirm.props.onClick();
  assert.equal(ui.dialog.open, false);
  assert.equal(ui.env.resets, 1);
});

test("Escape in the popup does not reach the Settings window shortcut", () => {
  const ui = view();
  let stopped = false;
  ui.popup.props.onKeyDown({ key: "Escape", stopPropagation() { stopped = true; } });
  assert.equal(stopped, true);
  assert.equal(ui.env.resets, 0);
});

test("Reset controls are disabled while resetting, and closing Settings dismisses the popup", () => {
  const ui = view({ resetting: true, settingsClosing: true });
  const controls = ui.find((node) => node.type === "button" && node.props.className === "reset-button");
  assert.equal(controls.length, 2);
  assert.ok(controls.every((control) => control.props.disabled));
  assert.equal(ui.dialog.open, false);
  assert.equal(ui.env.resets, 0);
});

test("Support replaces Reset in the footer and opens the exact permitted Ko-fi URL", async () => {
  const ui = view();
  const footer = ui.find((node) => node.type === "footer")[0];
  const buttons = ui.find((node) => node.type === "button", footer);
  assert.equal(buttons.length, 1);
  assert.equal(buttons[0].props.children, "Support UsagePin");
  await buttons[0].props.onClick();
  const url = "https://ko-fi.com/mdbrelo/?hidefeed=true&widget=true&embed=true&preview=true";
  assert.deepEqual(ui.env.openedUrls, [url]);
  const capability = JSON.parse(fs.readFileSync(path.join(__dirname, "../../backend/capabilities/settings.json"), "utf8"));
  const opener = capability.permissions.find((permission) => permission.identifier === "opener:allow-open-url");
  assert.ok(opener.allow.some((entry) => entry.url === url));
  const resetSection = ui.find((node) => node.props?.className === "reset-setting")[0];
  assert.equal(ui.find((node) => node.props?.className === "reset-button", resetSection).length, 1);
});
