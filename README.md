# rsbids

[![Version](https://img.shields.io/github/v/tag/pvandyken/rsbids?label=version)](https://pypi.org/project/rsbids/)
[![Python versions](https://img.shields.io/pypi/pyversions/rsbids)](https://pypi.org/project/rsbids/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`rsbids` is a rust implementation of [`pybids`](https://github.com/bids-standard/pybids), currently under active development. It offers vastly improved runtimes compared to other bids indexers (benchmarks to come), a streamlined core api, and a pybids compatibility api.

**`rsbids` is currently in alpha**. Most of the core pybids features are implemented, however, there is little to no automated testing or documentation. It has only rudimentary validation and no configurability. Pybids compatibility has been implemented for much of `pybids.layout.layout`, `pybids.layout.indexers`, and `pybids.layout.models`. Not all features are available, however. Whenever possible, a `CompatibilityError` or warning will be raised when these features are encountered. Finally, api stability is not guarenteed for any aspect of the api.

The alpha period is an opportunity to test and experiment. Community engagement and feedback is highly valued, and will have an impact on future development. In the immediate future, work will focus on testing, stability, and basic configuration/validation. However, any feature ideas and feedback on the api are welcome. (Note that there's a number of issues I'm already aware of, so be sure to read this document before leaving bug reports).

## Installation

`rsbids` is precompiled for most environments, so installation is generally as simple as:

```sh
pip install rsbids
```

On more exotic linux versions, or custom environments such as HPCs, the precompiled wheels may not work and `rsbids` will need to be compiled. Fortunately, this is generally really straight forward.

First, ensure rust is installed on your system. You can follow the simple instructions from [rustup](https://rustup.rs/) to install directly, or on an HPC, load up rust using its software version control (e.g. for `lmod`: `module load rust`). Then just pip-install as normal, and `rsbids` should automatically be compiled (note that it may take several minutes).


## Benchmarks

Benchmarks are calculated on the openly available [_HBN EO/EC task_ dataset](https://openneuro.org/datasets/ds004186/versions/2.0.0), consisting of 177,065 files, including metadata. `rsbids` is compared to [`pybids`](https://github.com/bids-standard/pybids), [`ancpbids-bids`](https://github.com/ANCPLabOldenburg/ancp-bids), and [`bids2table`](https://github.com/cmi-dair/bids2table). The code for running the benchmarks and generating the figure can be found at the [rsbids-benchmark](https://github.com/pvandyken/rsbids-benchmark.git) repository. More information on the method and tasks can be found there.

![Benchmarks for rsbids](https://github.com/pvandyken/rsbids-benchmark/blob/07b1fdeee5be4ceda03737f793bb7e38042f03d5/assets/benchmarks.png)


## Pybids Compatibility

A compability api can be found under `rsbids.pybids`. So in general, you can:

```py
# replace
from bids import BIDSLayout

# with
from rsbids.pybids import BIDSLayout
```

As of now, the indexing and querying methods on `BIDSLayout` are implemented with some limitations:

- `BIDSLayout(validate=True)` redirects into `rsbids.BidsLayout(validate=True)`, which has a different meaning (validation will eventually be equivalent to pybids, but this needs to be developed)
- No regex based ignoring of files is possible
- `BIDSLayoutIndexer` can be constructed and used to skip metadata indexing, but all the other fields do nothing.
- Calling `BIDSLayout.get()` returns a list of `BIDSPaths` as before. The API for this compatibility `BIDSPath` is not yet complete (no `.copy`, `.get_associations`, or `.relpath`)
- Regex searching via `.get()` is not yet supported. `return_type="dir"` is also not supported
- Entity retrieval methods return a mocked version of [`Entity`](https://bids-standard.github.io/pybids/generated/bids.layout.Entity.html#bids.layout.Entity) (rsbids has no such `Entity` class). The methods and properties of `Entity` are all implemented, however, because `rsbids` does not use regex when parsing paths, it can only "guess" at the `pattern` and `regex` properties of `Entity`. These should not be trusted for any automated use.
- The methods searching for associated files on `BIDSLayout` are not yet implemented (including `get_bval`, `get_filedmap`, etc). `get_metadata` DOES work.
- Path building methods and data copying methods are also not implemented (e.g. `build_path`, `write_to_file`)
- `database_path` and `reset_database` are both implemented, but use `rsbids` caches, not `pybids` databases. So they won't read your previous pybids databases! (Because `rsbids` is so fast, caching should not be necessary unless your files are on a network filesystem).

That being said, we encourage users to try the new API. Feel free to leave feedback regarding any potential improvements!

## Notable differences from pybids

Along with the substantial speed boost, `rsbids` optimizes many aspects of the `pybids` api:

### Chained querying

`rsbids.BidsLayout.get()` returns a new instance of `rsbids.BidsLayout`. Calls to `.get` can thus be chained:

```python
view = layout.get(suffix="T1w")

# later

view.get(subject="01")
```

Because of this, most of the methods in `pybids.BIDSLayout` can be replaced by an appropriate combination of methods:

```python
# pybids
layout.get_subjects(suffix="events", task="stroop")

# rsbids
layout.get(suffix="events", task="stroop").entities["subject"]
```

```python
# pybids
layout.get_files(scope="fmriprep")

# rsbids
layout.filter(scope="fmriprep")
```

```python
# pybids
for f in layout.files:
    ...

# rsbids
for f in layout:
    ...
```


### Simplified single-file querying

`rsbids.BidsLayout` has the `.one` property, which errors out if the layout does not have exactly one path. If more than one path is present, the entities still to be filtered are listed in the error:

```python
# pybids (no error if more than one path)
layout.get(subject="001", session="02", suffix="dwi", extension=".nii.gz")[0]

# rsbids
layout.get(subject="001", session="02", suffix="dwi", extension=".nii.gz").one
```

### Seperate `.get()` and `.filter()` methods

`pybids` uses the `.get()` method as an omnibus query method. While convenient, it makes the method brittle because certain arguments are interpreted with special meaning (e.g. `scope`, `target`). This makes it challenging to add additional query methods (e.g. searching specificially by `pipeline` or file `root`).

With the split, arguments to `.get()` will always be interpreted as entity names (e.g. `subject`, `session`, `run`, etc) or metadata keys (e.g. `EchoTime`, etc). All other special search modes are handled by `.filter()`. Because each query returns a new layout, it's perfectly possible to chain these calls together, making an extremely flexible query interface.

`.get()` accepts the "short" names of entities in addition to their long version. For instance, the following calls are equivalent:

```python
layout.get(subject="001") == layout.get(sub=="001")
```

`.get()` also allows you to add a final `_` to entity names, dropping the `_` before matching. This is useful for querying python reserved words like `from`:

```py
layout.get(from="MNI")  # !!! Syntax Error
layout.get(from_="MNI")
```

`.filter()` currently takes the following arguments:

#### `root`

Root searches by dataset root, making it useful for multi-root layouts. It accepts either the complete root as a string, or glob patterns (e.g. `**/fmriprep-*`).

#### `scope`

Scope uses the same syntax as in pybids: `raw` and `self` both match the raw dataset, `derivatives` matches all derivative datasets, `<pipeline_name>` searches derivative datasets by pipeline names found in their `dataset_description.json`.

Note that the above uses of `scope` are primarily included for backward compatibility with `pybids`. There are (or will be) better, dedicated ways to achieve each of these searches. Moving forward, `scope` will be intended to index labelled derivatives (see below).

### Multi-root layouts

`pybids` supported single raw or root datasets with multiple, potentially nested derivative datasets. `rsbids` reimagines layouts as a flat collection of datasets, each tagged with various attributes. For example, one or more datasets may be `raw`, and the rest `derivative`. Datasets may be generated with one or more `pipeline`s and derive from one or more datasets. These attributes are (or will be) individually indexed and individually queryable.

Thus, `rsbids` allows multiple raw roots:

```python
# rsbids
layout = rsbids.BidsLayout(["root1", "root2"])
```

These roots can be then queried using roots:

```python
layout.filter(root="root1")
```

New to `rsbids`, derivatives can be labelled:

```python
#rsbids
layout = rsbids.BidsLayout(
    "dataset",
    derivatives={
        "proc1": "dataset/derivatives/proc1-v0.10.1",
        "anat": "dataset/derivatives/smriprep-v1.3",
    })
```

These labels can queried using `scope`:

```python
layout.filter(scope="anat")
```

All derivatives can be selected using `.derivatives`:

```python
layout.derivatives == layout.filter(scope="derivatives")
```

All dataset `roots` can be listed using with:

```python
layout.roots
```

If the dataset has a single raw root (with any number of derivatives), the `.root` attribute can be used to retrieve that root:

```python
layout = rsbids.BidsLayout(
    "dataset",
    derivatives={
        "proc1": "dataset/derivatives/proc1-v0.10.1",
        "anat": "dataset/derivatives/smriprep-v1.3",
    })

layout.root == "dataset"
```

If there is no raw root, but exactly one derivative root, `.root` will retrieve the derivative

```python
layout = rsbids.BidsLayout(
    "dataset",
    derivatives={
        "proc1": "dataset/derivatives/proc1-v0.10.1",
        "anat": "dataset/derivatives/smriprep-v1.3",
    })
layout.filter(scope="proc1").root == "dataset/derivatives/proc1-v0.10.1"
```

All other calls to `.root` will error:

```python
layout = rsbids.BidsLayout(
    "dataset",
    derivatives={
        "proc1": "dataset/derivatives/proc1-v0.10.1",
        "anat": "dataset/derivatives/smriprep-v1.3",
    })
layout.derivatives.root # !!! Error: multiple roots
```

The `.description` attribute works according to equivalent logic:

```python
layout = rsbids.BidsLayout(
    "dataset",
    derivatives={
        "proc1": "dataset/derivatives/proc1-v0.10.1",
        "anat": "dataset/derivatives/smriprep-v1.3",
    })
layout.description == <DatasetDescription>
```

_Note_: The error handling for `.description` and `.root` is still a bit janky. `DatasetDescription` reading has only preliminary support: the object is readonly, and values must be accessed as attributes using snakecase:

```python
layout.description.generated_by[0].name

layout.description["Name"] # !!! Error
```


### Metadata Indexing

`pybids` defaults to indexing the metadata, significantly increasing the time to index. `rsbids` defaults to not indexing, since in our experience, the metadata is not needed for most applications. Instead of requesting metadata using an argument on the `rsbids.BidsLayout` constructor, metadata is requested using the following method:

```py
layout = rsbids.BidsLayout("dataset").index_metadata()
```

This decouples metadata retrieval from layout construction, providing a few advantages:

- If you discover you later on need metadata, you don't have to reindex the entire layout (especially useful on network-attached filesystems with high latency)
- You can even index metadata when reading a layout from cache
- Functions consuming `BidsLayout` (e.g. from 3rd party apps) don't need to worry about whether metadata was indexed or not. If they need metadata, they can simply call `layout.index_metadata()`. If metadata is already indexed, the method will immediately return

The method returns back the same bids layout, so it can be easily chained:

```py
layout.index_metadata().get(EchoTime="...")
```


### dtypes

`pybids` associates each entity with a specific datatype. Most entities are strings, but some, such as `run`, are explicitely stored as integers.

`rsbids` stores all entities as strings. This simplifies the layout internals and ensures entities are saved nondestructively. For those used to querying runs with integers, however, fear not! `rsbids.get()` accepts integer queries for ALL entities:

```py
layout.get(subject=1)

# will match
#   sub-001_T1w.nii.gz
#   sub-01_T1w.nii.gz
#   sub-1_T1w.nii.gz
# but not
#   sub-Pre1_T1w.nii.gz
#   sub-Treatment001_T1w.nii.gz
```

If multiple valid matches are found, an error will be thrown.


### Flexible parsing algorithm

`rsbids` has two variants of its parsing algorithm. One looks for `entity-value` pairs specifically defined by the bids spec (similar to how pybids and all other bids indexers currently work). Invalid entities (`..._foobar-val_...`) are ignored. This mode is enabled by `rsbids.layout(..., validate=True)`, and gives a validation experience _somewhat_ similar to `pybids.BIDSLayout(..., validate=False, is_derivative=True)` (note that this will change in the future to match the `pybids` defaults).

The other parser is completely generic: it parses any path looking for `entity-value` combinations seperated by underscores (`_`). So long as the path structure looks _roughly_ bids-like, `rsbids` should correctly parse it, including missing extensions/suffixes, custom entities, any arbitrary value (so long as it has no `_`), custom datatypes, malformed directory structures, etc.

The flexible algorithm currently has **no** validation, so any path will be parsed into _something_ according to the algorithm. In the future, `rsbids` will allow for more fine-grained validation.

The details of the algorithm will be written at some point in the future. In summary, these are the main priorities:

1. Any valid bids path **MUST** be parsed correctly (if it's not, it's a bug)
2. Any almost-valid bids path **SHOULD** be parsed correctly. This include paths with one or a few of:
  - Custom entity
  - Custom datatype
  - Custom entity as a directory if it's also in the file name

Finally, any path bits that can't be interpreted as `key-value` pairs will generally be saved as `parts` (e.g. `sub-001_somepart_ses-1_...`). In the future, `rsbids` will supporting querying for these parts, making it potentially useful even for severely non-bids-compliant datasets.

