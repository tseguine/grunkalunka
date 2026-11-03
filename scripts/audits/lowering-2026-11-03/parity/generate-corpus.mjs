import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [repo, output] = process.argv.slice(2);
if (!repo || !output) throw new Error('usage: generate-corpus.mjs REPO OUTPUT');
const base = pathToFileURL(join(repo, 'tests/compatibility/'));
const { programs, rejected } = await import(new URL('corpus.mjs', base));
const { programSources } = await import(new URL('program-sources.mjs', base));
const { valuePrograms, valueRejected, valueFuelCases } = await import(new URL('primitive-values.mjs', base));
await mkdir(output, { recursive: true });
const rows = [];
const index = [];
async function add(id, profile, language, expected, source) {
  const file = join(output, `${id}.${language}`);
  await writeFile(file, source);
  rows.push([id, profile, language, expected, file].join('\t'));
  index.push({id, profile, language, expected, file});
}
for (const program of programs) {
  const [id] = program;
  const sources = programSources(program);
  if (id === 'affine') {
    sources.sourceJS = await readFile(new URL('fixtures/affine.js', base), 'utf8');
    sources.sourceTS = await readFile(new URL('fixtures/affine.ts', base), 'utf8');
  }
  await add(`numeric-ok-${id}-js`, 'numeric', 'js', 'ok', sources.sourceJS);
  await add(`numeric-ok-${id}-ts`, 'numeric', 'ts', 'ok', sources.sourceTS);
}
for (const [id, source, language = 'js'] of rejected) {
  await add(`numeric-error-${id}-${language}`, 'numeric', language, 'error', source);
}
for (const [fixtures, expected, group] of [[valuePrograms, 'ok', 'ok'], [valueRejected, 'error', 'error'], [valueFuelCases, 'ok', 'fuel']]) {
  for (const fixture of fixtures) {
    await add(`value-${group}-${fixture.id}-js`, 'value', 'js', expected, fixture.sourceJS);
    await add(`value-${group}-${fixture.id}-ts`, 'value', 'ts', expected, fixture.sourceTS);
  }
}
await writeFile(join(output, 'manifest.tsv'), `${rows.join('\n')}\n`);
await writeFile(join(output, 'manifest.json'), `${JSON.stringify(index, null, 2)}\n`);
console.log(JSON.stringify({ cases: rows.length, numericAdmitted: programs.length * 2, numericRejected: rejected.length, primitiveAdmitted: valuePrograms.length * 2, primitiveRejected: valueRejected.length * 2, primitiveFuel: valueFuelCases.length * 2 }));
