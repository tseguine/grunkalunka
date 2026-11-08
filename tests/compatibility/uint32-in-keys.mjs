export const uint32InKeyPrograms=[
 ["uint32-in-key-array-holes","const xs=[a,,b];return (a>>>0) in xs?1:0;","const xs=[a,,b];return (a>>>0) in xs?1:0;"],
 ["uint32-in-key-object-undefined","const o={0:undefined,1:b};return (a>>>0) in o?1:0;","const o={0:undefined,1:b};return (a>>>0) in o?1:0;"],
 ["uint32-in-key-dictionary","const d=Object.create(null);d[0]=a;return (a>>>b) in d?1:0;","const d=Object.create(null);d[0]=a;return (a>>>b) in d?1:0;"],
 ["uint32-in-key-mixed-prototype","const o=a?{}:Object.create(null);o[0]=b;return (0>>>0) in o?1:0;","const o=a?{}:Object.create(null);o[0]=b;return (0>>>0) in o?1:0;"],
 ["uint32-in-key-max-named","const xs=[a];xs[4294967295]=b;return ((-1)>>>0) in xs&&xs.length===1?1:0;","const xs=[a];xs[4294967295]=b;return ((-1)>>>0) in xs&&xs.length===1?1:0;"],
 ["uint32-in-key-last-index","const xs=[a];xs[4294967294]=b;return ((-2)>>>0) in xs&&xs.length===4294967295?1:0;","const xs=[a];xs[4294967294]=b;return ((-2)>>>0) in xs&&xs.length===4294967295?1:0;"],
 ["uint32-in-key-effects","let key=0,n=0;const xs=[a];const found=(n=1,key>>>0) in (n=n*10+2,key=1,xs);return found&&n===12&&key===1?1:0;","let key=0,n=0;const xs=[a];const found=(n=1,key>>>0) in (n=n*10+2,key=1,xs);return found&&n===12&&key===1?1:0;"],
 ["uint32-in-key-snapshot","let key=a;const found=(key>>>0) in (key=b,{0:a});return found===((a>>>0)===0)?1:0;","let key=a;const found=(key>>>0) in (key=b,{0:a});return found===((a>>>0)===0)?1:0;"],
 ["uint32-in-key-after-delete","const xs=[a,b];const before=(a>>>0) in xs;delete xs[a>>>0];return before?((a>>>0) in xs?-1:1):0;","const xs=[a,b];const before=(a>>>0) in xs;delete xs[a>>>0];return before?((a>>>0) in xs?-1:1):0;"],
 ["uint32-in-key-closure","const xs=[a];const has=()=>(a>>>0) in xs;return has()?1:0;","const xs=[a];const has=()=>(a>>>0) in xs;return has()?1:0;"],
 ["uint32-in-key-conditional","const xs=[a,b];return (a?(0>>>0):(1>>>0)) in xs?1:0;","const xs=[a,b];return (a?(0>>>0):(1>>>0)) in xs?1:0;"],
 ["uint32-in-key-assignment","const xs=[a];let key=0;return (key=a>>>0) in xs?1:0;","const xs=[a];let key=0;return (key=a>>>0) in xs?1:0;"],
];
