import assert from "node:assert/strict";
import { validateCapture } from "../../tests/browser-oracles/compare.mjs";

const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const properties = [
  "display", "boxSizing", "width", "height", "marginTop", "paddingLeft",
  "borderLeftWidth", "fontSize", "lineHeight", "whiteSpace"
];

// Independent of the fixture's observer and of cross-browser consensus.
// CSS 2.2 10.2: https://www.w3.org/TR/CSS22/visudet.html#the-width-property
// Box sizing: https://www.w3.org/TR/css-sizing-3/#box-sizing
// Border rects: https://drafts.csswg.org/cssom-view/#dom-element-getboundingclientrect
// Parent content = 240 - 2*(12+3) = 210; child content = 50%*210 = 105.
// Child border width = 105 + 2*(5+2) = 119; x/y = 3+12+4 = 19.
export function verifyBaseline(capture, engine, sourceSha256) {
  assert.ok(Object.hasOwn(families, engine), "known engine required");
  assert.match(sourceSha256, /^[a-f0-9]{64}$/, "expected source hash");
  assert.deepEqual(validateCapture(capture), [], "capture schema");
  const p = capture.provenance;
  assert.equal(p?.fixtureSha256, sourceSha256, "source hash mismatch");
  assert.equal(p.collector, "playwright");
  assert.equal(p.engine, engine, "engine identity");
  assert.equal(p.engineFamily, families[engine], "engine family");
  for (const key of ["playwrightVersion", "browserVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]) {
    assert.equal(typeof p[key], "string", key);
    assert.ok(p[key].length > 0 && p[key] !== "unrecorded", key);
  }
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(capture.environment.viewportWidth, 800);
  assert.equal(capture.environment.viewportHeight, 600);
  assert.equal(capture.environment.devicePixelRatio, 1);
  assert.equal(capture.fixtureId, "layout-baseline-v1");
  assert.deepEqual(capture.observations.map(o => o.id), ["frame", "block", "inline"]);
  for (const o of capture.observations) {
    for (const key of properties) {
      assert.ok(Object.hasOwn(o.computed, key), `missing ${o.id}.${key}`);
      assert.notEqual(o.computed[key], "", `empty ${o.id}.${key}`);
    }
  }
  const [frame, block, inline] = capture.observations;
  assert.equal(frame.text, "abcA B");
  assert.equal(block.text, "abc");
  assert.equal(inline.text, "A B");
  for (const [key, value] of Object.entries({ x: 0, y: 0, width: 240 })) {
    assert.equal(frame.rect[key], value, `frame.rect.${key}`);
  }
  assert.deepEqual(block.rect, { x: 19, y: 19, width: 119, height: 34 });
  assert.equal(frame.computed.boxSizing, "border-box");
  assert.equal(block.computed.boxSizing, "content-box");
  assert.equal(block.computed.width, "105px");
  assert.equal(block.computed.height, "20px");
}
