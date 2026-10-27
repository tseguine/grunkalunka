import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateCascade, runCascade, verifyCascade } from "../../tools/browser-oracles/cascade.mjs";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const source = await readFile(new URL("./cascade-mutations.html", import.meta.url), "utf8");
function receipt() {
  const ids = ["initial", "normal-inline", "important-inline", "remove-inline", "selected-class", "remove-selected", "remove-box", "restore-box"];
  return { schemaVersion: 1, fixtureId: "cascade-mutations-v1",
    environment: { userAgent: "Test/1", url: "file:///fixture.html", timestamp: "2026-10-27T10:00:00Z",
      viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1 },
    provenance: { collector: "playwright", engine: "chromium", engineFamily: "blink", fixtureSha256: hash(source),
      playwrightVersion: "test", browserVersion: "test", nodeVersion: "test", hostPlatform: "test", hostArchitecture: "test",
      viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations: [40, 40, 64, 40, 52, 40, 80, 40].map((width, i) => ({ id: ids[i], text: "",
      computed: { display: "block", boxSizing: "content-box", width: `${width}px`, height: "10px",
        paddingLeft: "2px", paddingRight: "2px", paddingTop: "2px", paddingBottom: "2px",
        borderLeftWidth: "1px", borderRightWidth: "1px", borderTopWidth: "1px", borderBottomWidth: "1px" },
      rect: { x: 0, y: 0, width: width + 6, height: 16 },
      dom: { className: i === 4 ? "box selected" : i === 6 ? "" : "box",
        inlineWidth: i === 1 ? "100px" : i === 2 ? "64px" : "", inlinePriority: i === 2 ? "important" : "" }
    })) };
}
const verify = c => verifyCascade(c, "chromium", hash(source));

test("independent manifest requires every ordered phase, style, rectangle and DOM state", () => {
  verify(receipt());
  for (let i = 0; i < 8; i++) {
    for (const change of [c => c.observations.splice(i, 1), c => c.observations[i].id = "wrong",
      c => c.observations[i].rect.width++, c => c.observations[i].dom.className += " stale",
      c => c.observations[i].text = "text"]) {
      const c = receipt(); change(c); assert.throws(() => verify(c));
    }
    for (const key of Object.keys(receipt().observations[i].computed)) {
      const c = receipt(); delete c.observations[i].computed[key]; assert.throws(() => verify(c));
    }
  }
  const reversed = receipt(); reversed.observations.reverse(); assert.throws(() => verify(reversed));
});

test("rejects wrong source, identity, environment and malformed observations", () => {
  for (const change of [c => c.schemaVersion++, c => c.fixtureId = "wrong", c => c.provenance.fixtureSha256 = "b".repeat(64),
    c => c.provenance.engine = "firefox", c => c.provenance.engineFamily = "gecko", c => c.provenance.browserVersion = "unrecorded",
    c => c.provenance.viewport.width++, c => c.environment.viewportWidth++, c => c.environment.timestamp = "bad",
    c => c.observations[0].rect.width = NaN, c => c.observations[2].computed.width = "40px",
    c => c.observations[4].dom.inlinePriority = "important"]) {
    const c = receipt(); change(c); assert.throws(() => verify(c));
  }
});

test("guarded source mutations are distinct and never rewrite observed data", () => {
  const mutations = ["reverse-order", "drop-important", "skip-class"].map(kind => {
    assert.throws(() => mutateCascade("no site", kind));
    assert.throws(() => mutateCascade(source + source, kind));
    return mutateCascade(source, kind);
  });
  assert.equal(new Set([source, ...mutations]).size, 4);
  assert.throws(() => mutateCascade(source, "unknown"));
});

test("runner retains exact sources and receipts, and does not accept unrelated rejection", async () => {
  const dir = await mkdtemp(join(tmpdir(), "cascade-oracle-"));
  const collect = async path => {
    const bytes = await readFile(path, "utf8"), c = receipt();
    c.provenance.fixtureSha256 = hash(bytes);
    for (const [kind, index, width] of [["reverse-order", 0, 24], ["drop-important", 0, 80], ["skip-class", 4, 40]]) {
      if (bytes === mutateCascade(source, kind)) {
        c.observations[index].computed.width = `${width}px`;
        c.observations[index].rect.width = width + 6;
      }
    }
    return c;
  };
  try {
    const outputDir = join(dir, "valid");
    const result = await runCascade("chromium", { collect, outputDir });
    assert.equal(result.phases, 8); assert.equal(result.mutations.length, 3);
    assert.equal(await readFile(join(outputDir, "first.html"), "utf8"), source);
    await assert.rejects(runCascade("chromium", { collect, outputDir }), { code: "EEXIST" });
    for (const [i, fault] of [c => c.fixtureId = "wrong", c => c.provenance.fixtureSha256 = "b".repeat(64)].entries()) {
      const invalidDir = join(dir, `invalid-${i}`);
      await assert.rejects(runCascade("chromium", { outputDir: invalidDir, collect: async path => {
        const c = await collect(path);
        if (path.endsWith("reverse-order.html")) fault(c);
        return c;
      } }));
      assert.ok(JSON.parse(await readFile(join(invalidDir, "reverse-order.json"), "utf8")));
    }
  } finally { await rm(dir, { recursive: true, force: true }); }
});
