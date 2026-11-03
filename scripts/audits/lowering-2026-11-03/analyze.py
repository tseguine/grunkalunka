#!/usr/bin/env python3
"""Conservative Rust call inventory and iterative SCCs; not a Rust type checker."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re


class Audit:
    def __init__(self, inventory):
        self.inventory = inventory
        self.nodes = {f['id']: f for f in inventory['functions']}
        assert len(self.nodes) == len(inventory['functions']), 'duplicate parser function identity'
        self.methods = defaultdict(list)
        self.free = defaultdict(list)
        self.types = defaultdict(list)
        for node in self.nodes.values():
            if node['owner']:
                self.methods[(node['owner'], node['name'])].append(node['id'])
            elif node['kind'] == 'function':
                self.free[(node['module'], node['name'])].append(node['id'])
        for definition in inventory['types']:
            self.types[definition['name']].append(definition)
        self.imports = defaultdict(dict)
        self.globs = defaultdict(list)
        for item in inventory['imports']:
            self.read_use(item['module'], item['use'])
        self.flow = defaultdict(set)
        self.parameter_types = defaultdict(dict)
        self.edges = []
        self.unresolved = []

    def read_use(self, module, tree):
        # This parses only the syn-rendered UseTree grammar, never Rust bodies.
        words = re.findall(r'::|[A-Za-z_][A-Za-z_0-9]*|[{},*]', tree)
        def walk(index, prefix):
            local = prefix.copy()
            while index < len(words):
                word = words[index]
                if word == '{':
                    index += 1
                    while index < len(words) and words[index] != '}':
                        index = walk(index, local)
                        if index < len(words) and words[index] == ',': index += 1
                    return index + 1
                if word == '*':
                    self.globs[module].append(local)
                    return index + 1
                if word in (',', '}'): break
                local.append(word); index += 1
                if index < len(words) and words[index] == '::': index += 1; continue
                alias = local[-1]
                if index < len(words) and words[index] == 'as':
                    alias = words[index + 1]; index += 2
                if local[-1] == 'self':
                    local.pop(); alias = local[-1]
                self.imports[module][alias] = local
                return index
            return index
        walk(0, [])

    def canonical(self, segments, module, depth=0):
        if not segments or depth > 12: return []
        if segments[0] == 'crate': return [segments]
        here = module.split('::')
        if segments[0] == 'self': return [here + segments[1:]]
        if segments[0] == 'super':
            count = 0
            while count < len(segments) and segments[count] == 'super': count += 1
            return [here[:-count] + segments[count:]]
        alias = self.imports[module].get(segments[0])
        if alias and alias != segments:
            return self.canonical(alias + segments[1:], module, depth + 1)
        result = [here + segments, ['crate'] + segments]
        for glob in self.globs[module]:
            for base in self.canonical(glob, module, depth + 1):
                result.extend(self.canonical(segments, '::'.join(base), depth + 1))
        return [list(parts) for parts in dict.fromkeys(tuple(parts) for parts in result)]

    def callable(self, expr, caller, depth=0):
        if not expr or depth > 24: return set()
        kind = expr.get('kind')
        if kind == 'closure': return {expr['id']} if expr['id'] in self.nodes else set()
        if kind == 'parameter': return set(self.flow[(expr['function'], expr['index'])])
        if kind == 'union': return set().union(*(self.callable(v, caller, depth+1) for v in expr['values']))
        if kind != 'path': return set()
        parts = expr['segments']
        result = set()
        if len(parts) > 1:
            own = parts[-2]
            if own == 'Self': own = caller.get('context_owner') or caller.get('owner')
            if own: result.update(self.methods.get((own, parts[-1]), []))
        for full in self.canonical(parts, caller['module']):
            qualified = '::'.join(full)
            if qualified in self.nodes: result.add(qualified)
            if len(full) > 1: result.update(self.free.get(('::'.join(full[:-1]), full[-1]), []))
        if len(parts) == 1:
            # Nested item functions retain their enclosing identity in parser IDs.
            for node in self.nodes.values():
                if node['name'] == parts[0] and node['id'].startswith(caller['id'] + '::'):
                    result.add(node['id'])
        return result

    @staticmethod
    def typename(t):
        return t.get('path', [''])[-1] if isinstance(t, dict) else ''

    @staticmethod
    def peel(t):
        while isinstance(t, dict) and Audit.typename(t) in ('Box','Rc','Arc','Ref','RefMut','Pin') and t.get('args'):
            t = t['args'][0]
        return t

    def concrete(self, t, node):
        if not isinstance(t, dict): return t
        result=t.copy()
        if result.get('path')==['Self']:
            result['path']=[node.get('context_owner') or node.get('owner') or 'Self']
        if 'args' in result: result['args']=[self.concrete(a,node) for a in result['args']]
        return result

    def infer(self, expr, caller, depth=0):
        if not expr or depth > 18: return []
        kind = expr.get('kind')
        if kind=='typed': return [self.concrete(expr['type'],caller)] if expr.get('type') else []
        if kind=='parameter':
            t=expr.get('type')
            if t and self.typename(t): return [self.concrete(t,self.nodes[expr['function']])]
            return list(self.parameter_types[(expr['function'],expr['index'])].values())
        if kind == 'union': return [t for v in expr['values'] for t in self.infer(v, caller, depth+1)]
        if kind == 'try':
            return [t['args'][0] for t in self.infer(expr['inner'],caller,depth+1) if t.get('args')]
        if kind == 'element':
            return [t['args'][0] for t in self.infer(expr['container'],caller,depth+1) if t.get('args')]
        if kind == 'field':
            result = []
            for t in self.infer(expr['base'], caller, depth+1):
                for definition in self.types.get(self.typename(self.peel(t)), []):
                    result.extend(f['type'] for f in definition['fields'] if f['name'] == expr['field'])
            return result
        if kind == 'call':
            result = [self.concrete(self.nodes[target]['return_type'],self.nodes[target]) for target in self.callable(expr['callee'], caller) if self.nodes[target]['return_type']]
            parts = expr['callee'].get('segments', [])
            if parts[-2:] in (['Box','new'],['Rc','new'],['Arc','new'],['RefCell','new']):
                result.extend({'path':[parts[-2]],'args':[t]} for t in self.infer(expr['args'][0], caller, depth+1))
            if parts[-1:] in (['replace'],['take']) and expr.get('args'):
                result.extend(self.infer(expr['args'][0],caller,depth+1))
            if not result and len(parts)>1 and parts[-1] in ('new','default'):
                result.append({'path':[parts[-2]],'args':[]})
            return result
        if kind == 'method':
            name = expr['method']; receiver_types = self.infer(expr['receiver'],caller,depth+1); result=[]
            for t in receiver_types:
                outer = self.typename(t); inner = self.peel(t)
                targets = self.methods.get((self.typename(inner),name), [])
                result.extend(self.concrete(self.nodes[target]['return_type'],self.nodes[target]) for target in targets if self.nodes[target]['return_type'])
                if name in ('unwrap','expect','unwrap_or','unwrap_or_else') and outer in ('Option','Result') and t.get('args'): result.append(t['args'][0])
                elif name in ('borrow','borrow_mut') and outer == 'RefCell' and t.get('args'): result.append(t['args'][0])
                elif name in ('as_ref','as_mut','clone','cloned','copied','to_owned','take'): result.append(t)
                elif name in ('pop','first','last','get','get_mut') and outer in ('Vec','VecDeque','Box','Slice','Array') and t.get('args'): result.append({'path':['Option'],'args':[t['args'][0]]})
                elif name in ('iter','iter_mut','into_iter'): result.append({'path':['Iterator'],'args':t.get('args',[])})
                elif name in ('map','filter','filter_map','flat_map','chain','enumerate','rev','take','skip','copied','cloned') and outer=='Iterator': result.append({'path':['Iterator'],'args':[]})
                elif name == 'next' and outer == 'Iterator' and t.get('args'): result.append({'path':['Option'],'args':[t['args'][0]]})
            if not result and name in ('iter','iter_mut','into_iter','encode_utf16','chars','bytes'):
                result.append({'path':['Iterator'],'args':[]})
            return result
        return []

    def targets(self, site, caller):
        if site['kind'] == 'call':
            result = self.callable(site['callee'],caller)
            kind = site['callee'].get('kind')
            return result, 'callback_flow' if kind=='parameter' else 'direct_path' if result else 'unresolved_or_external_call'
        types = self.infer(site['receiver'],caller)
        owners = {self.typename(self.peel(t)) for t in types} - {''}
        unknown_deref = bool(owners & {'Box','Rc','Arc','Ref','RefMut','Pin'})
        result=set()
        for owner in owners: result.update(self.methods.get((owner,site['method']), []))
        if result and not unknown_deref: return result, 'typed_receiver'
        if not owners or unknown_deref:
            result |= {target for (owner,name),targets in self.methods.items() if name==site['method'] for target in targets}
            if result: return result, 'conservative_method_name'
        return set(), 'unresolved_or_external_method'

    def run(self):
        for iteration in range(100):
            changed=False
            for caller in self.nodes.values():
                for site in caller['calls']:
                    targets,precision=self.targets(site,caller)
                    if precision=='conservative_method_name': continue
                    for target in targets:
                        offset=1 if site['kind']=='call' and self.nodes[target]['has_receiver'] else 0
                        for index,arg in enumerate(site['args'][offset:]):
                            if index>=len(self.nodes[target]['params']):continue
                            values=self.callable(arg,caller);slot=self.flow[(target,index)];old=len(slot);slot.update(values);changed|=len(slot)!=old
                            types=self.parameter_types[(target,index)]
                            for t in self.infer(arg,caller):
                                if self.typename(t):
                                    key=json.dumps(t,sort_keys=True)
                                    if key not in types:types[key]=t;changed=True
            if not changed:break
        else: raise RuntimeError('callback flow did not stabilize')
        self.flow_iterations=iteration+1
        for caller in self.nodes.values():
            for site in caller['calls']:
                targets,precision=self.targets(site,caller)
                receipt={'caller':caller['id'],'file':caller['file'],'position':site['position'],'syntax':site['syntax'],'precision':precision}
                if not targets or precision=='conservative_method_name':
                    self.unresolved.append({**receipt,'possible_targets':sorted(targets),'receiver_types':self.infer(site['receiver'],caller) if site['kind']=='method' else [],'expression':site.get('receiver',site.get('callee'))})
                for target in sorted(targets):self.edges.append({**receipt,'target':target})
                if not targets:
                    # Standard iterator/Option helpers invoke passed closures.
                    # Unknown APIs may store them: retain this as an overapproximation.
                    for arg in site['args']:
                        for target in sorted(self.callable(arg,caller)):
                            self.edges.append({**receipt,'target':target,'precision':'external_callback_argument_overapproximation'})
        adjacency={node:set() for node in self.nodes}
        for edge in self.edges:adjacency[edge['caller']].add(edge['target'])
        entries=[node for node in self.nodes if node in ('crate::compile','crate::compile_value','crate::compile_async','crate::compile_heap_value','crate::compile_ui_import')]
        reachable=set(entries);pending=entries.copy()
        while pending:
            for child in adjacency[pending.pop()]:
                if child not in reachable:reachable.add(child);pending.append(child)
        components=scc(adjacency)
        cyclic=[sorted(c) for c in components if len(c)>1 or next(iter(c)) in adjacency[next(iter(c))]]
        lowering=[c for c in cyclic if any(n.startswith('crate::lower::') for n in c)]
        raw_macros=[{'function':n['id'],'file':n['file'],**m} for n in self.nodes.values() for m in n['macros']]
        report_nodes=[{**{k:v for k,v in node.items() if k not in ('calls','macros','body_tokens')},'body_sha256':hashlib.sha256(node.get('body_tokens','').encode()).hexdigest()} for node in self.nodes.values()]
        report={'nodes':report_nodes,'edges':self.edges,'unresolved':self.unresolved,
                'callback_bindings':[{'function':f,'parameter':i,'targets':sorted(ts)} for (f,i),ts in self.flow.items() if ts],
                'macros':raw_macros,'item_macros':self.inventory['item_macros'],'unresolved_modules':self.inventory['unresolved_modules'],
                'cyclic_components':cyclic,'lowering_cyclic_components':lowering,'public_entry_points':entries,'reachable':sorted(reachable),
                'summary':{'files':len(self.inventory['files']),'functions_and_closures':len(self.nodes),'syntactic_call_sites':sum(len(n['calls']) for n in self.nodes.values()),'edges':len(self.edges),'edge_precision':dict(Counter(e['precision'] for e in self.edges)),'unresolved_or_conservative_sites':len(self.unresolved),'unresolved_modules':len(self.inventory['unresolved_modules']),'callback_flow_iterations':self.flow_iterations,'cyclic_components':len(cyclic),'lowering_cyclic_components':len(lowering),'public_reachable_lowering_cyclic_components':sum(any(n in reachable for n in c) for c in lowering),'unparsed_expression_macros':sum(not m['arguments_parsed_as_expressions'] for m in raw_macros)},
                'limitations':['Not rustc type checking or macro expansion; unresolved calls are retained, not certified absent.','Method-name fallback and unknown external callback invocation are overapproximations and may create false cycles.','Conditional compilation other than exact cfg(test) is retained conservatively. Derive-generated methods and external dependencies are outside parsed bodies.','Function-pointer flows through heap fields, returned closures, complex patterns or trait objects may remain unresolved.','SCCs show possible native call cycles, not source-dependent runtime depth. Manual body/guard/queue classification and exercised 2 MiB boundaries remain necessary.']}
        return report


def scc(graph):
    # Iterative Kosaraju avoids making the audit's own DFS native-recursive.
    seen=set();order=[]
    for start in graph:
        if start in seen:continue
        seen.add(start);stack=[(start,iter(graph[start]))]
        while stack:
            node,children=stack[-1]
            try:child=next(children)
            except StopIteration:order.append(node);stack.pop();continue
            if child not in seen:seen.add(child);stack.append((child,iter(graph[child])))
    reverse={node:set() for node in graph}
    for node,children in graph.items():
        for child in children:reverse[child].add(node)
    seen=set();components=[]
    for start in reversed(order):
        if start in seen:continue
        component={start};seen.add(start);pending=[start]
        while pending:
            for child in reverse[pending.pop()]:
                if child not in seen:seen.add(child);component.add(child);pending.append(child)
        components.append(component)
    return components


def main():
    p=argparse.ArgumentParser();p.add_argument('inventory',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
    raw=json.loads(args.inventory.read_text());report=Audit(raw).run();report['inventory_sha256']=hashlib.sha256(args.inventory.read_bytes()).hexdigest();args.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report['summary']))


if __name__=='__main__':main()
