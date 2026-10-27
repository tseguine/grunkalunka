import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { compareCaptures } from "../../tests/browser-oracles/compare.mjs";
import { collectOne } from "./collect.mjs";
import { verifyBaseline } from "./verify-baseline.mjs";
import { runCascade } from "./cascade.mjs";
import { runEvents } from "./events.mjs";
import { runLifecycle } from "./listener-lifecycle.mjs";
import { runFlex } from "./flex.mjs";
import { runWrap } from "./flex-wrap-lines.mjs";
import { probeDiagnosticIsolation, verifyDiagnosticIsolation } from "./diagnostic-isolation.mjs";

const hash = bytes => createHash("sha256").update(bytes).digest("hex");

export async function runSmoke(engine, { browserTypes, playwrightVersion, outputDir }) {
  assert.ok(["chromium", "firefox", "webkit"].includes(engine), "unknown engine");
  const source = await readFile(new URL("../../tests/browser-oracles/layout-baseline.html", import.meta.url));
  const sourceHash = hash(source);
  await mkdir(dirname(resolve(outputDir)), { recursive: true });
  await mkdir(outputDir); // A run must not mix new evidence with an old directory.
  const save = (name, value) => writeFile(join(outputDir, name), JSON.stringify(value, null, 2) + "\n", { flag: "wx" });
  const snapshot = join(outputDir, "snapshot.html");
  await writeFile(snapshot, source, { flag: "wx", mode: 0o444 });
  const take = async (fixturePath, name) => {
    const capture = await collectOne(engine, { browserTypes, playwrightVersion, fixturePath });
    await save(name, capture); // Retain even observations that fail verification.
    return capture;
  };
  const first = await take(snapshot, "first.json");
  verifyBaseline(first, engine, sourceHash);
  const repeat = await take(snapshot, "repeat.json");
  verifyBaseline(repeat, engine, sourceHash);
  const stable = compareCaptures(first, repeat);
  await save("repeat-comparison.json", stable);
  assert.equal(stable.status, "equal", "fresh-context repeat must be stable");

  const text = source.toString("utf8"), needle = "width: 50%;";
  assert.equal(text.split(needle).length, 2, "exactly one mutation site required");
  const changed = text.replace(needle, "width: 60%;");
  const mutationPath = join(outputDir, "mutation.html");
  await writeFile(mutationPath, changed, { flag: "wx", mode: 0o444 });
  const mutated = await take(mutationPath, "mutated.json");
  assert.equal(mutated.provenance.fixtureSha256, hash(changed));
  assert.throws(() => verifyBaseline(mutated, engine, sourceHash), /source hash mismatch/);
  assert.throws(() => verifyBaseline(mutated, engine, hash(changed))); // Independent geometry also rejects it.
  assert.equal(mutated.observations[1].rect.width, 140);
  assert.equal(mutated.observations[1].computed.width, "126px");
  const sensitivity = compareCaptures(first, mutated);
  await save("intentional-source-mutation.json", sensitivity);
  assert.equal(sensitivity.status, "different", "real source mutation must be detected");
  const isolation = await probeDiagnosticIsolation(browserTypes[engine], snapshot);
  await save("diagnostic-isolation.json", { engine, sourceHash, browserVersion: first.provenance.browserVersion, observations: isolation });
  for (const { viewport, measurement } of isolation) verifyDiagnosticIsolation(measurement, viewport);
  await runCascade(engine, { outputDir: join(outputDir, "cascade"),
    collect: fixturePath => collectOne(engine, { browserTypes, playwrightVersion, fixturePath }) });
  await runEvents(engine, { outputDir: join(outputDir, "events"),
    collect: fixturePath => collectOne(engine, { browserTypes, playwrightVersion, fixturePath }) });
  await runLifecycle(engine, { outputDir: join(outputDir, "listeners"),
    collect: fixturePath => collectOne(engine, { browserTypes, playwrightVersion, fixturePath }) });
  await runFlex(engine, { outputDir: join(outputDir, "flex"),
    collect: fixturePath => collectOne(engine, { browserTypes, playwrightVersion, fixturePath }) });
  await runWrap(engine, { outputDir: join(outputDir, "wrap-lines"),
    collect: fixturePath => collectOne(engine, { browserTypes, playwrightVersion, fixturePath }) });
  const summary = { engine, browserVersion: first.provenance.browserVersion, sourceHash,
    repeat: stable.status, mutation: sensitivity.status, selectedBaseline: "passed", diagnosticIsolation: "passed", cascade: "passed", events: "passed", listeners: "passed", flex: "passed", wrapLines: "passed" };
  await save("summary.json", summary);
  return summary;
}

async function main(args) {
  assert.equal(args.length, 2, "usage: node tools/browser-oracles/smoke.mjs ENGINE NEW_OUTPUT_DIR");
  const browserTypes = await import("playwright");
  const require = createRequire(import.meta.url);
  const playwrightVersion = require("playwright/package.json").version;
  const summary = await runSmoke(args[0], { browserTypes, playwrightVersion, outputDir: resolve(args[1]) });
  process.stdout.write(JSON.stringify(summary, null, 2) + "\n");
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    console.error(error);
    process.exitCode = 1;
  });
}
