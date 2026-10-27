import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import test from "node:test";
import { collectOne, parseFlags } from "./collect.mjs";

function observation() {
  return {
    schemaVersion: 1,
    fixtureId: "layout-baseline-v1",
    environment: {
      userAgent: "Mock Chromium",
      viewportWidth: 800,
      viewportHeight: 600,
      devicePixelRatio: 1,
      url: "file:///some/layout-baseline.html",
      timestamp: "2026-10-27T09:00:00Z"
    },
    observations: [{
      id: "frame",
      text: "abc",
      computed: { width: "240px" },
      rect: { x: 0, y: 0, width: 240, height: 40 }
    }]
  };
}

function fakeTypes(payload) {
  const events = [];
  const browser = {
    version: () => "148.0.mock",
    newPage: async config => {
      events.push({ type: "newPage", config });
      return {
        goto: async (url, options) => events.push({ type: "goto", url, options }),
        locator: selector => {
          assert.equal(selector, "#output");
          return { textContent: async () => typeof payload === "function" ? payload() : JSON.stringify(payload) };
        }
      };
    },
    close: async () => events.push({ type: "close" })
  };
  return {
    browserTypes: { chromium: { launch: async opts => {
      events.push({ type: "launch", opts });
      return browser;
    } } },
    events
  };
}

test("parses exact engines and finite viewport controls", () => {
  assert.deepEqual(parseFlags([]).engines, ["chromium", "firefox", "webkit"]);
  assert.deepEqual(parseFlags(["--engine=firefox", "--width=1024", "--height=768", "--dpr=2"]),
    { engines: ["firefox"], fixturePath: null, outputDir: null, width: 1024, height: 768, devicePixelRatio: 2 });
  for (const arg of ["--engine=blink", "--width=0", "--dpr=NaN", "--height=2.5", "--bogus=x", "--fixture="]) {
    assert.throws(() => parseFlags([arg]), Error, arg);
  }
  assert.throws(() => parseFlags(["--engine=webkit", "--engine=firefox"]), /duplicate option/);
});

test("collects browser-native output without normalizing semantics or engine internals", async () => {
  const dir = mkdtempSync(join(tmpdir(), "grunkalunka-collector-"));
  const fixturePath = join(dir, "fixture.html");
  const bytes = "<!doctype html><title>fixture</title>";
  writeFileSync(fixturePath, bytes);
  try {
    const mock = fakeTypes(observation());
    const capture = await collectOne("chromium", {
      browserTypes: mock.browserTypes,
      fixturePath,
      playwrightVersion: "1.64.1"
    });
    assert.equal(capture.fixtureId, "layout-baseline-v1");
    assert.equal(capture.observations[0].computed.width, "240px");
    assert.equal(capture.provenance.engineFamily, "blink");
    assert.equal(capture.provenance.browserVersion, "148.0.mock");
    assert.equal(capture.provenance.playwrightVersion, "1.64.1");
    assert.equal(capture.provenance.fixtureSha256, createHash("sha256").update(bytes).digest("hex"));
    assert.equal(mock.events[0].type, "launch");
    assert.deepEqual(mock.events[1].config.viewport, { width: 800, height: 600 });
    assert.equal(mock.events[1].config.deviceScaleFactor, 1);
    assert.equal(mock.events[2].url, pathToFileURL(fixturePath).href);
    assert.equal(mock.events[2].options.waitUntil, "load");
    assert.equal(mock.events.at(-1).type, "close");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rejects incompatible viewport reports and always closes browser", async () => {
  const dir = mkdtempSync(join(tmpdir(), "grunkalunka-collector-"));
  const fixturePath = join(dir, "fixture.html");
  writeFileSync(fixturePath, "fixture");
  try {
    const mismatch = observation();
    mismatch.environment.viewportWidth += 1;
    const mock = fakeTypes(mismatch);
    await assert.rejects(() => collectOne("chromium", { browserTypes: mock.browserTypes, fixturePath }),
      /different CSS viewport/);
    assert.equal(mock.events.at(-1).type, "close");

    const invalid = fakeTypes({ observations: [] });
    await assert.rejects(() => collectOne("chromium", { browserTypes: invalid.browserTypes, fixturePath }),
      /unsupported or empty/);
    assert.equal(invalid.events.at(-1).type, "close");

    const broken = fakeTypes(() => { throw new Error("bad page"); });
    await assert.rejects(() => collectOne("chromium", { browserTypes: broken.browserTypes, fixturePath }),
      /bad page/);
    assert.equal(broken.events.at(-1).type, "close");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rejects an unsupported engine before trying to open a browser", async () => {
  await assert.rejects(() => collectOne("blink", { browserTypes: {}, fixturePath: "x" }), /unsupported engine/);
});
