from os import PathLike
from pathlib import Path
from typing import Iterable, Mapping
from typing_extensions import Self

from rsbids.bidspath import BidsPath

StrPath = str | PathLike[str]

DerivPathList = StrPath | Iterable[StrPath]

FilterType = str | bool | None | Iterable[str | bool]

class BidsLayout:
    def __new__(
        cls,
        roots: None | StrPath | Iterable[StrPath] = ...,
        derivatives: None | bool | DerivPathList | Mapping[str, DerivPathList] = ...,
        *,
        validate: bool = ...,
        cache: StrPath | None = ...,
        reset_cache: bool = ...,
    ) -> Self: ...
    def __init__(
        self,
        roots: None | StrPath | Iterable[StrPath] = ...,
        derivatives: None | bool | DerivPathList | Mapping[str, DerivPathList] = ...,
        *,
        validate: bool = ...,
        cache: StrPath | None = ...,
        reset_cache: bool = ...,
    ) -> None: ...
    @property
    def entities(self) -> dict[str, list[str]]: ...
    @property
    def metadata(self) -> dict[str, list[str]]: ...
    @property
    def roots(self) -> list[str]: ...
    @property
    def root(self) -> str: ...
    @property
    def description(self) -> DatasetDescription: ...
    @property
    def derivatives(self) -> Self: ...
    @property
    def one(self) -> BidsPath: ...
    def get(
        self,
        **entities: FilterType,
    ) -> Self: ...
    def filter(
        self,
        *,
        root: StrPath | Iterable[StrPath] = ...,
        scope: str | Iterable[str] = ...,
    ) -> Self: ...
    def parse(self, path: StrPath) -> BidsPath: ...
    def index_metadata(self) -> Self: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __iter__(self) -> LayoutIterator: ...
    @classmethod
    def load(cls, path: StrPath) -> Self: ...
    def save(self, path: StrPath) -> None: ...
    def __getstate__(self) -> bytes: ...
    def __setstate__(self, state: bytes) -> None: ...

def create_pybidspath(path: Path) -> BidsPath: ...
def entity_long_to_short(e: str) -> str: ...
def entity_short_to_long(e: str) -> str: ...

class LayoutIterator:
    def __iter__(self) -> Self: ...
    def __next__(self) -> BidsPath: ...

class GeneratedBy:
    @property
    def name(self) -> str: ...
    @property
    def version(self) -> str | None: ...
    @property
    def description(self) -> str | None: ...
    @property
    def code_url(self) -> str | None: ...
    @property
    def container(self) -> str | None: ...

class SourceDataset:
    @property
    def uri(self) -> str | None: ...
    @property
    def doi(self) -> str | None: ...
    @property
    def version(self) -> str | None: ...

class DatasetDescription:
    @property
    def name(self) -> str | None: ...
    @property
    def bids_version(self) -> str | None: ...
    @property
    def hed_version(self) -> list[str] | None: ...
    @property
    def dataset_links(self) -> dict[str, str] | None: ...
    @property
    def dataset_type(self) -> str | None: ...
    @property
    def license(self) -> str | None: ...
    @property
    def authors(self) -> list[str] | None: ...
    @property
    def acknowledgements(self) -> str | None: ...
    @property
    def how_to_acknowledge(self) -> str | None: ...
    @property
    def funding(self) -> list[str] | None: ...
    @property
    def ethics_approvals(self) -> list[str] | None: ...
    @property
    def references_and_links(self) -> list[str] | None: ...
    @property
    def dataset_doi(self) -> str | None: ...
    @property
    def generated_by(self) -> list[GeneratedBy] | None: ...
    @property
    def source_datasets(self) -> list[SourceDataset] | None: ...
    @property
    def pipeline_description(self) -> GeneratedBy | None: ...
