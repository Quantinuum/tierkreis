# Pytket worker

A Tierkreis worker that interacts with the Quantinuums `guppylang` language infrastructure.

The GUPPY worker wraps some functionality from [Guppy](https://guppylang.org).

## Installation


```sh
pip install tkr-guppy-worker
```

will install the `guppy_worker` api into your virtual environment.

```sh
pip install tkr-guppy-worker-impl
```

will install the `tkr-guppy-worker` script into your virtual environment.


## Elementary tasks

The Guppy worker exposes the following elementary tasks to the user:

- `emulate`. Emulate a Hugr locally.
- `from_circuit`. Generate a Hugr from a `pyktet.Circuit`
- `to_backend_result`. Transform a new result into a `pykter.BackendResult`
- `ghz`. Construct an size n GHZ state.

The full api is available in the {py:mod}`API Docs <guppy_worker>`.
