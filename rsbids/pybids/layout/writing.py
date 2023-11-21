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
from typing import TYPE_CHECKING, Iterable, Literal, Mapping

from rsbids.exceptions import CompatibilityError

if TYPE_CHECKING:
    from rsbids._lib import StrPath


def build_path(
    entities: Mapping[str, Iterable[str]],
    path_patterns: str | Iterable[str],
    strict: bool = False,
):
    """
    Constructs a path given a set of entities and a list of potential
    filename patterns to use.

    Parameters
    ----------
    entities : :obj:`dict`
        A dictionary mapping entity names to entity values.
        Entities with ``None`` or empty-string value will be removed.
        Otherwise, entities will be cast to string values, therefore
        if any format is expected (e.g., zero-padded integers), the
        value should be formatted.
    path_patterns : :obj:`str` or :obj:`list`
        One or more filename patterns to write
        the file to. Entities should be represented by the name
        surrounded by curly braces. Optional portions of the patterns
        should be denoted by square brackets. Entities that require a
        specific value for the pattern to match can pass them inside
        angle brackets. Default values can be assigned by specifying a string after
        the pipe operator. E.g., (e.g., {type<image>|bold} would only match
        the pattern if the entity 'type' was passed and its value is
        "image", otherwise the default value "bold" will be used).
    strict : :obj:`bool`
        If True, all passed entities must be matched inside a
        pattern in order to be a valid match. If False, extra entities will
        be ignored so long as all mandatory entities are found.

    Returns
    -------
    A constructed path for this file based on the provided patterns, or
    ``None`` if no path was built given the combination of entities and patterns.

    Examples
    --------
    >>> entities = {
    ...     'extension': '.nii',
    ...     'space': 'MNI',
    ...     'subject': '001',
    ...     'suffix': 'inplaneT2',
    ... }
    >>> patterns = ['sub-{subject}[/ses-{session}]/anat/sub-{subject}[_ses-{session}]'
    ...             '[_acq-{acquisition}][_ce-{ceagent}][_rec-{reconstruction}]_'
    ...             '{suffix<T[12]w|T1rho|T[12]map|T2star|FLAIR|FLASH|PDmap|PD|PDT2|'
    ...             'inplaneT[12]|angio>}{extension<.nii|.nii.gz|.json>|.nii.gz}',
    ...             'sub-{subject}[/ses-{session}]/anat/sub-{subject}[_ses-{session}]'
    ...             '[_acq-{acquisition}][_ce-{ceagent}][_rec-{reconstruction}]'
    ...             '[_space-{space}][_desc-{desc}]_{suffix<T1w|T2w|T1rho|T1map|T2map|'
    ...             'T2star|FLAIR|FLASH|PDmap|PD|PDT2|inplaneT[12]|angio>}'
    ...             '{extension<.nii|.nii.gz|.json>|.nii.gz}']
    >>> build_path(entities, patterns)
    'sub-001/anat/sub-001_inplaneT2.nii'

    >>> build_path(entities, patterns, strict=True)
    'sub-001/anat/sub-001_space-MNI_inplaneT2.nii'

    >>> entities['space'] = None
    >>> build_path(entities, patterns, strict=True)
    'sub-001/anat/sub-001_inplaneT2.nii'

    >>> # If some entity is set to None, they are dropped
    >>> entities['extension'] = None
    >>> build_path(entities, patterns, strict=True)
    'sub-001/anat/sub-001_inplaneT2.nii.gz'

    >>> # If some entity is set to empty-string, they are dropped
    >>> entities['extension'] = ''
    >>> build_path(entities, patterns, strict=True)
    'sub-001/anat/sub-001_inplaneT2.nii.gz'

    >>> # If some selector is not in the pattern, skip it...
    >>> entities['datatype'] = 'anat'
    >>> build_path(entities, patterns)
    'sub-001/anat/sub-001_inplaneT2.nii.gz'

    >>> # ... unless the pattern should be strictly matched
    >>> entities['datatype'] = 'anat'
    >>> build_path(entities, patterns, strict=True) is None
    True

    >>> # If the value of an entity is not valid, do not match the pattern
    >>> entities['suffix'] = 'bold'
    >>> build_path(entities, patterns) is None
    True

    >>> entities = {
    ...     'extension': '.bvec',
    ...     'subject': '001',
    ... }
    >>> patterns = (
    ...     "sub-{subject}[/ses-{session}]/{datatype|dwi}/sub-{subject}[_ses-{session}]"
    ...     "[_acq-{acquisition}]_{suffix|dwi}{extension<.bval|.bvec|.json|.nii.gz|.nii>|.nii.gz}"
    ... )
    >>> build_path(entities, patterns, strict=True)
    'sub-001/dwi/sub-001_dwi.bvec'

    >>> # Lists of entities are expanded
    >>> entities = {
    ...     'extension': '.bvec',
    ...     'subject': ['%02d' % i for i in range(1, 4)],
    ... }
    >>> build_path(entities, patterns, strict=True)
    ['sub-01/dwi/sub-01_dwi.bvec', 'sub-02/dwi/sub-02_dwi.bvec', 'sub-03/dwi/sub-03_dwi.bvec']

    """
    CompatibilityError("bids_path() is not yet implemented")


def write_to_file(
    path: StrPath,
    contents: str | bytes | None = None,
    link_to: StrPath | None = None,
    copy_from: StrPath | None = None,
    content_mode: Literal["text", "binary"] = "text",
    root: StrPath | None = None,
    conflicts: Literal["fail", "skip", "overwrite", "append"] = "fail",
):
    """
    Writes provided contents to a new path, or copies from an old path.

    Parameters
    ----------
    path : str
        Destination path of the desired contents.
    contents : str
        Raw text or binary encoded string of contents to write
        to the new path.
    link_to : str
        Optional path with which to create a symbolic link to.
        Used as an alternative to, and takes priority over, the contents
        argument.
    copy_from : str
        Optional filename to copy to new location. Used an alternative to, and
        takes priority over, the contents argument.
    content_mode : {'text', 'binary'}
        Either 'text' or 'binary' to indicate the writing
        mode for the new file. Only relevant if contents is provided.
    root : str
        Optional root directory that all patterns are relative
        to. Defaults to current working directory.
    conflicts : {'fail', 'skip', 'overwrite', 'append'}
        One of 'fail', 'skip', 'overwrite', or 'append'
        that defines the desired action when the output path already
        exists. 'fail' raises an exception; 'skip' does nothing;
        'overwrite' overwrites the existing file; 'append' adds a suffix
        to each file copy, starting with 1. Default is 'fail'.
    """
    CompatibilityError("write_to_file() is not yet implemented")