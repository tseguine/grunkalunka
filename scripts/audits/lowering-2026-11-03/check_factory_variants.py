#!/usr/bin/env python3
"""Check the finite variant model from the manual source ledger.

This is an independent, small consequence check, not a Rust exhaustive-match
checker or a replacement implementation of the accompanying gloss evaluator.
"""
import json


EDGES = [
    ("start_arguments", "advance_arguments"),
    ("advance_arguments", "finish_declaration"),
    ("advance_arguments", "finish_expression"),
    ("finish_declaration", "start_returned_check"),
    ("finish_expression", "start_returned_check"),
    ("start_returned_check", "resume_returned_check"),
    ("resume_returned_check", "schedule_initializer"),
    ("schedule_initializer", "start_arguments"),
]
OUTPUTS = {"Complete", "Materialize", "CheckClosure"}
FACTORY_ARM = "Factory"


def consequences(exhaustive):
    # The premises are the ledger's reviewed successful returns, unchanged
    # forwarding and exact enum dispatch. Unknown is not treated as false.
    support = FACTORY_ARM in OUTPUTS
    opposition = exhaustive and all(variant != FACTORY_ARM for variant in OUTPUTS)
    state = {(False, False): "unknown", (True, False): "supported",
             (False, True): "opposed", (True, True): "contradiction"}[support, opposition]
    return {"supports_factory_arm": support, "opposes_factory_arm": opposition,
            "state": state, "complete_path_queues_advance": True}


def main():
    reachable = set(EDGES)
    while True:
        expanded = reachable | {(source, target) for source, middle in reachable
                                for following, target in reachable if middle == following}
        if expanded == reachable:
            break
        reachable = expanded
    assert ("start_arguments", "start_arguments") in reachable
    complete = consequences(True)
    incomplete = consequences(False)
    assert complete["state"] == "opposed"
    assert incomplete["state"] == "unknown"
    print(json.dumps({"syntactic_cycle": True, "with_exhaustive_success_set": complete,
                      "without_exhaustiveness_premise": incomplete,
                      "scope": "Finite consequences of reviewed source premises; no whole-program acyclicity claim."}, indent=2))


if __name__ == "__main__":
    main()
