import assert from 'node:assert/strict';

// Metadata supplies complete source text; no normalization or wrapping follows.
export function programSources([id, bodyJS, bodyTS, , oracleBody, metadata]) {
  assert.equal(typeof bodyJS, 'string', `${id}: JS body source`);
  assert.equal(typeof bodyTS, 'string', `${id}: TS body source`);
  if (oracleBody !== undefined) assert.equal(typeof oracleBody, 'string', `${id}: oracle body source`);
  if (metadata !== undefined) {
    assert.ok(metadata !== null && typeof metadata === 'object' && !Array.isArray(metadata), `${id}: source metadata`);
    assert.ok(Object.keys(metadata).every(key => ['sourceJS', 'sourceTS', 'oracleSource'].includes(key)), `${id}: unknown source metadata`);
    assert.equal(typeof metadata.sourceJS, 'string', `${id}: full JS source`);
    assert.equal(typeof metadata.sourceTS, 'string', `${id}: full TS source`);
    if (metadata.oracleSource !== undefined) {
      assert.equal(typeof metadata.oracleSource, 'string', `${id}: full oracle source`);
      assert.equal(oracleBody, undefined, `${id}: ambiguous oracle sources`);
    }
  }
  const wrap = body => `export function run(a, b) { ${body} }\n`;
  const sourceJS = metadata === undefined ? wrap(bodyJS) : metadata.sourceJS;
  const sourceTS = metadata === undefined
    ? `export function run(a: number, b: number): number { ${bodyTS} }\n` : metadata.sourceTS;
  const oracleSource = oracleBody === undefined ? (metadata?.oracleSource ?? sourceJS) : wrap(oracleBody);
  return { sourceJS, sourceTS, oracleSource };
}
