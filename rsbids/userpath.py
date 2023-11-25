"""Backport of py312 features

py312 introduces `Path.with_segments`, allowing `Path` to be flexibly, stably
subclassed, even across future feature additions and API changes. This module backports
that method as far as py38. Certain other features, such as indexing `Path.parents` with
slices and negative numbers, are also backported
"""
from __future__ import annotations
from pathlib import Path, PurePath
from pathlib import _PathParents  # type: ignore
import sys
from typing import (
    TYPE_CHECKING,
    Any,
    Generator,
    Sequence,
    SupportsIndex,
    cast,
    overload,
)
from typing_extensions import Self

if TYPE_CHECKING:
    from _typeshed import StrPath


class _UserPathParents(_PathParents):  # type: ignore
    """This object provides sequence-like access to the logical ancestors
    of a path.  Don't try to construct it yourself."""

    def __init__(self, path: UserPath):
        super().__init__(Path(path))  # type: ignore
        self.path = path

    @overload
    def __getitem__(self, idx: SupportsIndex) -> Path:
        ...

    @overload
    def __getitem__(self, idx: slice) -> tuple[Path, ...]:
        ...

    if sys.version_info < (3, 10):

        def __getitem__(self, idx: SupportsIndex | slice) -> Path | tuple[Path, ...]:
            if isinstance(idx, slice):
                return tuple(self._get(i) for i in range(len(self))[idx])
            idx = int(idx)
            if idx < 0:
                return self._get(len(self) + idx)
            return self._get(idx)

    else:

        def __getitem__(self, idx: SupportsIndex | slice) -> Path | tuple[Path, ...]:
            elem = cast(
                "tuple[Path, ...] | Path", super().__getitem__(idx)  # type: ignore
            )
            if isinstance(elem, tuple):
                return tuple(self.path.with_segments(p) for p in elem)
            return self.path.with_segments(elem)

    def _get(self, idx: SupportsIndex) -> Path:
        elem = cast("Path", super().__getitem__(idx))  # type: ignore
        return self.path.with_segments(elem)


class UserPathImpl(type(Path())):
    entities: dict[str, str]

    def __new__(cls, *pathsegments: StrPath, **kwargs: Any):
        return super().__new__(cls, *pathsegments, **kwargs)

    def __init__(self, *pathsegments: StrPath, **kwargs: Any):
        super().__init__()

    def with_segments(self, *pathsegments: StrPath) -> Self:
        return type(self)(*pathsegments)

    def __truediv__(self, key: StrPath) -> Self:
        return self.with_segments(self, key)

    def __rtruediv__(self, key: StrPath) -> Self:
        return self.with_segments(key, self)

    @property
    def parents(self) -> Sequence[Self]:
        return _UserPathParents(self)

    @property
    def parent(self) -> Self:
        return self.with_segments(Path(self).parent)

    def joinpath(self, *other: StrPath) -> Self:
        return self.with_segments(self, *other)

    def with_name(self, name: str) -> Self:
        return self.with_segments(Path(self).with_name(name))

    if sys.version_info >= (3, 9):

        def with_stem(self, stem: str) -> Self:
            return self.with_segments(Path(self).with_stem(stem))

    def with_suffix(self, suffix: str) -> Self:
        return self.with_segments(Path(self).with_suffix(suffix))

    def glob(self, pattern: str) -> Generator[Self, None, None]:
        for path in Path(self).glob(pattern):
            yield self.with_segments(path)

    def iterdir(self) -> Generator[Self, None, None]:
        for path in Path(self).iterdir():
            yield self.with_segments(path)

    def rglob(self, pattern: str) -> Generator[Self, None, None]:
        for path in Path(self).rglob(pattern):
            yield self.with_segments(path)

    def rename(self, target: str | PurePath) -> Self:
        return self.with_segments(Path(self).rename(target))

    def replace(self, target: str | PurePath) -> Self:
        return self.with_segments(Path(self).replace(target))

    def absolute(self) -> Self:
        return self.with_segments(Path(self).absolute())

    def resolve(self, strict: bool = False) -> Self:
        return self.with_segments(Path(self).resolve(strict))

    def expanduser(self) -> Self:
        return self.with_segments(Path(self).expanduser())

    def relative_to(self, *other: StrPath) -> Self:
        return self.with_segments(Path(self).relative_to(*other))


if sys.version_info >= (3, 12):
    UserPath = type(Path())
else:
    UserPath = UserPathImpl
