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

### Quantum Operations
- `emulate`. Emulate a Hugr locally.
- `ghz`. Construct an size n GHZ state.

### Circuit Conversion
- `from_circuit`. Build a Hugr from a `pytket.Circuit`
- `to_backend_result`. Transform a Hugr emulator result to a `pytket.BackendResult`

### Package Serialization
- `from_str`. Build a Hugr from a string representation.
- `to_str`. Convert a Hugr to a string representation.
- `from_bytes`. Build a Hugr from a bytes representation.
- `to_bytes`. Convert a Hugr to a bytes representation.

### Optimization Passes
- `normalize`. Normalize a Hugr package.
- `apply_pytket_pass`. Apply a pytket pass to a Hugr package.
- `remove_redundancies`. Remove redundant operations from a Hugr package.
- `squash_rz_phased_x`. Squash Rz and PhasedX gates in a Hugr package.

### Code Generation
- `to_qir`. Convert a Hugr package to a QIR representation.

The full api is available in the {py:mod}`API Docs <guppy_worker>`.
