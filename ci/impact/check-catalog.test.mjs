import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { loadCanonicalCommands } from '../../scripts/check-catalog.mjs';
import { verifyCanonicalChecks } from './canonical.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const catalogDirectory = fileURLToPath(new URL('../../scripts/checks/', import.meta.url));
const descriptorDirectory = fileURLToPath(new URL('./checks/', import.meta.url));

test('canonical manifest commands match the independent impact descriptors', () => {
  const names = readdirSync(catalogDirectory)
    .filter(name => name.endsWith('.json'))
    .map(name => name.replace(/^\d{6}-/, '').replace(/\.json$/, ''));
  const descriptors = readdirSync(descriptorDirectory)
    .filter(name => name.endsWith('.json'))
    .map(name => JSON.parse(readFileSync(join(descriptorDirectory, name), 'utf8')));
  assert.equal(new Set(names).size, names.length, 'no duplicate logical check names');
  assert.deepEqual(names.slice().sort(), descriptors.map(d => d.id.slice('check:'.length)).sort());
  const actual = loadCanonicalCommands();
  assert.deepEqual(verifyCanonicalChecks(actual, descriptors),
    { commands: actual.length, mapped: actual.length });
});

test('actual canonical inventory capture preserves manifest order', () => {
  const captured = JSON.parse(execFileSync(process.execPath, ['ci/impact/collect-checks.mjs'],
    { cwd: root, encoding: 'utf8', timeout: 10000 }));
  assert.deepEqual(captured, loadCanonicalCommands());
});

function temporaryCatalog(t) {
  const directory = mkdtempSync(join(tmpdir(), 'grunkalunka-check-catalog-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const put = (file, data) => writeFileSync(join(directory, file), data);
  return { directory, put };
}

test('manifest catalog is sorted and supports sparse insertion positions', t => {
  const { directory, put } = temporaryCatalog(t);
  put('009000-b.json', JSON.stringify({ command: 'cargo', args: ['test'] }));
  put('001000-a.json', JSON.stringify({ command: 'node', args: ['--test', 'a.test.mjs'] }));
  put('README.md', 'Documentation is not a check.');
  assert.deepEqual(loadCanonicalCommands(directory), [
    { command: 'node', args: ['--test', 'a.test.mjs'] },
    { command: 'cargo', args: ['test'] },
  ]);
});

test('missing, malformed, duplicate or unexpectedly named catalog entries fail closed', t => {
  const { directory, put } = temporaryCatalog(t);
  assert.throws(() => loadCanonicalCommands(directory), /empty/);
  put('001000-one.json', JSON.stringify({ command: 'node', args: ['one.mjs'] }));
  assert.equal(loadCanonicalCommands(directory).length, 1);
  put('002000-two.json', JSON.stringify({ command: 'node', args: ['one.mjs'] }));
  assert.throws(() => loadCanonicalCommands(directory), /duplicate/);
  put('002000-two.json', JSON.stringify({ command: 'shell', args: ['echo', 'hi'] }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: [] }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: ['test'], extra: true }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', 'not-json');
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check manifest/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: ['test'] }));
  put('unexpected.mjs', '// unexpected code');
  assert.throws(() => loadCanonicalCommands(directory), /unsupported canonical-check catalog entry/);
});
