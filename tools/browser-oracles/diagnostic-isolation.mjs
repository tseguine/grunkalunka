import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

// Runs in the browser, without captured Node variables. Read layout after each
// write; do not merely compare the fixture's previously serialized observations.
export async function measureDiagnosticIsolation() {
  await document.fonts.ready;
  const output = document.getElementById("output");
  if (!output) throw new Error("missing diagnostic output");
  const originalText = output.textContent, originalHidden = output.hidden;
  const sample = () => ({
    viewport: [innerWidth, innerHeight, devicePixelRatio],
    root: [document.documentElement.clientWidth, document.documentElement.clientHeight,
      document.documentElement.scrollWidth, document.documentElement.scrollHeight],
    body: [document.body.scrollWidth, document.body.scrollHeight],
    scroll: [scrollX, scrollY],
    boxes: ["frame", "block", "inline"].map(id => {
      const r = document.getElementById(id).getBoundingClientRect();
      return [id, r.x, r.y, r.width, r.height];
    })
  });
  try {
    const original = sample(), outputRects = output.getClientRects().length;
    const payload = JSON.parse(originalText);
    output.textContent = "";
    const empty = sample();
    output.textContent = "diagnostic line\n".repeat(4096);
    const enlarged = sample();
    output.textContent = originalText;
    const restored = sample();
    // Positive control: showing large diagnostics must demonstrably add layout.
    output.textContent = "diagnostic line\n".repeat(4096);
    output.hidden = false;
    const visibleControl = sample(), visibleRects = output.getClientRects().length;
    output.hidden = originalHidden;
    output.textContent = originalText;
    const rehidden = sample();
    return { originalHidden, outputRects, visibleRects, original, empty, enlarged,
      restored, visibleControl, rehidden, payloadVersion: payload.schemaVersion,
      textPreserved: output.textContent === originalText };
  } finally {
    output.hidden = originalHidden;
    output.textContent = originalText;
  }
}

export function verifyDiagnosticIsolation(result, viewport) {
  assert.equal(result.originalHidden, true, "diagnostics must start hidden");
  assert.equal(result.outputRects, 0, "hidden diagnostics must generate no boxes");
  assert.equal(result.payloadVersion, 1, "JSON transport remains readable");
  assert.equal(result.textPreserved, true, "diagnostics restored exactly");
  assert.deepEqual(result.original.viewport, [viewport.width, viewport.height, 1]);
  for (const phase of ["empty", "enlarged", "restored", "rehidden"]) {
    assert.deepEqual(result[phase], result.original, `${phase} diagnostics changed layout`);
  }
  assert.ok(result.visibleRects > 0, "visible positive control must generate boxes");
  assert.ok(result.visibleControl.root[3] > result.original.root[3], "visible control must increase scroll height");
}

export async function probeDiagnosticIsolation(browserType, fixturePath) {
  const browser = await browserType.launch({ headless: true, timeout: 30000 });
  const observations = [];
  try {
    for (const viewport of [{ width: 800, height: 600 }, { width: 320, height: 64 }]) {
      const page = await browser.newPage({ viewport, deviceScaleFactor: 1, locale: "en-US",
        colorScheme: "light", reducedMotion: "reduce" });
      try {
        await page.goto(pathToFileURL(resolve(fixturePath)).href, { waitUntil: "load", timeout: 30000 });
        const measurement = await page.evaluate(measureDiagnosticIsolation);
        observations.push({ viewport, measurement });
      } finally { await page.close(); }
    }
    return observations;
  } finally { await browser.close(); }
}
