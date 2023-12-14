from __future__ import annotations
from rsbids import parse
from hypothesis import given, strategies as st, assume, settings

from snakebids import bids
from snakebids.utils.utils import BidsEntity
import rsbids.tests.strategies as rb_st

from rsbids.tests.helpers import debug


@settings(max_examples=1000)
@given(
    root=st.text(), entities=rb_st.entity_vals(max_entities=10, restrict_patterns=True)
)
def test_parse_single_path(root: str, entities: dict[str, str]):
    assume(not parse(root).entities)
    # for now need to convert entities to tag names
    path = bids(
        root=root, **{BidsEntity.normalize(e).wildcard: v for e, v in entities.items()}
    )
    parsed = parse(path)
    assert parsed.entities == entities


@given(
    entities=st.lists(
        rb_st.entity_vals(max_entities=10, restrict_patterns=True), max_size=3
    )
)
def test_parse_multiple_paths(entities: list[dict[str, str]]):
    # for now need to convert entities to tag names
    paths = [
        bids(**{BidsEntity.normalize(e).wildcard: v for e, v in ents.items()})
        for ents in entities
    ]
    for path, ents in zip(parse(paths), entities):
        assert path.entities == ents
