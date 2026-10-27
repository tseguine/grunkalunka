import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateShadowSource, runShadowEvents, verifyShadowEvents } from "../../tools/browser-oracles/shadow-events.mjs";

const source = await readFile(new URL("./shadow-events.html", import.meta.url), "utf8");
const hash = s => createHash("sha256").update(s).digest("hex");
const listeners = {
  "scoped-no-bubble": ["shadow-capture", "inner-capture", "inner-bubble"],
  "scoped-bubble": ["shadow-capture", "inner-capture", "inner-bubble", "shadow-bubble"],
  "composed-no-bubble": ["outer-capture", "host-capture", "shadow-capture", "inner-capture", "inner-bubble", "host-bubble"],
  "composed-bubble": ["outer-capture", "host-capture", "shadow-capture", "inner-capture", "inner-bubble", "shadow-bubble", "host-bubble", "outer-bubble"]
};
function synthetic(defect = null) {
  const observations = [];
  for (const mode of ["open", "closed"]) for (const composed of [false, true]) for (const bubbles of [false, true]) {
    const id = `${mode}-${composed ? "composed" : "scoped"}-${bubbles ? "bubble" : "no-bubble"}`;
    const changed = (defect === "skip-composed" || defect === "skip-bubble") && id === "open-composed-bubble";
    const effectiveComposed = changed && defect === "skip-composed" ? false : composed;
    const effectiveBubbles = changed && defect === "skip-bubble" ? false : bubbles;
    const rootMode = defect === "open-closed-root" && id === "closed-composed-bubble" ? "open" : mode;
    const full = effectiveComposed ? ["inner", "shadow", "host", "outer"] : ["inner", "shadow"];
    const outside = rootMode === "closed" ? ["host", "outer"] : full;
    const sequence = listeners[`${effectiveComposed ? "composed" : "scoped"}-${effectiveBubbles ? "bubble" : "no-bubble"}`];
    observations.push({ id, mode, composed, bubbles, returned: true,
      trace: sequence.map(listener => {
        const currentTarget = listener.split("-")[0];
        const far = currentTarget === "host" || currentTarget === "outer";
        return { listener, target: far ? "host" : "inner", currentTarget,
          phase: currentTarget === "host" || currentTarget === "inner" ? 2 : listener.endsWith("capture") ? 1 : 3,
          composed: effectiveComposed, bubbles: effectiveBubbles, isTrusted: false,
          path: far ? outside : full };
      }),
      after: { target: effectiveComposed ? "host" : null, currentTarget: null, phase: 0, path: [] }
    });
  }
  return { schemaVersion: 1, fixtureId: "shadow-event-boundary-v1", observationKind: "shadow-events",
    environment: { userAgent: "Synthetic/1", viewportWidth: 800, viewportHeight: 600,
      devicePixelRatio: 1, url: "file:///oracle.html", timestamp: "2026-10-27T14:00:00Z" },
    provenance: { collector: "playwright", playwrightVersion: "test", engine: "chromium", engineFamily: "blink",
      browserVersion: "test", fixtureSha256: hash(source), hostPlatform: "test", hostArchitecture: "test",
      nodeVersion: "test", viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations };
}
const verify = c => verifyShadowEvents(c, "chromium", hash(source));

test("independent exact event retargeting, phases and paths reject every trace/cleanup mutation", () => {
  verify(synthetic());
  for (let i = 0; i < 8; i++) {
    for (const change of [
      x => x.trace.pop(), x => x.trace.reverse(), x => x.trace[0].listener = "bogus",
      x => x.trace[0].target = "invalid", x => x.trace[0].currentTarget = "invalid",
      x => x.trace[0].phase = 0, x => x.trace[0].path = [],
      x => x.trace[0].isTrusted = true, x => x.trace[0].composed = !x.trace[0].composed,
      x => x.trace[0].bubbles = !x.trace[0].bubbles,
      x => x.after.currentTarget = "host", x => x.after.phase = 2,
      x => x.after.path = ["host"], x => x.after.target = "random",
      x => x.returned = false, x => x.mode = "unknown", x => x.bubbles = !x.bubbles,
      x => x.composed = !x.composed
    ]) {
      const c = synthetic(); change(c.observations[i]);
      assert.throws(() => verify(c), `rejects ${i}: ${change.toString()}`);
    }
  }
});

test("complete independent payload, environment and provenance must be valid", () => {
  for (const change of [c => c.schemaVersion++, c => c.fixtureId = "wrong",
    c => c.observationKind = "layout", c => c.observations.reverse(), c => c.observations.pop(),
    c => c.observations[2].trace[0].path.push("document"),
    c => c.provenance.fixtureSha256 = "0".repeat(64), c => c.provenance.engine = "firefox",
    c => c.provenance.engineFamily = "gecko", c => c.provenance.browserVersion = "unrecorded",
    c => c.provenance.viewport.width++, c => c.environment.viewportWidth++,
    c => c.environment.timestamp = "bad", c => c.environment.url = "",
    c => c.observations[0].trace = null, c => c.observations[7].after.target = "inner"]) {
    const c = synthetic(); change(c); assert.throws(() => verify(c));
  }
});

test("mutated source satisfies the entire alternate trace before targeted baseline rejection", () => {
  for (const [kind, phase] of [["skip-composed", "open-composed-bubble"],
    ["skip-bubble", "open-composed-bubble"], ["open-closed-root", "closed-composed-bubble"]]) {
    assert.throws(() => mutateShadowSource("missing mutation site", kind));
    assert.throws(() => mutateShadowSource(source + source, kind));
    const changed = mutateShadowSource(source, kind), c = synthetic(kind);
    c.provenance.fixtureSha256 = hash(changed);
    verifyShadowEvents(c, "chromium", hash(changed), kind);
    assert.throws(() => verifyShadowEvents(c, "chromium", hash(changed)),
      error => error.message.includes(`${phase}: exact retargeting`));
    c.observations[7].after.phase = 2; // A later unrelated defect must not be hidden.
    assert.throws(() => verifyShadowEvents(c, "chromium", hash(changed), kind));
  }
  assert.throws(() => mutateShadowSource(source, "unknown"));
});

test("runner saves each source/receipt before validation and rejects stale output, corrupt evidence or launch errors", async () => {
  const tmp = await mkdtemp(join(tmpdir(), "shadow-event-oracle-"));
  const defects = ["skip-composed", "skip-bubble", "open-closed-root"];
  const collect = async file => {
    const bytes = await readFile(file, "utf8");
    const kind = defects.find(k => mutateShadowSource(source, k) === bytes);
    const c = synthetic(kind); c.provenance.fixtureSha256 = hash(bytes);
    return c;
  };
  try {
    const outputDir = join(tmp, "normal");
    const result = await runShadowEvents("chromium", { collect, outputDir });
    assert.equal(result.scenarios, 8);
    assert.deepEqual(result.mutations.map(m => m.kind), defects);
    assert.equal(await readFile(join(outputDir, "first.html"), "utf8"), source);
    await assert.rejects(runShadowEvents("chromium", { collect, outputDir }), { code: "EEXIST" });
    for (const [i, change] of [c => c.provenance.fixtureSha256 = "0".repeat(64),
      c => c.fixtureId = "wrong", c => c.observations[7].trace.pop()].entries()) {
      const target = join(tmp, `corrupt-${i}`);
      await assert.rejects(runShadowEvents("chromium", { outputDir: target, collect: async file => {
        const c = await collect(file);
        if (file.endsWith("skip-composed.html")) change(c);
        return c;
      } }));
      assert.ok(JSON.parse(await readFile(join(target, "skip-composed.json"), "utf8")));
      await assert.rejects(readFile(join(target, "summary.json")), { code: "ENOENT" });
    }
    await assert.rejects(runShadowEvents("chromium", { outputDir: join(tmp, "unavailable"),
      collect: async () => { throw Error("browser launch unavailable"); } }), /browser launch unavailable/);
  } finally { await rm(tmp, { recursive: true, force: true }); }
});
