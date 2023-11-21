from typing import Callable, ParamSpec, Protocol, TypeVar

import bids

from rsbids import BidsLayout
import ancpbids
from bids2table import bids2table

_T = TypeVar("_T")
_T_contra = TypeVar("_T_contra", contravariant=True)
_P = ParamSpec("_P")


class Benchmark(Protocol):
    def __call__(
        self, func: Callable[_P, _T], *args: _P.args, **kwargs: _P.kwargs
    ) -> _T:
        ...

def test_benchmark_validate(benchmark: Benchmark):
    benchmark(BidsLayout, "topsy", validate=True)

def test_benchmark_rsbids_indexing(benchmark: Benchmark):
    benchmark(BidsLayout, "topsy")


def test_benchmark_ancp_indexing(benchmark: Benchmark):
    benchmark(ancpbids.load_dataset, "topsy")


def test_benchmark_pybids_indexing(benchmark: Benchmark):
    benchmark(bids.BIDSLayout, "topsy")

def test_benchmark_bids2table_indexing(benchmark: Benchmark):
    benchmark(bids2table, "topsy")


def test_benchmark_rsbids_query(benchmark: Benchmark):
    layout = BidsLayout("topsy")
    benchmark(layout.get, subject="001")


def test_benchmark_ancp_query(benchmark: Benchmark):
    layout = ancpbids.load_dataset("topsy")
    benchmark(layout.query, subject="001")


def test_benchmark_pybids_query(benchmark: Benchmark):
    layout = bids.BIDSLayout("topsy")
    benchmark(layout.get, subject="001")


def test_benchmark_bids2table_query(benchmark: Benchmark):
    layout = bids2table("topsy")
    benchmark(layout.filter, "subject", "001")


def test_benchmark_rsbids_large_query(benchmark: Benchmark):
    layout = BidsLayout("topsy")
    benchmark(
        layout.get,
        subject=["001", "002", "003", "004", "005"],
        suffix="T1w",
        session="1",
    )


def test_benchmark_ancp_large_query(benchmark: Benchmark):
    layout = ancpbids.load_dataset("topsy")
    benchmark(
        layout.query,
        subject=["001", "002", "003", "004", "005"],
        suffix="T1w",
        session="1",
    )


def test_benchmark_pybids_large_query(benchmark: Benchmark):
    layout = bids.BIDSLayout("topsy")
    benchmark(
        layout.get,
        subject=["001", "002", "003", "004", "005"],
        suffix="T1w",
        session="1",
    )


def test_benchmark_bids2table_large_query(benchmark: Benchmark):
    layout = bids2table("topsy")
    benchmark(
        lambda: layout.filter("subject", items=["001", "002", "003", "004", "005"])
        .filter("suffix", "T1w")
        .filter("session", "1")
    )
