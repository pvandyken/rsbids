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
from functools import cached_property

import re
from typing import TYPE_CHECKING, Any, Iterable, Literal
import warnings
from rsbids.bidspath import BidsPath
from rsbids.exceptions import CompatibilityError
from rsbids._lib import BidsLayout, entity_long_to_short


if TYPE_CHECKING:
    from rsbids._lib import StrPath


class BIDSFile:
    """Represents a single file or directory in a BIDS dataset.

    Parameters
    ----------
    filename : str
        The path to the corresponding file.
    """

    def __init__(self, filename: StrPath, _layout: BidsLayout | None = None):
        self._bidspath = BidsPath(filename)
        self.path = str(filename)
        self.filename = self._bidspath.name
        self.dirname = str(self._bidspath.parent)
        self.is_dir = self._bidspath.is_dir()
        self._layout = _layout

    def __repr__(self):
        return "<{} filename='{}'>".format(self.__class__.__name__, self.path)

    def __fspath__(self):
        return self.path

    def __eq__(self, other: Any):
        if not isinstance(other, self.__class__):
            return False
        return self._bidspath == other._bidspath and self._layout == other._layout

    def __hash__(self):
        return hash(hash(self._bidspath) + hash(self._layout))

    @classmethod
    def _from_bidspath(cls, bidspath: BidsPath, _layout: BidsLayout | None = None):
        obj = cls("")
        obj._bidspath = bidspath
        obj.path = str(bidspath)
        obj.filename = obj._bidspath.name
        obj.dirname = str(obj._bidspath.parent)
        obj.is_dir = obj._bidspath.is_dir()
        obj._layout = _layout
        return obj

    @property
    def relpath(self):
        """Return path relative to layout root"""
        raise CompatibilityError("BIDSFIle.relpath() is not yet implemented")

    def get_associations(self, kind=None, include_parents=False):
        """Get associated files, optionally limiting by association kind.

        Parameters
        ----------
        kind : str
            The kind of association to return (e.g., "Child").
            By default, all associations are returned.
        include_parents : bool
            If True, files related through inheritance
            are included in the returned list. If False, only directly
            associated files are returned. For example, a file's JSON
            sidecar will always be returned, but other JSON files from
            which the sidecar inherits will only be returned if
            include_parents=True.

        Returns
        -------
        list
            A list of BIDSFile instances.
        """
        raise CompatibilityError("get_associations() is not yet supported")

    def get_metadata(self):
        """Return all metadata associated with the current file."""
        return self._bidspath.metadata

    def get_entities(
        self, metadata: bool = False, values: Literal["tags", "objects"] = "tags"
    ):
        """Return entity information for the current file.

        Parameters
        ----------
        metadata : bool or None
            If False (default), only entities defined
            for filenames (and not those found in the JSON sidecar) are
            returned. If True, only entities found in metadata files (and not
            defined for filenames) are returned. If None, all available
            entities are returned.
        values : str
            The kind of object to return in the dict's values.
            Must be one of:
                * 'tags': Returns only the tagged value--e.g., if the key
                is "subject", the value might be "01".
                * 'objects': Returns the corresponding Entity instance.

        Returns
        -------
        dict
            A dict, where keys are entity names and values are Entity
            instances.
        """
        entities = self._bidspath.entities if metadata is not True else {}
        md = self._bidspath.metadata if metadata is not False else {}
        result = {**entities, **md}
        if values == "tags":
            return result
        if self._layout is None:
            raise CompatibilityError(
                "BIDSFile.get_entities() requires a layout to be supplied to BIDSFile"
            )
        return {
            k: Entity._from_rsbids(self._layout, k, v)  # type: ignore
            for k, v in result.items()
        }

    def copy(self, path_patterns, symbolic_link=False, root=None, conflicts="fail"):
        """Copy the contents of a file to a new location.

        Parameters
        ----------
        path_patterns : list
            List of patterns used to construct the new
            filename. See :obj:`build_path` documentation for details.
        symbolic_link : bool
            If True, use a symbolic link to point to the
            existing file. If False, creates a new file.
        root : str
            Optional path to prepend to the constructed filename.
        conflicts : str
            Defines the desired action when the output path already exists.
            Must be one of:
                'fail': raises an exception
                'skip' does nothing
                'overwrite': overwrites the existing file
                'append': adds  a suffix to each file copy, starting with 1
        """
        raise CompatibilityError("BIDSFile.copy() is not yet supported")


