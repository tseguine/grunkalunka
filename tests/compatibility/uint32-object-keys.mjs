export const uint32ObjectKeyPrograms=[
 ["uint32-object-key-numeric","const o={[a>>>0]:b};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0)+(Object.hasOwn(o,1)?2:0);","const o={[a>>>0]:b};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0)+(Object.hasOwn(o,1)?2:0);"],
 ["uint32-object-key-undefined","const o={[a>>>b]:undefined};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);","const o={[a>>>b]:undefined};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);"],
 ["uint32-object-key-effects","let n=0;const o={[(n=1,a)>>>0]:(n=n*10+2,b)};return n;","let n=0;const o={[(n=1,a)>>>0]:(n=n*10+2,b)};return n;"],
 ["uint32-object-key-snapshot","let key=0;const o={[key>>>0]:(key=1,b)};return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)&&key===1?1:0;","let key=0;const o={[key>>>0]:(key=1,b)};return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)&&key===1?1:0;"],
 ["uint32-object-key-collision","const o={[a>>>0]:1,[a>>>0]:2};return Object.keys(o).length===1&&Object.values(o)[0]===2?1:0;","const o={[a>>>0]:1,[a>>>0]:2};return Object.keys(o).length===1&&Object.values(o)[0]===2?1:0;"],
 ["uint32-object-key-order","const o={z:a,[2>>>0]:b,[1>>>0]:a,y:b};const keys=Object.keys(o);return keys[0]==='1'&&keys[1]==='2'&&keys[2]==='z'&&keys[3]==='y'?1:0;","const o={z:a,[2>>>0]:b,[1>>>0]:a,y:b};const keys=Object.keys(o);return keys[0]==='1'&&keys[1]==='2'&&keys[2]==='z'&&keys[3]==='y'?1:0;"],
 ["uint32-object-key-boundary","const o={z:a,[(-1)>>>0]:b,[(-2)>>>0]:a};const keys=Object.keys(o);return keys[0]==='4294967294'&&keys[1]==='z'&&keys[2]==='4294967295'?1:0;","const o={z:a,[(-1)>>>0]:b,[(-2)>>>0]:a};const keys=Object.keys(o);return keys[0]==='4294967294'&&keys[1]==='z'&&keys[2]==='4294967295'?1:0;"],
 ["uint32-object-key-spread","const base={x:a};const o={...base,[a>>>0]:b};return Object.keys(o).length===2&&Object.hasOwn(o,'x')?1:0;","const base={x:a};const o={...base,[a>>>0]:b};return Object.keys(o).length===2&&Object.hasOwn(o,'x')?1:0;"],
 ["uint32-object-key-reference","const child={v:a};const o={[0>>>0]:child};return Object.values(o)[0]===child?1:0;","const child={v:a};const o={[0>>>0]:child};return Object.values(o)[0]===child?1:0;"],
 ["uint32-object-key-closure","const make=()=>({[a>>>0]:b});const o=make();return Object.keys(o).length;","const make=()=>({[a>>>0]:b});const o=make();return Object.keys(o).length;"],
 ["uint32-object-key-conditional","const o={[a?(0>>>0):(1>>>0)]:b};return Object.hasOwn(o,0)?1:0;","const o={[a?(0>>>0):(1>>>0)]:b};return Object.hasOwn(o,0)?1:0;"],
 ["uint32-object-key-assignment","let key=0;const o={[key=a>>>0]:b};return Object.keys(o).length;","let key=0;const o={[key=a>>>0]:b};return Object.keys(o).length;"],
];
