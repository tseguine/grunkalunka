import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = (await readFile(new URL("../../.github/workflows/browser-smoke.yml", import.meta.url), "utf8")).replace(/\r\n/g, "\n");
const pkg = JSON.parse(await readFile(new URL("../../tools/browser-oracles/package.json", import.meta.url), "utf8"));

function verifyImageContract(text) {
  const image = text.match(/^  smoke:[\s\S]*?^    container:\n      image: mcr\.microsoft\.com\/playwright:v(\d+\.\d+\.\d+)-noble@sha256:([a-f0-9]{64})\n      options: --user 1001$/m);
  assert.ok(image, "smoke uses the documented Playwright image as an unprivileged user");
  assert.equal(image[1], pkg.dependencies.playwright, "image and npm Playwright versions match exactly");
  assert.equal(image[2], "47b510dd9230bf0b9bfc9b20102241e577ea7a99334ab4c3ca19f19770e9b8ea", "image digest matches the tested three-engine container");
  const smoke = text.split("\n  cross-engine:\n")[0];
  assert.doesNotMatch(smoke, /playwright\/cli\.js install(?:\s|$)|install --with-deps/, "no apt-based browser provisioning");
  assert.ok(smoke.includes("node --test tools/browser-oracles/hosted/smoke.test.mjs"),
    "live test suite, not inline YAML assertions");
  const hosted = readFileSync(new URL("../../tools/browser-oracles/hosted/smoke.test.mjs",
    import.meta.url), "utf8");
  assert.ok(hosted.includes("process.env.PLAYWRIGHT_BROWSERS_PATH"));
  assert.ok(hosted.includes("require('playwright/package.json')"));
}

test("browser CI consumes preprovisioned version-matched image without apt", () => {
  verifyImageContract(workflow);
});

test("mismatched image, missing nonroot policy, and restored apt install are rejected", () => {
  for (const mutated of [
    workflow.replace("playwright:v1.64.1-noble", "playwright:v1.63.0-noble"),
    workflow.replace("options: --user 1001", "options: --user 0"),
    workflow.replace("@sha256:47b510dd9230bf0b9bfc9b20102241e577ea7a99334ab4c3ca19f19770e9b8ea", ""),
    workflow.replace("@sha256:47b510dd9230bf0b9bfc9b20102241e577ea7a99334ab4c3ca19f19770e9b8ea", "@sha256:" + "0".repeat(64)),
    workflow.replace("    container:\n", "    no-container:\n"),
    workflow.replace("node --test tools/browser-oracles/hosted/smoke.test.mjs", "node incomplete.mjs"),
    workflow.replace("      - name: Record font and host inventory",
      "      - run: node tools/browser-oracles/node_modules/playwright/cli.js install --with-deps firefox\n      - name: Record font and host inventory")
  ]) assert.throws(() => verifyImageContract(mutated));
});
