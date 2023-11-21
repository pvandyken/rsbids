from __future__ import annotations
from typing import TYPE_CHECKING, Iterable, overload, Mapping

from rsbids.bidspath import BidsPath
from rsbids._lib import BidsLayout


if TYPE_CHECKING:
    from rsbids._lib import StrPath, DerivPathList


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


def layout(
    roots: None | StrPath | Iterable[StrPath] = ...,
    derivatives: None | bool | DerivPathList | Mapping[str, DerivPathList] = ...,
    *,
    validate: bool = ...,
    cache: StrPath | None = ...,
    reset_cache: bool = ...,
):
    return BidsLayout(
        roots=roots,
        derivatives=derivatives,
        validate=validate,
        cache=cache,
        reset_cache=reset_cache,
    )
