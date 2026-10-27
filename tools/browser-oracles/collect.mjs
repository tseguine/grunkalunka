import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createRequire } from "node:module";
import { arch, platform } from "node:os";
import { resolve, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const engineFamilies = Object.freeze({
  chromium: "blink",
  firefox: "gecko",
  webkit: "webkit"
});

export function parseFlags(args) {
  const values = {
    engines: ["chromium", "firefox", "webkit"],
    fixturePath: null,
    outputDir: null,
    width: 800,
    height: 600,
    devicePixelRatio: 1
  };
  const keys = new Set();
  for (const arg of args) {
    const sep = arg.indexOf("=");
    const key = sep < 0 ? arg : arg.slice(0, sep);
    const raw = sep < 0 ? null : arg.slice(sep + 1);
    const recognized = ["--engine", "--fixture", "--output-dir", "--width", "--height", "--dpr"];
    if (!recognized.includes(key) || raw === null || raw.length === 0) {
      throw new Error(`invalid option ${JSON.stringify(arg)}`);
    }
    if (keys.has(key)) throw new Error(`duplicate option ${key}`);
    keys.add(key);
    if (key === "--engine") {
      if (!["all", "chromium", "firefox", "webkit"].includes(raw)) {
        throw new Error("engine must be all, chromium, firefox or webkit");
      }
      values.engines = raw === "all" ? ["chromium", "firefox", "webkit"] : [raw];
    } else if (key === "--fixture") {
      values.fixturePath = raw;
    } else if (key === "--output-dir") {
      values.outputDir = raw;
    } else {
      const number = Number(raw);
      if (!Number.isFinite(number) || number <= 0) {
        throw new Error(`${key} must be a finite positive number`);
      }
      if (key === "--width" || key === "--height") {
        if (!Number.isSafeInteger(number)) throw new Error(`${key} must be a positive integer`);
        values[key === "--width" ? "width" : "height"] = number;
      } else {
        values.devicePixelRatio = number;
      }
    }
  }
  return values;
}

/**
 * Capture one reference engine's native observations of the exact local HTML
 * fixture. browserTypes can be injected for offline unit tests.
 *
 * Returned metadata does NOT make the browser output a standards oracle.
 */
export async function collectOne(engine, options) {
  if (!Object.hasOwn(engineFamilies, engine)) {
    throw new Error(`unsupported engine ${engine}`);
  }
  const {
    browserTypes,
    fixturePath,
    width = 800,
    height = 600,
    devicePixelRatio = 1,
    playwrightVersion = "unrecorded"
  } = options;
  if (!browserTypes?.[engine]) throw new Error(`missing browserType ${engine}`);
  if (typeof fixturePath !== "string" || fixturePath.length === 0) {
    throw new Error("fixturePath is required");
  }
  const path = resolve(fixturePath);
  const content = await readFile(path);
  const fixtureSha256 = createHash("sha256").update(content).digest("hex");

  const browser = await browserTypes[engine].launch({ headless: true });
  try {
    const page = await browser.newPage({
      viewport: { width, height },
      deviceScaleFactor: devicePixelRatio,
      locale: "en-US",
      colorScheme: "light",
      reducedMotion: "reduce"
    });
    await page.goto(pathToFileURL(path).href, { waitUntil: "load", timeout: 30000 });
    const raw = await page.locator("#output").textContent({ timeout: 10000 });
    if (typeof raw !== "string") throw new Error("fixture produced no observation JSON");
    const capture = JSON.parse(raw);
    if (capture?.schemaVersion !== 1 ||
        typeof capture?.fixtureId !== "string" ||
        !Array.isArray(capture?.observations) ||
        capture.observations.length === 0) {
      throw new Error("fixture emitted an unsupported or empty observation payload");
    }
    if (capture.environment?.viewportWidth !== width ||
        capture.environment?.viewportHeight !== height ||
        capture.environment?.devicePixelRatio !== devicePixelRatio) {
      throw new Error("browser reported a different CSS viewport or device scale");
    }
    return {
      ...capture,
      provenance: {
        collector: "playwright",
        playwrightVersion,
        engine,
        engineFamily: engineFamilies[engine],
        browserVersion: browser.version(),
        fixtureSha256,
        hostPlatform: platform(),
        hostArchitecture: arch(),
        nodeVersion: process.version,
        viewport: { width, height, devicePixelRatio }
      }
    };
  } finally {
    await browser.close();
  }
}

export async function main(args) {
  const flags = parseFlags(args);
  const fixturePath = resolve(flags.fixturePath ??
    fileURLToPath(new URL("../../tests/browser-oracles/layout-baseline.html", import.meta.url)));
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const outDir = resolve(flags.outputDir ??
    fileURLToPath(new URL(`../../target/browser-oracles/run-${timestamp}/`, import.meta.url)));
  // Browser automation is optional: no playwright import or download on ordinary builds.
  let browserTypes;
  try {
    browserTypes = await import("playwright");
  } catch (cause) {
    throw new Error("Install optional browser dependencies with npm --prefix tools/browser-oracles install, then npm --prefix tools/browser-oracles exec -- playwright install chromium firefox webkit", { cause });
  }
  const require = createRequire(import.meta.url);
  const packageFile = require.resolve("playwright/package.json");
  const playwrightVersion = JSON.parse(await readFile(packageFile, "utf8")).version;
  await mkdir(outDir, { recursive: true });
  const results = [];
  for (const engine of flags.engines) {
    try {
      const capture = await collectOne(engine, { ...flags, fixturePath, playwrightVersion, browserTypes });
      const filename = join(outDir, `${engine}.json`);
      await writeFile(filename, JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
      results.push({ engine, status: "captured", filename, browserVersion: capture.provenance.browserVersion });
    } catch (error) {
      results.push({ engine, status: "error", message: String(error) });
      process.exitCode = 1;
    }
  }
  process.stdout.write(JSON.stringify({ fixturePath, outDir, results }, null, 2) + "\n");
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
