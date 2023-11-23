from __future__ import annotations
from typing import TYPE_CHECKING, Iterable, overload

from rsbids.bidspath import BidsPath


if TYPE_CHECKING:
    from rsbids._lib import StrPath


@overload
def parse(path: StrPath) -> BidsPath:
    ...


@overload
def parse(path: Iterable[StrPath]) -> list[BidsPath]:
    ...


def parse(path: StrPath | Iterable[StrPath]):
    try:
        return BidsPath(path)  # type: ignore
    except TypeError:
        result: list[BidsPath] = []
        for p in path:  # type: ignore
            result.append(BidsPath(p))  # type: ignore
        return result
