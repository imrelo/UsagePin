const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");
const ts = require("typescript");

const source = ts.transpileModule(fs.readFileSync(path.join(__dirname, "useCompactView.ts"), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;

function harness(width, height, { reducedMotion = false, failAt = 0 } = {}) {
  let clock = 0;
  let resizeCalls = 0;
  const state = { width, height, compact: false, frames: [], progress: [] };
  const position = { x: 1732, y: 96 };
  const appWindow = {
    async innerSize() { return { width: state.width, height: state.height }; },
    async setSize(size) {
      if (++resizeCalls === failAt) throw new Error("Resize failed");
      state.width = size.width;
      state.height = size.height;
      state.frames.push({ width: size.width, height: size.height });
    },
    async setPosition() { throw new Error("The widget must stay in place"); },
  };
  const modules = {
    "react": require("react"),
    "@tauri-apps/api/core": { async invoke(command, args) { assert.equal(command, "set_compact_view"); state.compact = args.compact; } },
    "@tauri-apps/api/dpi": { PhysicalSize: class { constructor(width, height) { this.width = width; this.height = height; } } },
    "@tauri-apps/api/window": { getCurrentWindow: () => appWindow },
  };
  const context = {
    exports: {}, require: (name) => modules[name],
    window: { matchMedia: () => ({ matches: reducedMotion }) },
    performance: { now: () => clock },
    requestAnimationFrame(callback) { clock += 16; queueMicrotask(() => callback(clock)); },
  };
  vm.runInNewContext(source, context, { filename: "useCompactView.ts" });
  return { state, position, animate: (compact) => context.exports.animateCompactView(compact, (progress) => state.progress.push(progress)) };
}

test("collapse and expand preserve width and position at different display sizes", async () => {
  for (const [width, expanded, compact] of [[320, 140, 98], [192, 84, 59], [384, 168, 118]]) {
    const view = harness(width, expanded);
    await view.animate(true);
    assert.equal(view.state.height, compact);
    assert.equal(view.state.width, width);
    assert.equal(view.state.compact, true);
    assert.deepEqual(view.position, { x: 1732, y: 96 });
    assert.equal(view.state.progress.at(-1), 0);
    assert.ok(view.state.progress.length > 1);
    assert.ok(view.state.progress.every((p, i, all) => i === 0 || p <= all[i - 1]));
    assert.ok(view.state.frames.every((frame, i, all) => frame.width === width && (i === 0 || frame.height <= all[i - 1].height)));
    view.state.progress.length = 0;
    view.state.frames.length = 0;
    await view.animate(false);
    assert.equal(view.state.height, expanded);
    assert.equal(view.state.width, width);
    assert.equal(view.state.compact, false);
    assert.equal(view.state.progress.at(-1), 1);
    assert.ok(view.state.progress.every((p, i, all) => i === 0 || p >= all[i - 1]));
  }
});

test("reduced motion changes the window in one step", async () => {
  const view = harness(320, 140, { reducedMotion: true });
  await view.animate(true);
  assert.deepEqual(view.state.frames, [{ width: 320, height: 98 }]);
  assert.deepEqual(view.state.progress, [0]);
});

test("a resize failure restores the original size, mode and visible chrome", async () => {
  const view = harness(320, 140, { failAt: 3 });
  await assert.rejects(view.animate(true), /Resize failed/);
  assert.equal(view.state.width, 320);
  assert.equal(view.state.height, 140);
  assert.equal(view.state.compact, false);
  assert.equal(view.state.progress.at(-1), 1);
});
