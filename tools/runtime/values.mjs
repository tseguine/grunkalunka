/**
 * Decode the primitive-return-v1 (i32, f64) result described by ADR 0002.
 * No coercion, imports, instantiation or resource management is performed.
 * @param {unknown} result The two-element Array returned by a Wasm export.
 * @returns {undefined|null|boolean|number}
 * @throws {TypeError} For a malformed pair, unknown tag or noncanonical payload.
 */
export function decodePrimitiveResult(result) {
  if (!Array.isArray(result) || result.length !== 2
      || !Object.hasOwn(result, 0) || !Object.hasOwn(result, 1)) {
    throw new TypeError('Expected a two-element primitive result array');
  }
  const tag = result[0];
  const payload = result[1];
  if (typeof payload !== 'number' || Object.is(tag, -0)) {
    throw new TypeError('Invalid primitive result tag or payload');
  }
  switch (tag) {
    case 0:
    case 1:
      if (!Object.is(payload, 0)) throw new TypeError('Nullish payload must be positive zero');
      return tag === 0 ? undefined : null;
    case 2:
      if (Object.is(payload, 0)) return false;
      if (payload === 1) return true;
      throw new TypeError('Boolean payload must be positive zero or one');
    case 3:
      return payload;
    default:
      throw new TypeError('Unknown primitive result tag');
  }
}
