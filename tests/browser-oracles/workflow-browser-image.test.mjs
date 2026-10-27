import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = await readFile(new URL("../../.github/workflows/browser-smoke.yml", import.meta.url), "utf8");
const pkg = JSON.parse(await readFile(new URL("../../tools/browser-oracles/package.json", import.meta.url), "utf8"));

function verifyImageContract(text) {
  const image = text.match(/^  smoke:[\s\S]*?^    container:\n      image: mcr\.microsoft\.com\/playwright:v(\d+\.\d+\.\d+)-noble\n      options: --user 1001$/m);
  assert.ok(image, "smoke uses the documented Playwright image as an unprivileged user");
  assert.equal(image[1], pkg.dependencies.playwright, "image and npm Playwright versions match exactly");
  const smoke = text.split("\n  cross-engine:\n")[0];
  assert.doesNotMatch(smoke, /playwright\/cli\.js install(?:\s|$)|install --with-deps/, "no apt-based browser provisioning");
  assert.match(smoke, /- name: Verify version-matched browser image\n/, "verify image path at runtime");
  assert.match(smoke, /test "\$\{PLAYWRIGHT_BROWSERS_PATH:-\}" = \/ms-playwright/, "verify installed path");
  assert.match(smoke, /node --test [^\n]*workflow-browser-image\.test\.mjs/, "regression runs on workflow-only edits");
}

test("browser CI consumes preprovisioned version-matched image without apt", () => {
  verifyImageContract(workflow);
});

test("mismatched image, missing nonroot policy, and restored apt install are rejected", () => {
  for (const mutated of [
    workflow.replace("playwright:v1.64.1-noble", "playwright:v1.63.0-noble"),
    workflow.replace("options: --user 1001", "options: --user 0"),
    workflow.replace("    container:\n", "    no-container:\n"),
    workflow.replace("Verify version-matched browser image", "Remove version verification"),
    workflow.replace("      - name: Record font and host inventory",
      "      - run: node tools/browser-oracles/node_modules/playwright/cli.js install --with-deps firefox\n      - name: Record font and host inventory")
  ]) assert.throws(() => verifyImageContract(mutated));
});
