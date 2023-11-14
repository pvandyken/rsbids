from pathlib import Path
from typing import Iterable, Mapping, Self

_StrPath = str | Path

_DerivPathList = _StrPath | Iterable[_StrPath]

class BidsLayout:
    def __init__(
        self,
        roots: _StrPath | Iterable[str | Path],
        derivatives: None | bool | _DerivPathList | Mapping[str, _DerivPathList],
    ) -> None: ...
    def get(
        self,
        root: _StrPath | Iterable[_StrPath],
        scope: str | Iterable[str],
        **entities: str | bool | None | Iterable[str | bool],
    ) -> Self: ...
