from __future__ import annotations
from pathlib import Path
from typing import TYPE_CHECKING, Self

from rsbids.userpath import UserPath
from rsbids.rsbids import create_pybidspath

if TYPE_CHECKING:
    from _typeshed import StrPath


class BidsPath(UserPath):
    entities: dict[str, str]
    dataset_root: str

    def __init__(
        self,
        *segments: StrPath,
        _entities: dict[str, str] | None = None,
        _dataset_root: str | None = None,
    ):
        if _entities is None and _dataset_root is None:
            tpl = create_pybidspath(Path(*segments))
            _entities = tpl.entities
            _dataset_root = tpl.dataset_root
        super().__init__(self, *segments)
        self.entities = _entities or {}
        self.dataset_root = _dataset_root or ""

    def with_segments(self, *pathsegments: StrPath) -> Path:
        return Path(*pathsegments)

    def absolute(self) -> Self:
        return BidsPath(
            Path(self).absolute(),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
        )

    def resolve(self, strict: bool = False) -> Self:
        return BidsPath(
            Path(self).resolve(strict),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
        )

    def expanduser(self) -> Path:
        return BidsPath(
            Path(self).expanduser(),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
        )
