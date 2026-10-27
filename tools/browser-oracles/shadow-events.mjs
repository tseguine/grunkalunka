import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// The tables are derived from DOM's retargeting/event-path construction and
// listener invocation phases. The browser fixture never imports these rules.
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const modes = ["open", "closed"], flags = [false, true];
const scenarios = modes.flatMap(mode => flags.flatMap(composed => flags.map(bubbles => ({
  id: `${mode}-${composed ? "composed" : "scoped"}-${bubbles ? "bubble" : "no-bubble"}`,
  mode, composed, bubbles
}))));
const sourceSites = {
  "skip-composed": {
    phase: "open-composed-bubble",
    before: '        const event = new Event("probe", { composed, bubbles });',
    after: '        const event = new Event("probe", { composed: id === "open-composed-bubble" ? false : composed, bubbles });'
  },
  "skip-bubble": {
    phase: "open-composed-bubble",
    before: '        const event = new Event("probe", { composed, bubbles });',
    after: '        const event = new Event("probe", { composed, bubbles: id === "open-composed-bubble" ? false : bubbles });'
  },
  "open-closed-root": {
    phase: "closed-composed-bubble",
    before: '        const root = host.attachShadow({ mode });',
    after: '        const root = host.attachShadow({ mode: id === "closed-composed-bubble" ? "open" : mode });'
  }
};
const sha256 = value => createHash("sha256").update(value).digest("hex");

function expectedScenario({ id, mode, composed, bubbles }, defect) {
  const changed = sourceSites[defect]?.phase === id;
  const actualComposed = changed && defect === "skip-composed" ? false : composed;
  const actualBubbles = changed && defect === "skip-bubble" ? false : bubbles;
  const actualMode = changed && defect === "open-closed-root" ? "open" : mode;
  const innerPath = actualComposed ? ["inner", "shadow", "host", "outer"] : ["inner", "shadow"];
  const outsidePath = actualMode === "closed" ? ["host", "outer"] : innerPath;
  const listeners = [
    ...(actualComposed ? ["outer-capture", "host-capture"] : []),
    "shadow-capture", "inner-capture", "inner-bubble",
    ...(actualBubbles ? ["shadow-bubble"] : []),
    ...(actualComposed ? ["host-bubble"] : []),
    ...(actualComposed && actualBubbles ? ["outer-bubble"] : [])
  ];
  const trace = listeners.map(listener => {
    const currentTarget = listener.split("-")[0];
    const outside = currentTarget === "outer" || currentTarget === "host";
    const target = outside ? "host" : "inner";
    const phase = currentTarget === "host" || currentTarget === "inner" ? 2 :
      listener.endsWith("-capture") ? 1 : 3;
    return { listener, target, currentTarget, phase,
      composed: actualComposed, bubbles: actualBubbles,
      isTrusted: false, path: outside ? outsidePath : innerPath };
  });
  return { id, mode, composed, bubbles, trace, returned: true,
    after: { target: actualComposed ? "host" : null, currentTarget: null, phase: 0, path: [] } };
}

export function verifyShadowEvents(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine required");
  assert.ok(defect === null || Object.hasOwn(sourceSites, defect), "known source defect required");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "source SHA-256");
  assert.equal(capture?.schemaVersion, 1, "capture schema");
  assert.equal(capture.fixtureId, "shadow-event-boundary-v1", "fixture identity");
  assert.equal(capture.observationKind, "shadow-events", "observation kind");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.collector, "playwright", "collector identity");
  assert.equal(p.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.engine, engine, "engine identity");
  assert.equal(p.engineFamily, families[engine], "engine family");
  for (const field of ["playwrightVersion", "browserVersion", "nodeVersion", "hostPlatform", "hostArchitecture"]) {
    assert.ok(typeof p[field] === "string" && p[field].trim() && p[field] !== "unrecorded", field);
  }
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 }, "selected viewport");
  assert.equal(e?.viewportWidth, 800, "CSS viewport width");
  assert.equal(e.viewportHeight, 600, "CSS viewport height");
  assert.equal(e.devicePixelRatio, 1, "device pixel ratio");
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)), "timestamp");
  for (const field of ["userAgent", "url"]) assert.ok(typeof e[field] === "string" && e[field].length, field);
  assert.ok(Array.isArray(capture.observations), "observations required");
  assert.deepEqual(capture.observations.map(o => o?.id), scenarios.map(s => s.id), "scenario sequence");
  for (const [i, spec] of scenarios.entries()) {
    assert.deepEqual(capture.observations[i], expectedScenario(spec, defect), `${spec.id}: exact retargeting, phases and path`);
  }
}

export function mutateShadowSource(source, kind) {
  assert.ok(Object.hasOwn(sourceSites, kind), "known source defect");
  const { before, after } = sourceSites[kind];
  assert.equal(source.split(before).length, 2, `${kind}: exactly one mutation site`);
  return source.replace(before, after);
}

export async function runShadowEvents(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/shadow-events.html", import.meta.url));
  await mkdir(outputDir); // An existing evidence directory must fail, not mingle captures.
  const take = async (bytes, name) => {
    const file = join(outputDir, `${name}.html`);
    await writeFile(file, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(file);
    await writeFile(join(outputDir, `${name}.json`), JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
    return capture; // Retain even invalid original evidence before asserting.
  };
  const originalHash = sha256(source);
  const first = await take(source, "first"); verifyShadowEvents(first, engine, originalHash);
  const repeat = await take(source, "repeat"); verifyShadowEvents(repeat, engine, originalHash);
  assert.deepEqual(repeat.provenance, first.provenance, "fresh-instance provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-instance stability");
  const mutations = [];
  for (const [kind, { phase }] of Object.entries(sourceSites)) {
    const changed = mutateShadowSource(source.toString("utf8"), kind), changedHash = sha256(changed);
    const capture = await take(changed, kind);
    verifyShadowEvents(capture, engine, changedHash, kind); // Complete alternate trace, not just changed value/hash.
    assert.throws(() => verifyShadowEvents(capture, engine, changedHash), error =>
      error.code === "ERR_ASSERTION" && error.message.includes(`${phase}: exact retargeting`));
    mutations.push({ kind, phase, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash: originalHash, scenarios: scenarios.length,
    repeat: "equal", mutations };
  await writeFile(join(outputDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n", { flag: "wx" });
  return summary;
}
