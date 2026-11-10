// The compiler integration test consumes the same sources as the full CLI replay.
import { rejected } from './corpus.mjs';
import { valueRejected } from './primitive-values.mjs';

const inputs = rejected.map(([id, source, language = 'js', diagnostic]) =>
  ({ id, source, language, diagnostic, profile: 'numeric' }));
for (const { id, sourceJS, sourceTS, diagnostic } of valueRejected) {
  inputs.push({ id, source: sourceJS, language: 'js', diagnostic, profile: 'value' },
    { id, source: sourceTS, language: 'ts', diagnostic, profile: 'value' });
}
// Match the UTF-8 files written by the CLI replay for lone surrogate code units.
for (const fixture of inputs) fixture.source = fixture.source.toWellFormed();
process.stdout.write(JSON.stringify(inputs));
