// Proven Boolean pairs compare without Number coercion.
export const booleanLooseEquality = [
  ['boolean-loose-equality-field', 'const o={flag:true};return o.flag==true&&o.flag!=false?1:0;'],
  ['boolean-loose-equality-comparison', 'const xs=[a<b,false];return xs[0]!=xs[1]?1:0;'],
  ['boolean-loose-equality-not', 'const o={flag:!a};return o.flag==!!b?1:0;'],
  ['boolean-loose-equality-snapshot', 'const o={flag:true};const value=o.flag==(o.flag=false,true);return value&&o.flag==false?1:0;'],
  ['boolean-loose-equality-iteration', 'let n=0;for(const flag of [true,false,true]){if(flag==true)n++;}return n;'],
].map(([id, body]) => [id, body, body]);
