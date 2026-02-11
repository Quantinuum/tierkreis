# Pytket worker

A Tierkreis worker that interacts with the Quantinuums `pytket` toolkit.

The pytket worker largely wraps the functionality from [tket](https://github.com/CQCL/tket).

## Installation

```sh
pip install tkr-pytket-worker
```

will install an executable Python script `tkr_pytket_worker` into your virtual environment.

## Authentication

This worker is designed to work without authentication for the majority of the tasks.

```{important}
The `get_backend_info` function is the notable exception and requires authentication with Quantinuum Nexus to access the devices.
The authentication depends on IBMQ and Quantinuum web services respectively.
```

## Elementary tasks

The pytket worker exposes the following elementary tasks to the user:

- `get_backend_info` retrieves the backend info given a configuration dict. **Requires authentication**. Requires optional dependencies `uv sync --groups backends`.
- `device_name_from_info` retrieves the device name from backend info object.
- `compile_using_info` compiles a circuit with a default pass for a backend info that was previously acquired.
- `add_measure_all` adds final measurements to a circuit.
- `append_pauli_measurement_impl` adds measurements according to a Pauli string to a circuit.
- `optimise_phase_gadgets` applies the phase gadget optimization pass to a circuit.
- `apply_pass` applies a user defined optimization pass.
- `compile_generic_with_fixed_pass` generic compile function with a variety of options using a predefined pass without considering fidelities. No authentication is required.
- `to_qasm_str` and `from_qasm_str` transforms a Circuit to/from QASM 2.
- `to_qir_bytes` and `from_qir_bytes` transforms a Circuit to/from QIR.
- `expectation` estimates the expectation value from shot counts.
- `n_qubits` returns the number of qubits in a const circuit.
- `backend_result_to_dict` and `backend_result_from_dict` convert between a `BackendResult` and a register based dictionary mapping register names to a list of shot bitstrings.

The straight forward approach to compiling a circuit with the default pass for a backend is:

1. Construct a `qnx.BackendConfig` object defining the desired backend
2. Get the according `pytket.BackendInfo` object
3. Provide both together with a circuit to immediately compile the circuit for the backend.

```{warning}
While there is an apply pass function, this will only work with serializable passes.
Trying to use a non-serializable pass will result in an error.
```

## Example

An example use is in `docs/source/examples/hamiltonian.ipynb` in the [Tierkreis repo](https://github.com/CQCL/tierkreis).
A shorter example which runs on a named IBM backend, e.g. `"ibm_torino"` is given here
For debugging purposes this also showcases how to use the `InMemoryExecutor` which only works in conjunction with the `InMemoryStorage`.

```python
class IBMInput(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821
    n_shots: TKR[int]
    backend: TKR[str]


def compile_run_single():
    g = GraphBuilder(
        IBMInput, TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]
    )

    compiled_circuit = g.task(
        compile_circuit_ibmq(
            circuit=g.inputs.circuit,
            device_name=g.inputs.backend,
            optimisation_level=g.const(2),
        )
    )
    res = g.task(submit_single(compiled_circuit, g.inputs.n_shots))
    g.outputs(res)
    return g

circuit = ...your circuit here...

storage = InMemoryStorage(UUID(int=109))
executor = InMemoryExecutor(
    Path(__file__).parent.parent / "tierkreis_workers", storage
)
n_shots = 30
run_graph(
    storage,
    executor,
    g,
    {
        "circuit": circuit,
        "n_shots": n_shots,
        "backend": "<ibm_backend>", # e.g. ibm_torino
    },
)
res = read_outputs(g, storage)
print(res)
```
