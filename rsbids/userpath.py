from __future__ import annotations
from pathlib import Path, PurePath, _PathParents
import functools as ft
import sys
from typing import TYPE_CHECKING, Any, Generator, Self, Sequence

if TYPE_CHECKING:
    from _typeshed import StrPath


class UserPathImpl(type(Path())):
    entities: dict[str, str]

    def __new__(cls, *pathsegments: StrPath, **kwargs: Any):
        obj = super().__new__(cls, *pathsegments, **kwargs)
        obj.__init__(*pathsegments, **kwargs)
        return obj

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
        class _UserPathParents(_PathParents):
            def __getitem__(self_, idx: int):
                return self.with_segments(super().__getitem__(idx))

        return _UserPathParents(self)

    @property
    def parent(self) -> Self:
        return self.with_segments(Path(self).parent)

    def joinpath(self, *other: StrPath) -> Self:
        return self.with_segments(self, *other)

    def with_name(self, name: str) -> Self:
        return self.with_segments(Path(self).with_name(name))

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


if sys.version_info >= (3, 12):
    UserPath = type(Path())
else:
    UserPath = UserPathImpl