class Entity:
    """
    Represents a single entity defined in the JSON config.

    Parameters
    ----------
    name : str
        The name of the entity (e.g., 'subject', 'run', etc.)
    pattern : str
        A regex pattern used to match against file names.
        Must define at least one group, and only the first group is
        kept as the match.
    mandatory : bool
        If True, every File _must_ match this entity.
    directory : str
        Optional pattern defining a directory associated
        with the entity.
    dtype : str
        The optional data type of the Entity values. Must be
        one of 'int', 'float', 'bool', or 'str'. If None, no type
        enforcement will be attempted, which means the dtype of the
        value may be unpredictable.
    """

    __tablename__ = "entities"

    def __init__(
        self,
        name: str,
        pattern: str | None = None,
        mandatory: bool = False,
        directory: str | None = None,
        dtype: str | type[Any] = "str",
        _layout: BidsLayout | None = None,
    ):
        self.name = name
        self._pattern = pattern
        self._regex = re.compile(self._pattern) if self._pattern else None
        self.mandatory = mandatory
        self.directory = directory
        self._layout = _layout

        if dtype != "str":
            warnings.warn(
                "Entity(dtype=...) currently has no effect in rsbids, all entities "
                "values are treated as strings"
            )

        self.dtype = str

    @classmethod
    def _from_rsbids(cls, layout: BidsLayout, entity: str, value: str):
        if entity in layout.entities:
            short = entity_long_to_short(entity)
            if entity == "suffix":
                pattern = r"(?:^|[_/\\])([a-zA-Z0-9]+)\.[^/\\]+$"
            elif entity == "extension":
                pattern = r"[^./\\](\.[^/\\]+)$"
            elif entity == "datatype":
                pattern = (
                    r"[/\\]+(anat|beh|dwi|eeg|fmap|func|ieeg|meg|motion|micr|nirs|perf|"
                    r"pet)[/\\]+"
                )
            else:
                # "better-than-nothing" catch-all
                pattern = rf"[_/\\]+{short}-([a-zA-Z0-9])"
            if entity == "subject":
                directory = "{subject}"
            elif entity == "session":
                directory = "{subject}{session}"
            else:
                directory = None
            return cls(
                name=entity,
                pattern=pattern,
                mandatory=False,
                directory=directory,
                _layout=layout,
            )

    def __repr__(self):
        return f"<Entity {self.name} (pattern={self.pattern}, dtype={self.dtype})>"

    def __iter__(self) -> Iterable[str]:
        yield from self.unique()

    @property
    def pattern(self):
        warnings.warn(
            "Entity.pattern has only limited support from rsbids. Buggy behaviour is "
            "possible"
        )
        return self._pattern

    @pattern.setter
    def pattern(self, value: str):
        self._pattern = value

    @property
    def regex(self):
        warnings.warn(
            "Entity.regex has only limited support from rsbids. Buggy behaviour is "
            "possible"
        )
        return self._regex

    @regex.setter
    def regex(self, value: re.Pattern[Any]):
        self._regex = value

    @cached_property
    def files(self):
        if self._layout == None:
            raise CompatibilityError(
                "Entity.files cannot be used if a layout is not provided to Entities"
            )
        return {
            str(p): p.entities[self.name] for p in self._layout.get(**{self.name: True})
        }

    def match_file(self, f: BIDSFile) -> str | None:
        """
        Determine whether the passed file matches the Entity.

        Parameters
        ----------
        f : BIDSFile
            The BIDSFile instance to match against.

        Returns
        -------
        the matched value if a match was found, otherwise None.
        """
        if self.regex is None:
            return None
        m = self.regex.search(f.path)
        val = m.group(1) if m is not None else None

        return val

    def unique(self):
        """Return all unique values/levels for the current entity."""
        return list(set(self.files.values()))

    def count(self, files: bool = False):
        """Return a count of unique values or files.

        Parameters
        ----------
        files : bool
            When True, counts all files mapped to the Entity.
            When False, counts all unique values.

        Returns
        -------
        int
            Count of unique values or files.
        """
        return len(self.files) if files else len(self.unique())
