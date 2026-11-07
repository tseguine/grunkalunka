// Boolean-only logical writes preserve the cell kind without predicting selection.
export const booleanLogicalOrigins = [
  ['boolean-logical-origin-and', 'let [flag]=[true];const value=flag&&=false;return value==false&&flag==false?1:0;'],
  ['boolean-logical-origin-or', 'let {flag}={flag:false};return +(flag||=true);'],
  ['boolean-logical-origin-nullish', 'let [flag]=[false];let n=0;const value=flag??=(n++,true);return value==false&&flag==false&&n===0?1:0;'],
  ['boolean-logical-origin-capture', 'let [flag]=[false];const read=()=>flag;flag||=true;return read()==true?1:0;'],
  ['boolean-logical-origin-loop', 'let [flag]=[true];let count=0;while(flag){count++;flag&&=false;}return count+ +flag;'],
].map(([id, body]) => [id, body, body]);
