# The code in this files has been adapted from pybids in accordance with the following
# license:
#
# The MIT License (MIT)
#
# Copyright (c) 2015-2016, Ariel Rokem, The University of Washington eScience Institute.
# Copyright (c) 2016--, PyBIDS developers, Planet Earth
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#
# See https://github.com/bids-standard/pybids for more information

from __future__ import annotations
from typing import Iterable, Sequence, TypeVar

_T = TypeVar("_T", "str | bool", str, bool)

class BIDSLayoutIndexer:
    """Indexer class for BIDSLayout.
    
    Compatibility layer for pybids: The class itself does not do anything but can
    provide configuration to the rust implementation
    """

    validate: bool
    """
    If True, all files are checked for BIDS compliance when first indexed,
    and non-compliant files are ignored. This provides a convenient way to
    restrict file indexing to only those files defined in the "core" BIDS
    spec, as setting ``validate=True`` will lead noncompliant files
    like ``sub-01/nonbidsfile.txt`` to be ignored.
    """
    ignore: Sequence[str]
    """
    Path(s) to exclude from indexing. Each path is either a string or a
    SRE_Pattern object (i.e., compiled regular expression). If a string is
    passed, it must be either an absolute path, or be relative to the BIDS
    project root. If an SRE_Pattern is passed, the contained regular
    expression will be matched against the full (absolute) path of all
    files and directories. By default, indexing ignores all files in
    'code/', 'stimuli/', 'sourcedata/', 'models/', and any hidden
    files/dirs beginning with '.' at root level.
    """
    force_index: Sequence[str]
    """
    Path(s) to forcibly index in the BIDSLayout, even if they would
    otherwise fail validation. See the documentation for the ignore
    argument for input format details. Note that paths in force_index takes
    precedence over those in ignore (i.e., if a file matches both ignore
    and force_index, it *will* be indexed).
    Note: NEVER include 'derivatives' here; use the derivatives argument
    (or :obj:`bids.layout.BIDSLayout.add_derivatives`) for that.
    """
    index_metadata: bool
    """
    If True, all metadata files are indexed. If False, metadata will not be
    available (but indexing will be faster).
    """
    config_filename: str | None
    """
    Optional name of filename within directories
    that contains configuration information.
    """
    filters: dict[str, Sequence[str | bool]]
    """
    keyword arguments passed to the .get() method of a
    :obj:`bids.layout.BIDSLayout` object. These keyword arguments define
    what files get selected for metadata indexing.
    """

    def __init__(
        self,
        validate: bool = False,
        ignore: str | Iterable[str] | None = None,
        force_index: str | Iterable[str] | None = None,
        index_metadata: bool = True,
        config_filename: str | None = None,
        **filters: None | str | bool | Sequence[str | bool],
    ):
        self.validate = validate
        self.ignore = self._to_sequence(ignore)
        self.force_index = self._to_sequence(force_index)
        self.index_metadata = index_metadata
        self.config_filename = config_filename
        self.filters = {f: self._to_sequence(v) for f, v in filters}

    def _to_sequence(self, param: None | _T | Iterable[_T]) -> Sequence[_T]:
        if param is None:
            return []
        if isinstance(param, (str, bool)):
            return [param]  # type: ignore
        return list(iter(param))

