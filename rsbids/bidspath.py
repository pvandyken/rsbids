from __future__ import annotations
import json
from pathlib import Path
from typing import TYPE_CHECKING, Any, Container, Iterable
from typing_extensions import Self

from rsbids.userpath import UserPath
from rsbids._lib import create_pybidspath, BidsLayout

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
        _spec: BidsLayout | None = None,
    ):
        if _entities is None or _dataset_root is None:
            tpl = create_pybidspath(Path(*segments))
            _entities = tpl.entities
            _dataset_root = tpl.dataset_root
        super().__init__(self, *segments)
        self.entities = _entities
        self.dataset_root = _dataset_root

        # Eventually, this will be a "proper" spec defining how the path was parsed,
        # but for now we just use the layout it came from
        self._spec = _spec

    @property
    def metadata(self):
        result: dict[str, Any] = {}
        for parent in self.parents[::-1]:
            # Crude check for when we traverse past the root
            if len(str(parent)) < len(self.dataset_root):
                continue
            jsons = [self._parse(p) for p in parent.iterdir() if p.suffix == ".json"]
            jsons = list(self._subset_paths(jsons, exclude="extension"))

            # For propery bids validity, there should only be one file at this point,
            # but don't worry about that for now
            for path in jsons:
                result.update(path.read_json())

        return result

    def read_json(self, encoding: str | None = None, errors: str | None = None) -> Any:
        with self.open(encoding=encoding, errors=errors) as f:
            return json.load(f)

    def _subset_paths(
        self, paths: Iterable[Self], exclude: Container[str] | None = None
    ):
        exclude = set() if exclude is None else exclude
        for path in paths:
            entities = {e: v for e, v in path.entities.items() if e not in exclude}
            theirs = set(entities.items())
            ours = set(self.entities.items())
            # They have no keys we don't have, and at least one of our keys
            if not theirs - ours and theirs & ours:
                yield path

    def _parse(self, path: Path):
        """Parse new path according to the spec of the BidsPath"""
        if self._spec is not None:
            return self._spec.parse(path)
        return BidsPath(path)

    def with_segments(self, *pathsegments: StrPath) -> Path:  # type: ignore
        return Path(*pathsegments)

    def absolute(self) -> Self:
        return BidsPath(
            Path(self).absolute(),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
            _spec=self._spec,
        )

    def resolve(self, strict: bool = False) -> Self:
        return BidsPath(
            Path(self).resolve(strict),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
            _spec=self._spec,
        )

    def expanduser(self) -> Self:
        return BidsPath(
            Path(self).expanduser(),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
            _spec=self._spec,
        )

    def relative_to(self, *other: StrPath) -> Self:
        return BidsPath(
            Path(self).relative_to(*other),
            _entities=self.entities,
            _dataset_root=self.dataset_root,
            _spec=self._spec,
        )
