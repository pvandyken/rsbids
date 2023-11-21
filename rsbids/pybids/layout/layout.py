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

"""rsbids implementation of (nearly) complete pybids layout api

The code borrows liberally from the pybids repository, including code and (especially)
docstrings
"""
from __future__ import annotations
import enum
from functools import cached_property
from pathlib import Path
from typing import TYPE_CHECKING, Any, Callable, Iterable, Literal, Mapping
from typing_extensions import TypeAlias
import warnings
from rsbids import BidsLayout
from rsbids.exceptions import CompatibilityError
from rsbids.pybids.layout.index import BIDSLayoutIndexer
from rsbids.pybids.layout.models import BIDSFile

if TYPE_CHECKING:
    from rsbids._lib import StrPath, DerivPathList, FilterType

PybidsFilterType: TypeAlias = (
    "str | bool | None | Query | int | Iterable[str | bool | Query | int]"
)


class BIDSLayout:
    """Layout class representing an entire BIDS dataset.

    Parameters
    ----------
    root : str
        The root directory of the BIDS dataset.
    validate : bool, optional
        If True, all files are checked for BIDS compliance when first indexed,
        and non-compliant files are ignored. This provides a convenient way to
        restrict file indexing to only those files defined in the "core" BIDS
        spec, as setting validate=True will lead files in supplementary folders
        like derivatives/, code/, etc. to be ignored.
    absolute_paths : bool, optional
        If True, queries always return absolute paths.
        If False, queries return relative paths (for files and
        directories).
    derivatives : bool or str or list, optional
        Specifies whether and/or which
        derivatives to index. If True, all pipelines found in the
        derivatives/ subdirectory will be indexed. If a str or list, gives
        the paths to one or more derivatives directories to index. If False
        or None, the derivatives/ directory is ignored during indexing, and
        derivatives will have to be added manually via add_derivatives().
        Note: derivatives datasets MUST contain a dataset_description.json
        file in order to be indexed.
    config : str or list or None, optional
        Optional name(s) of configuration file(s) to use.
        By default (None), uses 'bids'.
    sources : :obj:`bids.layout.BIDSLayout` or list or None, optional
        Optional BIDSLayout(s) from which the current BIDSLayout is derived.
    config_filename : str
        Optional name of filename within directories
        that contains configuration information.
    regex_search : bool
        Whether to require exact matching (True) or regex
        search (False, default) when comparing the query string to each
        entity in .get() calls. This sets a default for the instance, but
        can be overridden in individual .get() requests.
    database_path : str
        Optional path to directory containing SQLite database file index
        for this BIDS dataset. If a value is passed and the folder
        already exists, indexing is skipped. By default (i.e., if None),
        an in-memory SQLite database is used, and the index will not
        persist unless .save() is explicitly called.
    reset_database : bool
        If True, any existing directory specified in the
        database_path argument is deleted, and the BIDS dataset provided
        in the root argument is reindexed. If False, indexing will be
        skipped and the existing database file will be used. Ignored if
        database_path is not provided.
    indexer: BIDSLayoutIndexer or callable
        An optional BIDSLayoutIndexer instance to use for indexing, or any
        callable that takes a BIDSLayout instance as its only argument. If
        None, a new indexer with default parameters will be implicitly created.
    is_derivative: bool
        Index dataset as a derivative dataset. This can be enabled along with
        validate=False to index derivatives without dataset_description.json. If
        validate=True, the dataset must have a dataset_description.json with
        DatasetType=derivative and GeneratedBy
    indexer_kwargs: dict
        Optional keyword arguments to pass onto the newly created
        BIDSLayoutIndexer. Valid keywords are 'ignore', 'force_index',
        'index_metadata', and 'config_filename'. Ignored if indexer is not
        None.
    """

    def __init__(
        self,
        root: None | StrPath | Iterable[StrPath] = None,
        validate: bool = True,
        absolute_paths: bool = True,
        derivatives: None | bool | DerivPathList | Mapping[str, DerivPathList] = False,
        config: Any = None,
        sources: Any = None,
        regex_search: bool = False,
        database_path: StrPath | None = None,
        reset_database: bool = False,
        indexer: BIDSLayoutIndexer | None = None,
        is_derivative: bool = False,
        **indexer_kwargs: Any,
    ):
        if is_derivative:
            derivatives = self._handle_is_derivatives(root, derivatives)
            root = None
        if config is not None:
            warnings.warn("config=... is not currently handled")
        if sources is not None:
            warnings.warn("sources=... does not have any effect")
        if indexer is None:
            indexer = BIDSLayoutIndexer(**indexer_kwargs)
        else:
            validate = indexer.validate
        if indexer.filters:
            msg = "BIDSLayoutIndexer(filters=...) is not currently handled"
            warnings.warn(msg)
        if indexer.ignore:
            msg = "BIDSLayoutIndexer(ignore=...) is not currently handled"
            warnings.warn(msg)
        if indexer.force_index:
            msg = "BIDSLayoutIndexer(force_index=...) is not currently handled"
            warnings.warn(msg)
        if indexer.config_filename is not None:
            msg = "BIDSLayoutIndexer(config_filename=...) is not currently handled"
            warnings.warn(msg)

        self._indexer = indexer
        self._validate = validate

        self._layout = BidsLayout(
            root,
            derivatives=derivatives,
            validate=validate,
            cache=database_path,
            reset_cache=reset_database,
        )
        if indexer.index_metadata:
            self._layout.index_metadata()
        self.regex_search = regex_search

    @staticmethod
    def _find_derivatives(root: Path):
        for d in root.iterdir():
            if d.joinpath("dataset_description.json").exists():
                yield d

    @classmethod
    def _handle_is_derivatives(
        cls,
        root: StrPath | Iterable[StrPath] | None,
        derivatives: None | bool | DerivPathList | Mapping[str, DerivPathList],
    ) -> Path | list[Path | StrPath]:
        if root is None:
            msg = "Root must be provided if is_derivatives=True"
            raise CompatibilityError(msg)
        try:
            root = Path(root)  # type: ignore
        except TypeError as err:
            msg = "A single root must be provided if is_derivatives=True"
            raise CompatibilityError(msg) from err
        if isinstance(derivatives, (dict, Mapping)):
            msg = "Derivatives cannot be specified as a dict if is_derivatives=True"
            raise CompatibilityError(msg)

        if derivatives is True:
            return [root, *cls._find_derivatives(root)]
        if derivatives is False or derivatives is None:
            return root

        try:
            return [root, Path(derivatives)]  # type: ignore
        except TypeError:
            return [root, *derivatives]  # type: ignore

    def _normalize_filters(self, filters: dict[str, PybidsFilterType]):
        def remap(val: PybidsFilterType) -> str | bool | None | list[str | bool | None]:
            if isinstance(val, (str, bool, int)):
                return val
            elif isinstance(val, Query):
                if val == Query.ANY or val == Query.REQUIRED:
                    return True
                elif val == Query.OPTIONAL:
                    return None
                elif val == Query.NONE:
                    return False
                else:
                    raise ValueError(f"Unrecognized query item: {val}")
            elif val is None:
                return False
            else:
                return [remap(v) for v in val]  # type: ignore

        result: dict[str, FilterType] = {}
        for key, val in filters.items():
            result[key] = remap(val)
        print(result)
        return result

    def __getattr__(self, key: str):
        """Dynamically inspect missing methods for get_<entity>() calls
        and return a partial function of get() if a match is found."""
        # Code adapted from
        # https://github.com/bids-standard/pybids/blob/master/bids/layout/layout.py
        if key.startswith("get_"):
            ent_name = key.replace("get_", "")
            if ent_name not in self._layout.entities and ent_name[-1] == "s":
                # try a couple plurals to replicate the inflect engine used in pybids
                for singu in (ent_name[:-1], ent_name[:-2], ent_name[:-3] + "y"):
                    if singu in self._layout.entities:
                        ent_name = singu
                        break
                else:
                    raise AttributeError(
                        f"'get_{ent_name}' can't be called because '{ent_name}' isn't "
                        "a recognized entity name."
                    )
            partial: Callable[..., list[str]] = (
                lambda **kwargs: self._layout.filter(scope=kwargs.pop("scope", None))
                .get(**kwargs)
                .entities[ent_name]
            )
            return partial
        # Spit out default message if we get this far
        return getattr(self._layout, key)

    def __repr__(self):
        return repr(self._layout)

    @property
    def root(self):
        return self._layout.root

    @property
    def description(self):
        return self._layout.description

    @property
    def session(self):
        raise CompatibilityError(".session has no meaning in rsbids")

    @property
    def config(self):
        raise CompatibilityError("This property has not been implemented yet")

    @cached_property
    def files(self):
        return self.get_files()

    @property
    def connection_manager(self):
        raise CompatibilityError(
            "BIDSLayout.connection_manager is not implemented in rsbids"
        )

    @classmethod
    def load(cls, database_path: StrPath):
        """Load index from database path. Initialization parameters are set to
        those found in database_path JSON sidecar.

        Parameters
        ----------
        database_path : str, Path
            The path to the desired database folder. If a relative path is
            passed, it is assumed to be relative to the BIDSLayout root
            directory.
        """
        obj = object.__new__(cls)
        obj._layout = BidsLayout.load(database_path)
        obj.regex_search = False
        obj._indexer = None
        obj._validate = None
        return obj

    def save(self, database_path: StrPath, replace_connection: bool | None = None):
        """Save the current index as a SQLite3 DB at the specified location.

        Note: This is only necessary if a database_path was not specified
        at initialization, and the user now wants to save the index.
        If a database_path was specified originally, there is no need to
        re-save using this method.

        Parameters
        ----------
        database_path : str
            The path to the desired database folder. By default,
            uses .db_cache. If a relative path is passed, it is assumed to
            be relative to the BIDSLayout root directory.
        replace_connection : bool, optional
            If True, the newly created database will
            be used for all subsequent connections. This means that any
            changes to the index made after the .save() call will be
            reflected in the database file. If False, the previous database
            will continue to be used, and any subsequent changes will not
            be reflected in the new file unless save() is explicitly called
            again.
        """
        if replace_connection is not None:
            warnings.warn("replace_connection=... has no effect in rsbids")
        self._layout.save(database_path)

    def get_entities(
        self, scope: str | Iterable[str] = "all", metadata: bool | None = None
    ) -> dict[str, list[str]]:
        """Get entities for all layouts in the specified scope.

        Parameters
        ----------
        scope : str
            The scope of the search space. Indicates which
            BIDSLayouts' entities to extract.
            See :obj:`bids.layout.BIDSLayout.get` docstring for valid values.

        metadata : bool or None
            By default (None), all available entities
            are returned. If True, only entities found in metadata files
            (and not defined for filenames) are returned. If False, only
            entities defined for filenames (and not those found in JSON
            sidecars) are returned.

        Returns
        -------
        dict
            Dictionary where keys are entity names and
            values are Entity instances.
        """
        try:
            self._layout.metadata
        except AttributeError:
            metadata = False
        view = self._layout.filter(scope=scope) if scope != "all" else self._layout
        if metadata is False:
            return view.entities
        if metadata is True:
            return view.metadata
        result = view.entities
        result.update(view.metadata)
        return result

    def get_files(self, scope: str | Iterable[str] = "all"):
        """Get BIDSFiles for all layouts in the specified scope.

        Parameters
        ----------
        scope : str
            The scope of the search space. Indicates which
            BIDSLayouts' entities to extract.
            See :obj:`bids.layout.BIDSLayout.get` docstring for valid values.


        Returns:
            A dict, where keys are file paths and values
            are :obj:`bids.layout.BIDSFile` instances.

        """
        view = self._layout.filter(scope=scope) if scope != "all" else self._layout
        return {
            str(f): BIDSFile._from_bidspath(f, self._layout)  # type: ignore
            for f in view
        }

    def parse_file_entities(
        self,
        filename: StrPath,
        scope: str = "all",
        entities=None,
        config=None,
        include_unmatched: bool = False,
    ):
        """Parse the passed filename for entity/value pairs.

        Parameters
        ----------
        filename : str
            The filename to parse for entity values
        scope : str or list, optional
            The scope of the search space. Indicates which BIDSLayouts'
            entities to extract. See :obj:`bids.layout.BIDSLayout.get`
            docstring for valid values. By default, extracts all entities.
        entities : list or None, optional
            An optional list of Entity instances to use in
            extraction. If passed, the scope and config arguments are
            ignored, and only the Entities in this list are used.
        config : str or :obj:`bids.layout.models.Config` or list or None, optional
            One or more :obj:`bids.layout.models.Config` objects, or paths
            to JSON config files on disk, containing the Entity definitions
            to use in extraction. If passed, scope is ignored.
        include_unmatched : bool, optional
            If True, unmatched entities are included
            in the returned dict, with values set to None. If False
            (default), unmatched entities are ignored.

        Returns
        -------
        dict
            Dictionary where keys are Entity names and values are the
            values extracted from the filename.
        """
        raise CompatibilityError("Not yet implemented")

    def add_derivatives(
        self,
        path: StrPath | Iterable[StrPath],
        parent_database_path: StrPath | None = None,
        **kwargs: Any,
    ):
        """Add BIDS-Derivatives datasets to tracking.

        Parameters
        ----------
        path : str or list
            One or more paths to BIDS-Derivatives datasets.
            Each path can point to either a derivatives/ directory
            containing one or more pipeline directories, or to a single
            pipeline directory (e.g., derivatives/fmriprep).
        parent_database_path : str or Path
            If not None, use the pipeline name from the dataset_description.json
            file as the database folder name to nest within the parent database
            folder name to write out derivative index to.
        kwargs : dict
            Optional keyword arguments to pass on to
            BIDSLayout() when initializing each of the derivative datasets.

        Notes
        -----
        Every derivatives directory intended for indexing MUST contain a
        valid dataset_description.json file. See the BIDS-Derivatives
        specification for details.
        """
        if self._indexer is None or self._validate is None:
            raise CompatibilityError(
                "BIDSLayout.add_derivatives cannot be used on a database-loaded "
                "instance"
            )
        warnings.warn(
            "BIDSLayout.add_derivatives() innefficiently reindexes the entire layout "
            "and is not compatible with database_dir. Prefer to add derivatives via "
            f"the constructor (BIDSLayout(derivatives={path}))"
        )
        if parent_database_path is not None:
            warnings.warn(
                "BIDSLayout.add_derivative(parent_database_path=...) has no effect"
            )
        try:
            derivatives = self._layout.derivatives.roots
        except ValueError:
            derivatives = []
        try:
            derivatives.append(Path(path))
        except TypeError:
            derivatives.extend(path)
        self._layout = self.__class__(
            self.root,
            validate=self._validate,
            derivatives=derivatives,
            indexer=self._indexer,
        )._layout

    def to_df(self, metadata: bool = False, **filters: PybidsFilterType):
        """Return information for BIDSFiles tracked in Layout as pd.DataFrame.

        Parameters
        ----------
        metadata : bool, optional
            If True, includes columns for all metadata fields.
            If False, only filename-based entities are included as columns.
        filters : dict, optional
            Optional keyword arguments passed on to get(). This allows
            one to easily select only a subset of files for export.

        Returns
        -------
        :obj:`pandas.DataFrame`
            A pandas DataFrame, where each row is a file, and each column is
            a tracked entity. NaNs are injected whenever a file has no
            value for a given attribute.

        """
        raise CompatibilityError("Not yet implemented")

    def get(
        self,
        return_type: Literal["object", "file", "dir", "id"] = "object",
        target: str | None = None,
        scope: str | Iterable[str] = "all",
        regex_search: bool | None = None,
        absolute_paths: bool | None = None,
        invalid_filters: Literal["error", "drop", "allow"] | None = None,
        **filters: str | bool | None | Iterable[str | bool],
    ):
        """Retrieve files and/or metadata from the current Layout.

        Parameters
        ----------
        return_type : str, optional
            Type of result to return. Valid values:
            'object' (default): return a list of matching BIDSFile objects.
            'file' or 'filename': return a list of matching filenames.
            'dir': return a list of directories.
            'id': return a list of unique IDs. Must be used together
                  with a valid target.
        target : str, optional
            Optional name of the target entity to get results for
            (only used if return_type is 'dir' or 'id').
        scope : str or list, optional
            Scope of the search space. If passed, only
            nodes/directories that match the specified scope will be
            searched. Possible values include:
            'all' (default): search all available directories.
            'derivatives': search all derivatives directories.
            'raw': search only BIDS-Raw directories.
            'self': search only the directly called BIDSLayout.
            <PipelineName>: the name of a BIDS-Derivatives pipeline.
        regex_search : bool or None, optional
            Whether to require exact matching
            (False) or regex search (True) when comparing the query string
            to each entity.
        absolute_paths : bool, optional
            Optionally override the instance-wide option
            to report either absolute or relative (to the top of the
            dataset) paths. If None, will fall back on the value specified
            at BIDSLayout initialization.
        invalid_filters (str): Controls behavior when named filters are
            encountered that don't exist in the database (e.g., in the case of
            a typo like subbject='0.1'). Valid values:
                'error' (default): Raise an explicit error.
                'drop': Silently drop invalid filters (equivalent to not having
                    passed them as arguments in the first place).
                'allow': Include the invalid filters in the query, resulting
                    in no results being returned.
        filters : dict
            Any optional key/values to filter the entities on.
            Keys are entity names, values are regexes to filter on. For
            example, passing filters={'subject': 'sub-[12]'} would return
            only files that match the first two subjects. In addition to
            ordinary data types, the following enums are defined (in the
            Query class):
                * Query.NONE: The named entity must not be defined.
                * Query.ANY: the named entity must be defined, but can have any
                    value.

        Returns
        -------
        list of :obj:`bids.layout.BIDSFile` or str
            A list of BIDSFiles (default) or strings (see return_type).
        """
        if invalid_filters is not None:
            warnings.warn("invalid_filters=... is not yet implemented")
        if absolute_paths is not None:
            warnings.warn("absolute_paths=... is not yet implemented")
        if regex_search is not None:
            warnings.warn("regex_search=... is not yet implemented")
        if return_type == "dir":
            raise CompatibilityError('return_type="dir" is not yet implemented')
        filters = self._normalize_filters(filters)
        view = self._layout.filter(scope=scope).get(**filters)
        if return_type == "id":
            if target is None:
                msg = (
                    'If return type is "id" or "dir", a valid target entity must also '
                    "be specified"
                )
                raise ValueError(msg)
            return view.entities[target]
        if return_type == "file":
            return [str(f) for f in view]
        return [BIDSFile._from_bidspath(p, self._layout) for p in view]  # type: ignore

    def get_file(self, filename: StrPath, scope: str | Iterable[str] = "all"):
        """Return the BIDSFile object with the specified path.

        Parameters
        ----------
        filename : str
            The path of the file to retrieve. Must be either an absolute path,
            or relative to the root of this BIDSLayout.
        scope : str or list, optional
            Scope of the search space. If passed, only BIDSLayouts that match
            the specified scope will be searched. See :obj:`BIDSLayout.get`
            docstring for valid values. Default is 'all'.

        Returns
        -------
        :obj:`bids.layout.BIDSFile` or None
            File found, or None if no match was found.
        """
        files = self.get_files(scope=scope)
        file = Path(filename)
        if not file.is_absolute():
            file = Path(self.root, file).absolute()
        return files.get(str(file))

    def get_collections(
        self,
        level: Literal["run", "session", "subject", "dataset"],
        types: str | Iterable[str] | None = None,
        variables: Iterable[str] | None = None,
        merge: bool = False,
        sampling_rate: int | str | None = None,
        skip_empty: bool = False,
        **kwargs: Any,
    ):
        """Return one or more variable Collections in the BIDS project.

        Parameters
        ----------
        level : {'run', 'session', 'subject', 'dataset'}
            The level of analysis to return variables for.
            Must be one of 'run', 'session','subject', or 'dataset'.
        types : str or list
            Types of variables to retrieve. All valid values reflect the
            filename stipulated in the BIDS spec for each kind of variable.
            Valid values include: 'events', 'physio', 'stim', 'scans',
            'participants', 'sessions', and 'regressors'. Default is None.
        variables : list
            Optional list of variables names to return. If None, all available
            variables are returned.
        merge : bool
            If True, variables are merged across all observations of the
            current level. E.g., if level='subject', variables from all
            subjects will be merged into a single collection. If False, each
            observation is handled separately, and the result is returned
            as a list.
        sampling_rate : int or str
            If level='run', the sampling rate to pass onto the returned
            :obj:`bids.variables.collections.BIDSRunVariableCollection`.
        skip_empty : bool
            Whether or not to skip empty Variables (i.e., where there are no
            rows/records in a file after applying any filtering operations
            like dropping NaNs).
        kwargs
            Optional additional arguments to pass onto
            :obj:`bids.variables.io.load_variables`.

        Returns
        -------
        list of :obj:`bids.variables.collections.BIDSVariableCollection`
            or :obj:`bids.variables.collections.BIDSVariableCollection`
            A list if merge=False;
            a single :obj:`bids.variables.collections.BIDSVariableCollection`
            if merge=True.

        """
        raise CompatibilityError("get_collections() not yet implemented")

    def get_metadata(
        self,
        path: StrPath,
        include_entities: bool = False,
        scope: str | list[str] = "all",
    ):
        """Return metadata found in JSON sidecars for the specified file.

        Parameters
        ----------
        path : str
            Path to the file to get metadata for.
        include_entities : bool, optional
            If True, all available entities extracted
            from the filename (rather than JSON sidecars) are included in
            the returned metadata dictionary.
        scope : str or list, optional
            The scope of the search space. Each element must
            be one of 'all', 'raw', 'self', 'derivatives', or a
            BIDS-Derivatives pipeline name. Defaults to searching all
            available datasets.

        Returns
        -------
        dict
            A dictionary of key/value pairs extracted from all of the
            target file's associated JSON sidecars.

        Notes
        -----
        A dictionary containing metadata extracted from all matching .json
        files is returned. In cases where the same key is found in multiple
        files, the values in files closer to the input filename will take
        precedence, per the inheritance rules in the BIDS specification.

        """
        file = self.get_file(path, scope=scope)
        metadata = file.get_metadata()
        if include_entities:
            return {**file.get_entities(), **metadata}
        return metadata

    def get_dataset_description(
        self, scope: str | list[str] = "self", all_: bool = False
    ):
        """Return contents of dataset_description.json.

        Parameters
        ----------
        scope : str
            The scope of the search space. Only descriptions of
            BIDSLayouts that match the specified scope will be returned.
            See :obj:`bids.layout.BIDSLayout.get` docstring for valid values.
            Defaults to 'self' --i.e., returns the dataset_description.json
            file for only the directly-called BIDSLayout.
        all_ : bool
            If True, returns a list containing descriptions for
            all matching layouts. If False (default), returns for only the
            first matching layout.

        Returns
        -------
        dict or list of dict
            a dictionary or list of dictionaries (depending on all_).
        """
        if all_ == True:
            msg = "get_dataset_description(all_=True) not yet supported"
            raise CompatibilityError(msg)
        if scope == "self" or scope == "all":
            return self._layout.description
        return self._layout.filter(scope=scope).description

    def get_nearest(
        self,
        path: StrPath,
        return_type: Literal["filename", "tuple"] = "filename",
        strict: bool = True,
        all_: bool = False,
        ignore_strict_entities: str | Iterable[str] = "extension",
        full_search: bool = False,
        **filters: PybidsFilterType,
    ):
        """Walk up file tree from specified path and return nearest matching file(s).

        Parameters
        ----------
        path (str): The file to search from.
        return_type (str): What to return; must be one of 'filename'
            (default) or 'tuple'.
        strict (bool): When True, all entities present in both the input
            path and the target file(s) must match perfectly. When False,
            files will be ordered by the number of matching entities, and
            partial matches will be allowed.
        all_ (bool): When True, returns all matching files. When False
            (default), only returns the first match.
        ignore_strict_entities (str, list): Optional entity/entities to
            exclude from strict matching when strict is True. This allows
            one to search, e.g., for files of a different type while
            matching all other entities perfectly by passing
            ignore_strict_entities=['type']. Ignores extension by default.
        full_search (bool): If True, searches all indexed files, even if
            they don't share a common root with the provided path. If
            False, only files that share a common root will be scanned.
        filters : dict
            Optional keywords to pass on to :obj:`bids.layout.BIDSLayout.get`.
        """
        # This function is waiting on an api to filter the layout "above" and "below" a
        # given file (using filetree)
        raise CompatibilityError("get_nearest() not yet implemented")

    def get_bvec(self, path: StrPath, **kwargs: PybidsFilterType):
        """Get bvec file for passed path."""
        raise CompatibilityError("get_bvec() not yet implemented")

    def get_bval(self, path: StrPath, **kwargs: PybidsFilterType):
        """Get bval file for passed path."""
        raise CompatibilityError("get_bval() not yet implemented")

    def get_fieldmap(self, path: StrPath, return_list: bool = False):
        """Get fieldmap(s) for specified path."""
        raise CompatibilityError("get_field_map() not yet implemented")

    def get_tr(self, derivatives: bool = False, **filters: PybidsFilterType):
        """Return the scanning repetition time (TR) for one or more runs.

        Parameters
        ----------
        derivatives : bool
            If True, also checks derivatives images.
        filters : dict
            Optional keywords used to constrain the selected runs.
            Can be any arguments valid for a .get call (e.g., BIDS entities
            or JSON sidecar keys).

        Returns
        -------
        float
            A single float.

        Notes
        -----
        Raises an exception if more than one unique TR is found.
        """
        raise CompatibilityError("get_tr() not yet implemented")

    def build_path(
        self,
        source,
        path_patterns=None,
        strict=False,
        scope="all",
        validate=True,
        absolute_paths=None,
    ):
        """Construct a target filename for a file or dictionary of entities.

        Parameters
        ----------
        source : str or :obj:`bids.layout.BIDSFile` or dict
            The source data to use to construct the new file path.
            Must be one of:
            - A BIDSFile object
            - A string giving the path of a BIDSFile contained within the
              current Layout.
            - A dict of entities, with entity names in keys and values in
              values
        path_patterns : list
            Optional path patterns to use to construct
            the new file path. If None, the Layout-defined patterns will
            be used. Entities should be represented by the name
            surrounded by curly braces. Optional portions of the patterns
            should be denoted by square brackets. Entities that require a
            specific value for the pattern to match can pass them inside
            angle brackets. Default values can be assigned by specifying a string
            after the pipe operator. E.g., (e.g., {type<image>|bold} would
            only match the pattern if the entity 'type' was passed and its
            value is "image", otherwise the default value "bold" will be
            used).
                Example: 'sub-{subject}/[var-{name}/]{id}.csv'
                Result: 'sub-01/var-SES/1045.csv'
        strict : bool, optional
            If True, all entities must be matched inside a
            pattern in order to be a valid match. If False, extra entities
            will be ignored so long as all mandatory entities are found.
        scope : str or list, optional
            The scope of the search space. Indicates which
            BIDSLayouts' path patterns to use. See BIDSLayout docstring
            for valid values. By default, uses all available layouts. If
            two or more values are provided, the order determines the
            precedence of path patterns (i.e., earlier layouts will have
            higher precedence).
        validate : bool, optional
            If True, built path must pass BIDS validator. If
            False, no validation is attempted, and an invalid path may be
            returned (e.g., if an entity value contains a hyphen).
        absolute_paths : bool, optional
            Optionally override the instance-wide option
            to report either absolute or relative (to the top of the
            dataset) paths. If None, will fall back on the value specified
            at BIDSLayout initialization.
        """
        raise CompatibilityError("build_path() not yet implemented")

    def copy_files(
        self,
        files=None,
        path_patterns=None,
        symbolic_links=True,
        root=None,
        conflicts="fail",
        **kwargs,
    ):
        """Copy BIDSFile(s) to new locations.

        The new locations are defined by each BIDSFile's entities and the
        specified `path_patterns`.

        Parameters
        ----------
        files : list
            Optional list of BIDSFile objects to write out. If
            none provided, use files from running a get() query using
            remaining **kwargs.
        path_patterns : str or list
            Write patterns to pass to each file's write_file method.
        symbolic_links : bool
            Whether to copy each file as a symbolic link or a deep copy.
        root : str
            Optional root directory that all patterns are relative
            to. Defaults to dataset root.
        conflicts : str
            Defines the desired action when the output path already exists.
            Must be one of:
                'fail': raises an exception
                'skip' does nothing
                'overwrite': overwrites the existing file
                'append': adds a suffix to each file copy, starting with 1
        kwargs : dict
            Optional key word arguments to pass into a get() query.
        """
        raise CompatibilityError("copy_files() not yet implemented")

    def write_to_file(
        self,
        entities,
        path_patterns=None,
        contents=None,
        link_to=None,
        copy_from=None,
        content_mode="text",
        conflicts="fail",
        strict=False,
        validate=True,
    ):
        """Write data to a file defined by the passed entities and patterns.

        Parameters
        ----------
        entities : dict
            A dictionary of entities, with Entity names in
            keys and values for the desired file in values.
        path_patterns : list
            Optional path patterns to use when building
            the filename. If None, the Layout-defined patterns will be
            used.
        contents : object
            Contents to write to the generate file path.
            Can be any object serializable as text or binary data (as
            defined in the content_mode argument).
        link_to : str
            Optional path with which to create a symbolic link
            to. Used as an alternative to and takes priority over the
            contents argument.
        conflicts : str
            Defines the desired action when the output path already exists.
            Must be one of:
                'fail': raises an exception
                'skip' does nothing
                'overwrite': overwrites the existing file
                'append': adds a suffix to each file copy, starting with 1
        strict : bool
            If True, all entities must be matched inside a
            pattern in order to be a valid match. If False, extra entities
            will be ignored so long as all mandatory entities are found.
        validate : bool
            If True, built path must pass BIDS validator. If
            False, no validation is attempted, and an invalid path may be
            returned (e.g., if an entity value contains a hyphen).
        """
        raise CompatibilityError("write_to_file() not yet implemented")


class Query(enum.Enum):
    """Enums for use with BIDSLayout.get()."""

    NONE = 1  # Entity must not be present
    REQUIRED = ANY = 2  # Entity must be defined, but with an arbitrary value
    OPTIONAL = 3  # Entity may or may not be defined
