// Canonical offline browser-reference contract suite, independent of Playwright.
// The same node:test files run in full CI and dependency-selected PR builds.
// Hosted real-engine tests are deliberately excluded from this offline runner.
import { readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const folders = ['tests/browser-oracles', 'tools/browser-oracles'];

export function browserOfflineTestFiles(repositoryRoot = root) {
  const files = [];
  function discover(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const location = join(directory, entry.name);
      if (entry.isDirectory() && !['hosted', 'node_modules'].includes(entry.name)) {
        discover(location);
      }
      if (entry.isFile() && entry.name.endsWith('.test.mjs')) {
        files.push(relative(repositoryRoot, location).split(sep).join('/'));
      }
    }
  }
  for (const folder of folders) discover(join(repositoryRoot, folder));
  return files.sort();
}

export function main() {
  const files = browserOfflineTestFiles();
  if (!files.length) throw new Error('no offline browser-oracle tests found');
  const child = spawnSync(process.execPath, ['--test', ...files], {
    cwd: root,
    stdio: 'inherit',
  });
  if (child.error) throw child.error;
  if (child.signal) throw new Error('browser-oracle tests terminated by ' + child.signal);
  if (child.status !== 0) process.exitCode = child.status ?? 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main();
}
