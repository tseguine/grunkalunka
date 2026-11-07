// Boolean storage projects its actual Boolean tag before scalar coercion.
export const booleanProjection = [
  ['boolean-projection-field', 'const o={flag:a<b};return +o.flag;'],
  ['boolean-projection-elements', 'const xs=[true,false];return +xs[0]*10+ +xs[1];'],
  ['boolean-projection-negative-zero', 'const o={flag:false};return 1/-o.flag;'],
  ['boolean-projection-snapshot', 'const o={flag:true};const value=(+o.flag)+(o.flag=false,+o.flag);return value;'],
].map(([id, body]) => [id, body, body]);
