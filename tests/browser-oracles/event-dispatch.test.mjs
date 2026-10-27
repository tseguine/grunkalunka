import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateEvents, runEvents, verifyEvents } from "../../tools/browser-oracles/events.mjs";

const source = await readFile(new URL("./event-dispatch.html", import.meta.url), "utf8");
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
// Independent synthetic receipts test the verifier; these are NOT browser execution.
function receipt(defect = null) {
  const steps = { R: ["root-capture", "root", 1], P: ["parent-capture", "parent", 1],
    S: ["parent-second", "parent", 1], C: ["target-capture", "target", 2],
    T: ["target-first", "target", 2], U: ["target-second", "target", 2],
    B: ["parent-bubble", "parent", 3], E: ["root-bubble", "root", 3] };
  const orders = { ordinary: ["RPSCTUBE", "RPSCTUBE"], once: ["RPSCTUBE", "RPSCUBE"],
    stop: ["RPS", "RPS"], "stop-immediate": ["RP", "RP"], cancel: ["RPSCTUBE", "RPSCTUBE"],
    noncancelable: ["RPSCTUBE", "RPSCTUBE"], nonbubbling: ["RPSCTU", "RPSCTU"] };
  if (defect === "skip-stop") orders.stop = orders.ordinary;
  if (defect === "skip-once") orders.once = orders.ordinary;
  return { schemaVersion: 1, fixtureId: "event-dispatch-v1", observationKind: "event-dispatch",
    environment: { userAgent: "Test/1", url: "file:///fixture.html", timestamp: "2026-10-27T12:00:00Z",
      viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1 },
    provenance: { collector: "playwright", engine: "chromium", engineFamily: "blink", fixtureSha256: hash(source),
      playwrightVersion: "test", browserVersion: "test", nodeVersion: "test", hostPlatform: "test", hostArchitecture: "test",
      viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations: Object.entries(orders).map(([id, pair]) => ({ id, dispatches: pair.map((order, i) => {
      const canceled = id === "cancel" && defect !== "skip-cancel";
      return { dispatch: i + 1, bubbles: id !== "nonbubbling", cancelable: id !== "noncancelable",
        trace: [...order].map(key => ({ listener: steps[key][0], target: "target", currentTarget: steps[key][1],
          phase: steps[key][2], defaultPrevented: canceled && "TUBE".includes(key),
          cancelBubble: (id === "stop-immediate" || (id === "stop" && defect !== "skip-stop")) && "PS".includes(key),
          isTrusted: false, path: ["target", "parent", "root"] })), returned: !canceled,
        after: { target: "target", currentTarget: null, phase: 0, defaultPrevented: canceled,
          cancelBubble: false, isTrusted: false, path: [] } };
    }) })) };
}
const verify = c => verifyEvents(c, "chromium", hash(source));

test("all scenarios require exact ordered traces, flags, returns and post-dispatch cleanup", () => {
  verify(receipt());
  for (let s = 0; s < 7; s++) for (let d = 0; d < 2; d++) {
    for (const change of [x => x.trace.reverse(), x => x.trace.pop(), x => x.trace[0].phase++,
      x => x.trace[0].target = "root", x => x.trace[0].currentTarget = "target",
      x => x.trace[0].isTrusted = true, x => x.trace[0].path.reverse(),
      x => x.returned = !x.returned, x => x.after.currentTarget = "target", x => x.after.phase = 3,
      x => x.after.cancelBubble = true, x => x.after.path = ["target"],
      x => x.after.defaultPrevented = !x.after.defaultPrevented, x => x.cancelable = !x.cancelable,
      x => x.bubbles = !x.bubbles, x => delete x.trace[0].defaultPrevented]) {
      const c = receipt(); change(c.observations[s].dispatches[d]); assert.throws(() => verify(c));
    }
  }
});

test("rejects absent scenarios, wrong contracts, provenance and malformed records", () => {
  for (const change of [c => c.observations.reverse(), c => c.observations.pop(), c => c.observations = null,
    c => c.observations[0].dispatches.pop(), c => c.observations[0].dispatches[0] = null,
    c => c.schemaVersion++, c => c.observationKind = "layout", c => c.fixtureId = "wrong",
    c => c.provenance.fixtureSha256 = "a".repeat(64), c => c.provenance.engine = "firefox",
    c => c.provenance.engineFamily = "gecko", c => c.provenance.browserVersion = "unrecorded",
    c => c.provenance.viewport.width++, c => c.environment.viewportWidth++, c => c.environment.timestamp = "bad"]) {
    const c = receipt(); change(c); assert.throws(() => verify(c));
  }
});

test("source defects have guarded sites and fail at the intended semantic transition", () => {
  for (const [kind, failure] of [["skip-stop", "stop/dispatch-1"], ["skip-cancel", "cancel/dispatch-1"], ["skip-once", "once/dispatch-2"]]) {
    assert.throws(() => mutateEvents("no site", kind));
    assert.throws(() => mutateEvents(source + source, kind));
    const c = receipt(kind), changed = mutateEvents(source, kind);
    c.provenance.fixtureSha256 = hash(changed);
    verifyEvents(c, "chromium", hash(changed), kind);
    assert.throws(() => verifyEvents(c, "chromium", hash(changed)), error => error.message.includes(failure));
    // Correct defect plus unrelated corrupted trace must not qualify as success.
    c.observations[0].dispatches[0].returned = false;
    assert.throws(() => verifyEvents(c, "chromium", hash(changed), kind));
  }
  assert.throws(() => mutateEvents(source, "unknown"));
});

test("runner retains sources and failures, rejects reused directories and no-op mutations", async () => {
  const dir = await mkdtemp(join(tmpdir(), "event-oracle-"));
  const collect = async path => {
    const bytes = await readFile(path, "utf8");
    const defect = ["skip-stop", "skip-cancel", "skip-once"].find(k => bytes === mutateEvents(source, k));
    const c = receipt(defect); c.provenance.fixtureSha256 = hash(bytes); return c;
  };
  try {
    const outputDir = join(dir, "valid"), summary = await runEvents("chromium", { collect, outputDir });
    assert.equal(summary.scenarios, 7); assert.equal(summary.mutations.length, 3);
    assert.equal(await readFile(join(outputDir, "first.html"), "utf8"), source);
    await assert.rejects(runEvents("chromium", { collect, outputDir }), { code: "EEXIST" });
    for (const [i, fault] of [c => c.fixtureId = "wrong", c => c.provenance.fixtureSha256 = "b".repeat(64),
      c => c.observations = receipt().observations, c => c.observations[0].dispatches[0].returned = false].entries()) {
      const out = join(dir, `bad-${i}`);
      await assert.rejects(runEvents("chromium", { outputDir: out, collect: async path => {
        const c = await collect(path); if (path.endsWith("skip-stop.html")) fault(c); return c;
      } }));
      assert.ok(JSON.parse(await readFile(join(out, "skip-stop.json"), "utf8")));
      await assert.rejects(readFile(join(out, "summary.json")), { code: "ENOENT" });
    }
    await assert.rejects(runEvents("chromium", { outputDir: join(dir, "launch-failure"),
      collect: async () => { throw new Error("launch failed"); } }), /launch failed/);
  } finally { await rm(dir, { recursive: true, force: true }); }
});
