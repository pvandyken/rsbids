from __future__ import annotations
from pathlib import Path
from typing import TYPE_CHECKING, Self

from rsbids.userpath import UserPath

if TYPE_CHECKING:
    from _typeshed import StrPath


class BidsPath(UserPath):
    def __init__(self, *segments: StrPath, entities: dict[str, str], dataset_root: str):
        super().__init__(self, *segments)
        self.entities = entities
        self.dataset_root = dataset_root

    def with_segments(self, *pathsegments: StrPath) -> Path:
        return Path(*pathsegments)

    def absolute(self) -> Self:
        return BidsPath(super().absolute(), self.entities)

    def resolve(self, strict: bool = False) -> Self:
        return BidsPath(super().resolve(strict), self.entities)

    def expanduser(self) -> Path:
        return BidsPath(super().expanduser(), self.entities)
