import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateLifecycle, runLifecycle, verifyLifecycle } from "../../tools/browser-oracles/listener-lifecycle.mjs";

const source = await readFile(new URL("./listener-lifecycle.html", import.meta.url), "utf8");
const hash = text => createHash("sha256").update(text).digest("hex");
const after = canceled => ({ target: "target", currentTarget: null, phase: 0,
  bubbles: true, cancelable: true, defaultPrevented: canceled, cancelBubble: false,
  isTrusted: false, path: [] });
// Synthetic model for unit tests, not browser evidence and not imported by verifier.
function receipt(defect = null) {
  const cases = [
    ["remove-during-dispatch", ["remover"], ["remover"]],
    ["add-during-dispatch", ["adder", "stable"], ["adder", "stable", "added"]],
    ["nested-once", ["once", "peer:nested", "peer"], ["peer"]],
    ["passive-cancel", ["passive", "observer"], ["passive", "observer"]]
  ];
  if (defect === "skip-remove") cases[0][1].push("later"), cases[0][2].push("later");
  if (defect === "skip-add") cases[1][2].pop();
  if (defect === "skip-once") {
    cases[2][1].splice(1, 0, "once:nested");
    cases[2][2] = [...cases[2][1]];
  }
  const observations = cases.map(([id, first, second]) => ({ id,
    dispatches: [first, second].map((names, i) => {
      const canceled = id === "passive-cancel" && defect === "disable-passive";
      const hasNested = id === "nested-once" && (i === 0 || defect === "skip-once");
      return { dispatch: i + 1, trace: names.map(key => {
        const [listener, invocation = "outer"] = key.split(":");
        return { listener, invocation, target: "target", currentTarget: "target", phase: 2,
          bubbles: true, cancelable: true, defaultPrevented: canceled, cancelBubble: false,
          isTrusted: false, path: ["target"] };
      }), returned: !canceled, after: after(canceled),
      nested: hasNested ? [{ returned: true, after: after(false) }] : [] };
    })
  }));
  return { schemaVersion: 1, fixtureId: "listener-lifecycle-v1", observationKind: "listener-lifecycle",
    environment: { userAgent: "Test/1", url: "file:///fixture.html", timestamp: "2026-10-27T12:00:00Z",
      viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1 },
    provenance: { collector: "playwright", engine: "chromium", engineFamily: "blink", fixtureSha256: hash(source),
      playwrightVersion: "test", browserVersion: "test", nodeVersion: "test", hostPlatform: "test",
      hostArchitecture: "test", viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations };
}
const verify = capture => verifyLifecycle(capture, "chromium", hash(source));

test("verifier rejects changed lifecycle traces, flags, nested returns and cleanup", () => {
  verify(receipt());
  for (let scenario = 0; scenario < 4; scenario++) for (let dispatch = 0; dispatch < 2; dispatch++) {
    const changes = [d => d.trace.push(d.trace[0]), d => d.trace = d.trace.length > 1 ? [...d.trace].reverse() : [],
      d => d.trace[0].listener = "wrong", d => d.trace[0].invocation = "nested",
      d => d.trace[0].defaultPrevented = true, d => d.trace[0].phase = 3,
      d => d.trace[0].currentTarget = "document", d => d.trace[0].isTrusted = true,
      d => d.trace[0].path = [], d => d.returned = false,
      d => d.after.target = null, d => d.after.currentTarget = "target",
      d => d.after.defaultPrevented = true, d => d.after.phase = 2,
      d => d.after.path = ["target"], d => d.nested.push({ returned: true, after: after(false) })];
    for (const change of changes) {
      const capture = receipt(); change(capture.observations[scenario].dispatches[dispatch]);
      assert.throws(() => verify(capture));
    }
  }
});

test("verifier requires complete independent payload and provenance", () => {
  for (const change of [c => c.observations.pop(), c => c.observations.reverse(),
    c => c.observations[0].dispatches.pop(), c => c.observations[0].dispatches[0] = null,
    c => c.observations = null, c => c.schemaVersion++, c => c.fixtureId = "unexpected",
    c => c.observationKind = "layout", c => c.provenance.fixtureSha256 = "0".repeat(64),
    c => c.provenance.engine = "firefox", c => c.provenance.engineFamily = "gecko",
    c => c.provenance.browserVersion = "unrecorded", c => c.provenance.viewport.width++,
    c => c.environment.viewportWidth++, c => c.environment.timestamp = "invalid"]) {
    const c = receipt(); change(c); assert.throws(() => verify(c));
  }
});

test("four single-site source mutations pass full alternate traces but fail baseline at intended dispatch", () => {
  const cases = [["skip-remove", "remove-during-dispatch/dispatch-1"],
    ["skip-add", "add-during-dispatch/dispatch-2"],
    ["skip-once", "nested-once/dispatch-1"],
    ["disable-passive", "passive-cancel/dispatch-1"]];
  for (const [kind, failure] of cases) {
    assert.throws(() => mutateLifecycle("no site", kind));
    assert.throws(() => mutateLifecycle(source + source, kind));
    const changed = mutateLifecycle(source, kind), c = receipt(kind);
    c.provenance.fixtureSha256 = hash(changed);
    verifyLifecycle(c, "chromium", hash(changed), kind);
    assert.throws(() => verifyLifecycle(c, "chromium", hash(changed)), e => e.message.includes(failure));
    c.observations[3].dispatches[1].after.target = null;
    assert.throws(() => verifyLifecycle(c, "chromium", hash(changed), kind),
      "an unrelated error cannot masquerade as expected sensitivity");
  }
  assert.throws(() => mutateLifecycle(source, "unknown"));
});

test("runner preserves raw evidence, repeats and mutations; no stale or failed success summaries", async () => {
  const root = await mkdtemp(join(tmpdir(), "lifecycle-oracle-"));
  const defects = ["skip-remove", "skip-add", "skip-once", "disable-passive"];
  const collect = async path => {
    const bytes = await readFile(path, "utf8");
    const defect = defects.find(kind => bytes === mutateLifecycle(source, kind));
    const c = receipt(defect); c.provenance.fixtureSha256 = hash(bytes); return c;
  };
  try {
    const valid = join(root, "valid"), result = await runLifecycle("chromium", { collect, outputDir: valid });
    assert.equal(result.scenarios, 4); assert.equal(result.mutations.length, 4);
    assert.equal(await readFile(join(valid, "first.html"), "utf8"), source);
    await assert.rejects(runLifecycle("chromium", { collect, outputDir: valid }), { code: "EEXIST" });
    for (const [i, change] of [c => c.fixtureId = "wrong", c => c.provenance.fixtureSha256 = "a".repeat(64),
      c => c.observations = receipt().observations, c => c.observations[0].dispatches[0].returned = false].entries()) {
      const outputDir = join(root, `failed-${i}`);
      await assert.rejects(runLifecycle("chromium", { outputDir, collect: async path => {
        const c = await collect(path);
        if (path.endsWith("skip-remove.html")) change(c);
        return c;
      } }));
      assert.ok(JSON.parse(await readFile(join(outputDir, "skip-remove.json"), "utf8")));
      await assert.rejects(readFile(join(outputDir, "summary.json")), { code: "ENOENT" });
    }
    await assert.rejects(runLifecycle("chromium", { outputDir: join(root, "launch-failure"),
      collect: async () => { throw Error("browser unavailable"); } }), /browser unavailable/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
