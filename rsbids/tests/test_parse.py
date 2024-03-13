from __future__ import annotations
from rsbids import parse
from hypothesis import given, strategies as st, assume, settings

from snakebids import bids
import rsbids.tests.strategies as rb_st


@settings(max_examples=1000)
@given(entities=rb_st.entity_vals(max_entities=10))
def test_parse_single_path(entities: dict[str, str]):
    path = bids(**entities)
    if "suffix" not in entities:
        assume("_" not in entities.get("extension", ""))
    parsed = parse(path)

    assert parsed.entities == entities


@given(entities=st.lists(rb_st.entity_vals(max_entities=10), max_size=3))
def test_parse_multiple_paths(entities: list[dict[str, str]]):
    paths = [bids(**ents) for ents in entities]
    for ents in entities:
        if "suffix" not in ents:
            assume("_" not in ents.get("extension", ""))
    for path, ents in zip(parse(paths), entities):
        assert path.entities == ents
