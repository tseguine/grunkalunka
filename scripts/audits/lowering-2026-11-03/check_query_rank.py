#!/usr/bin/env python3
"""Check the finite graph implied by a manually reviewed calls-rank certificate.

This checks the certificate's consequences, not its Rust source premises.
"""
import argparse
from collections import deque
import json
from pathlib import Path


def topological_order(nodes, edges):
    successors = {node: set() for node in nodes}
    incoming = dict.fromkeys(nodes, 0)
    for source, target in edges:
        if target not in successors[source]:
            successors[source].add(target)
            incoming[target] += 1
    ready = deque(sorted(node for node, count in incoming.items() if count == 0))
    order = []
    longest = dict.fromkeys(nodes, 1)
    while ready:
        source = ready.popleft()
        order.append(source)
        for target in sorted(successors[source]):
            longest[target] = max(longest[target], longest[source] + 1)
            incoming[target] -= 1
            if incoming[target] == 0:
                ready.append(target)
    assert len(order) == len(nodes), "certificate permits a cycle"
    return order, max(longest.values())


def check(certificate):
    functions = certificate["functions"]
    low, high = certificate["counter_domain"]
    assert (low, high) == (0, 8)
    edges = certificate["edges"]
    assert all(edge["from"] in functions and edge["to"] in functions for edge in edges)
    assert all(edge["minimum_calls_increment"] >= 0 for edge in edges)
    zero = [(edge["from"], edge["to"]) for edge in edges
            if edge["minimum_calls_increment"] == 0]
    order, _ = topological_order(functions, zero)
    states = [(name, calls) for name in functions for calls in range(low, high + 1)]
    transitions = []
    for edge in edges:
        for calls in range(low, min(high, edge["max_source_calls"]) + 1):
            minimum = calls + edge["minimum_calls_increment"]
            maximum = edge["max_target_calls"] if edge["variable_increment"] else minimum
            for following in range(minimum, min(maximum, high) + 1):
                transitions.append(((edge["from"], calls), (edge["to"], following)))
    _, longest = topological_order(states, transitions)
    return {
        "reviewed_source_commit": certificate["source_commit"],
        "semantic_helpers": len(functions), "reviewed_edges": len(edges),
        "zero_increment_graph_acyclic": True,
        "zero_increment_topological_order": [functions[name] for name in order],
        "finite_overapproximation_states": len(states),
        "finite_overapproximation_transitions": len(set(transitions)),
        "finite_overapproximation_acyclic": True,
        "longest_semantic_helper_chain_in_model": longest,
        "scope": "Consequences of reviewed source premises; not a Rust control-flow or native-byte proof.",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("certificate", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(json.loads(args.certificate.read_text())), indent=2))
