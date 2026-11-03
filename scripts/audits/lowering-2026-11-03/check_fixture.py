#!/usr/bin/env python3
"""Assert only the deliberately constructed call-inventory sensitivity cases."""
import argparse
import json
from pathlib import Path


def check(graph):
    components = [set(component) for component in graph["cyclic_components"]]
    prefix = "crate::lower::"
    assert {prefix + "a", prefix + "b"} in components
    assert any({prefix + "Machine::step", prefix + "with"} <= component
               and any("::closure@" in node for node in component)
               for component in components)
    assert {prefix + "macro_cycle"} in components
    assert {prefix + "through_import", prefix + "nested::imported_cycle"} in components
    assert {prefix + "alias_target::target", prefix + "alias_child::child"} in components
    boxed = prefix + "BoxedMachine::local_enum_cycle"
    assert {boxed} in components
    assert any(edge["caller"] == boxed and edge["target"] == boxed
               and edge["precision"] == "typed_receiver" for edge in graph["edges"])
    assert any(edge["caller"] == prefix + "unknown_boxed_cycle"
               and edge["target"] == boxed
               and edge["precision"] == "conservative_method_name" for edge in graph["edges"])
    assert not any(prefix + "fake" in component for component in components)
    assert not any(node["id"] == prefix + "hidden" for node in graph["nodes"])
    assert graph["summary"]["unresolved_modules"] == 0
    assert graph["summary"]["unparsed_expression_macros"] == 0
    return {"passed": True, "cases": ["direct recursion", "typed callback flow",
            "macro argument recursion", "parent-defined glob import", "transitive imported alias", "local-enum boxed receiver", "unknown boxed fallback",
            "comments and strings excluded", "cfg(test) excluded"]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("graph", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(json.loads(args.graph.read_text())), indent=2))
