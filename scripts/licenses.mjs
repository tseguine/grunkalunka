// Metadata screening for locked workspace/research graphs, not a redistribution audit.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
// Exact reviewed expressions. An unfamiliar expression or missing license fails.
// Legacy slash alternatives were checked against their bundled license files.
const choices = new Map([
  ['MIT', 'MIT'], ['MIT OR Apache-2.0', 'MIT'], ['Apache-2.0 OR MIT', 'MIT'],
  ['MIT/Apache-2.0', 'MIT'], ['Apache-2.0/MIT', 'MIT'], ['Apache-2.0 / MIT', 'MIT'],
  ['Apache-2.0 WITH LLVM-exception', 'Apache-2.0 WITH LLVM-exception'],
  ['Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT', 'MIT'],
  ['Apache-2.0 WITH LLVM-exception OR BSL-1.0', 'Apache-2.0 WITH LLVM-exception'],
  ['Apache-2.0 OR GPL-2.0-only', 'Apache-2.0'], // self_cell's explicit alternative
  ['(MIT OR Apache-2.0) AND Unicode-3.0', 'MIT AND Unicode-3.0'],
  ['BSD-2-Clause OR MIT OR Apache-2.0', 'MIT'],
  ['Unlicense OR MIT', 'MIT'], ['Unicode-3.0', 'Unicode-3.0'], ['Zlib', 'Zlib'],
]);
const all = new Map();
for (const [label, manifest] of [['workspace', 'Cargo.toml'], ['frontend-research', 'spikes/001-frontend-semantics/Cargo.toml']]) {
  const result = spawnSync('cargo', ['metadata', '--manifest-path', manifest, '--locked', '--offline', '--format-version', '1'],
    { cwd: root, encoding: 'utf8', shell: false, timeout: 30_000, maxBuffer: 16 * 1024 * 1024 });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr);
  const packages = JSON.parse(result.stdout).packages.map(pkg => {
    const selected = choices.get(pkg.license);
    assert.ok(selected, `${pkg.name}@${pkg.version}: unreviewed license ${pkg.license}`);
    assert.doesNotMatch(selected, /GPL/);
    const record = { name: pkg.name, version: pkg.version, declared: pkg.license, selected, source: pkg.source };
    all.set(`${pkg.name}@${pkg.version}`, record);
    return record;
  }).sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  const destination = join(root, dirname(manifest), 'target', 'license-inventory.json');
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(destination, JSON.stringify({ manifest, packages }, null, 2) + '\n');
  console.log(`PASS licenses ${label}: ${packages.length} packages; no selected GPL/LGPL license`);
}
console.log(`PASS ${all.size} distinct package versions screened (all target/build packages returned by Cargo metadata)`);
