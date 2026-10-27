import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateFlex, runFlex, verifyFlex } from "../../tools/browser-oracles/flex.mjs";

const source = await readFile(new URL("./flex-mutations.html", import.meta.url), "utf8");
const hash = s => createHash("sha256").update(s).digest("hex");

// Manually specified samples, deliberately not imported from the verifier model.
const samples = [
  ["initial", [50, 200, 30], [0, 60, 270], 50, 1, 10, "row", "", "", "", ""],
  ["no-growth", [50, 0, 30], [0, 60, 70], 50, 0, 10, "row", "", "0", "", ""],
  ["restore-growth", [50, 200, 30], [0, 60, 270], 50, 1, 10, "row", "", "", "", ""],
  ["increase-basis", [80, 170, 30], [0, 90, 270], 80, 1, 10, "row", "80px", "", "", ""],
  ["increase-gap", [80, 150, 30], [0, 100, 270], 80, 1, 20, "row", "80px", "", "20px", ""],
  ["reverse-axis", [80, 150, 30], [220, 50, 0], 80, 1, 20, "row-reverse", "80px", "", "20px", "row-reverse"],
  ["restore-gap", [80, 170, 30], [220, 40, 0], 80, 1, 10, "row-reverse", "80px", "", "10px", "row-reverse"],
  ["restore-axis", [80, 170, 30], [0, 90, 270], 80, 1, 10, "row", "80px", "", "10px", "row"],
  ["restore-basis", [50, 200, 30], [0, 60, 270], 50, 1, 10, "row", "", "", "10px", "row"]
];
function receipt(defect = null) {
  const observations = samples.map(([id, originalWidths, originalXs, origBasis, origGrow, origGap, origDir, inlineBasis, originalInlineGrow, originalInlineGap, originalInlineDir]) => {
    let widths = [...originalWidths], xs = [...originalXs], basis = origBasis, grow = origGrow,
      gap = origGap, direction = origDir, inlineGrow = originalInlineGrow,
      inlineGap = originalInlineGap, inlineDir = originalInlineDir;
    if (defect === "skip-grow" && id === "no-growth") {
      widths = [50, 200, 30]; xs = [0, 60, 270]; grow = 1; inlineGrow = "";
    }
    if (defect === "skip-gap" && id === "increase-gap") {
      widths = [80, 170, 30]; xs = [0, 90, 270]; gap = 10; inlineGap = "";
    }
    if (defect === "skip-gap" && id === "reverse-axis") {
      widths = [80, 170, 30]; xs = [220, 40, 0]; gap = 10; inlineGap = "";
    }
    if (defect === "skip-reverse" && id === "reverse-axis") {
      xs = [0, 100, 270]; direction = "row"; inlineDir = "";
    }
    if (defect === "skip-reverse" && id === "restore-gap") {
      xs = [0, 90, 270]; direction = "row"; inlineDir = "";
    }
    return { id, container: {
      computed: { display: "flex", width: "300px", height: "40px", flexDirection: direction,
        gap: `${gap}px` },
      inlineGap, inlineDirection: inlineDir, rect: { x: 0, y: 0, width: 300, height: 40 }
    }, items: ["alpha", "beta", "gamma"].map((name, i) => ({
      id: name, text: "", computed: { display: "block", width: `${widths[i]}px`,
        height: "20px", flexGrow: String(i === 1 ? grow : 0),
        flexBasis: i === 0 ? `${basis}px` : i === 1 ? "0px" : "30px", minWidth: "0px" },
      rect: { x: xs[i], y: 0, width: widths[i], height: 20 },
      inlineBasis: i === 0 ? inlineBasis : "", inlineGrow: i === 1 ? inlineGrow : ""
    })) };
  });
  return { schemaVersion: 1, fixtureId: "flex-mutations-v1", observationKind: "flex-layout",
    environment: { userAgent: "Test/1", url: "file:///fixture.html", timestamp: "2026-10-27T12:00:00Z",
      viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1 },
    provenance: { collector: "playwright", engine: "chromium", engineFamily: "blink", fixtureSha256: hash(source),
      playwrightVersion: "test", browserVersion: "test", nodeVersion: "test", hostPlatform: "test",
      hostArchitecture: "test", viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations };
}
const verify = c => verifyFlex(c, "chromium", hash(source));

test("manual flex layout table satisfies independent expected geometry", () => {
  verify(receipt());
  for (let i = 0; i < samples.length; i++) {
    for (const change of [x => x.id = "bad", x => x.container.inlineGap = "200px",
      x => x.container.rect.width++, x => x.container.computed.flexDirection = "column",
      x => x.items.pop(), x => x.items[0].rect.x++, x => x.items[1].rect.width++,
      x => x.items[1].computed.flexBasis = "auto", x => x.items[2].text = "x",
      x => x.items[1].inlineGrow = "3", x => x.items.reverse()]) {
      const c = receipt(); change(c.observations[i]); assert.throws(() => verify(c), `${samples[i][0]} defect`);
    }
  }
});

test("complete provenance and phase sequence are mandatory", () => {
  for (const change of [c => c.schemaVersion++, c => c.observationKind = "block",
    c => c.fixtureId = "wrong", c => c.observations.reverse(), c => c.observations.pop(),
    c => c.provenance.fixtureSha256 = "0".repeat(64), c => c.provenance.engine = "firefox",
    c => c.provenance.engineFamily = "gecko", c => c.provenance.viewport.width++,
    c => c.provenance.browserVersion = "unrecorded", c => c.environment.viewportWidth++,
    c => c.environment.timestamp = "bad", c => c.observations[0].items[0].rect.width = NaN]) {
    const c = receipt(); change(c); assert.throws(() => verify(c));
  }
});

test("guarded source mutations pass full alternate geometry and fail specific baseline phase", () => {
  for (const [kind, phase] of [["skip-grow", "no-growth"], ["skip-gap", "increase-gap"],
    ["skip-reverse", "reverse-axis"]]) {
    assert.throws(() => mutateFlex("no match", kind));
    assert.throws(() => mutateFlex(source + source, kind));
    const changed = mutateFlex(source, kind), c = receipt(kind);
    c.provenance.fixtureSha256 = hash(changed);
    verifyFlex(c, "chromium", hash(changed), kind);
    assert.throws(() => verifyFlex(c, "chromium", hash(changed)),
      error => error.message.includes(`${phase}: expected flex`));
    c.observations[8].items[1].rect.width++;
    assert.throws(() => verifyFlex(c, "chromium", hash(changed), kind),
      "later corruption cannot hide behind the intentionally earlier failure");
  }
  assert.throws(() => mutateFlex(source, "unknown"));
});

test("runner retains raw source, rejects stale output and does not accept unrelated errors", async () => {
  const root = await mkdtemp(join(tmpdir(), "flex-oracle-"));
  const kinds = ["skip-grow", "skip-gap", "skip-reverse"];
  const collect = async path => {
    const bytes = await readFile(path, "utf8");
    const defect = kinds.find(kind => bytes === mutateFlex(source, kind));
    const c = receipt(defect); c.provenance.fixtureSha256 = hash(bytes); return c;
  };
  try {
    const dir = join(root, "passed"), result = await runFlex("chromium", { collect, outputDir: dir });
    assert.equal(result.phases, 9); assert.equal(result.mutations.length, 3);
    assert.equal(await readFile(join(dir, "first.html"), "utf8"), source);
    await assert.rejects(runFlex("chromium", { collect, outputDir: dir }), { code: "EEXIST" });
    for (const [i, change] of [c => c.fixtureId = "wrong", c => c.provenance.fixtureSha256 = "f".repeat(64),
      c => c.observations = receipt().observations, c => c.observations[8].items[2].rect.x = -1].entries()) {
      const broken = join(root, `rejected-${i}`);
      await assert.rejects(runFlex("chromium", { outputDir: broken, collect: async path => {
        const c = await collect(path);
        if (path.endsWith("skip-grow.html")) change(c);
        return c;
      } }));
      assert.ok(JSON.parse(await readFile(join(broken, "skip-grow.json"), "utf8")));
      await assert.rejects(readFile(join(broken, "summary.json")), { code: "ENOENT" });
    }
    await assert.rejects(runFlex("chromium", { outputDir: join(root, "launch-error"),
      collect: async () => { throw Error("browser unavailable"); } }), /browser unavailable/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
