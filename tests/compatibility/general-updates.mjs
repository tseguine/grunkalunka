// ECMA-262 13.4 update expressions and 7.1.3 ToNumeric, four primitive kinds.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-update-expressions
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumeric
const rows=[
 ['null-post','let value=null; return value++;'],
 ['undefined-pre','let value=undefined; return ++value;'],
 ['false-post','let value=false; return value--;'],
 ['true-pre','let value=true; return --value;'],
 ['store','let value=null; const old=value++; return old*10+(+value);'],
 ['alias','let value=null; const saved=value; value++; return saved===null?+value:99;'],
 ['rewrite','let value=null; const old=value++; value=false; return old+(value==false?10:0);'],
 ['twice','let value=null; return value++ + value++;'],
 ['pre-pair','let value=false; return ++value + --value;'],
 ['loop','let value=null; for(let i=0;i<3;i++){value++;} return +value;'],
 ['while','let value=false; while((+value)<3){value++;} return +value;'],
 ['selected','let value=null; return a?value++:++value;'],
 ['parameter','a=null; return a++ + (+a);'],
 ['parameter-decrement','a=false; const old=--a; return old+(+a);'],
 ['input-number','let value=null; value=a; return value++ + b;'],
 ['sequence','let value=null; const old=(value++,value--); return old*10+(+value);'],
 ['nan','let value=null; value=NaN; return value++;'],
 ['signed-zero','let value=null; value=-0; return value++;'],
 ['helper','let value=a?null:false; return ++value + b;'],
 ['type-wrapper','let value=null; return value++;','let value:number=null; return (value as number)++;'],
];
export const generalUpdates=rows.map(([id,js,ts=js])=>['general-update-'+id,js,ts]);
