# IBMQ Worker

A Tierkreis worker that provides convenient features to compile and run circuits on IBMQ backends using `pytket`.

## Installation

```sh
pip install tkr-ibmq-worker
```

will install an executable Python script `tkr_ibmq_worker` into your virtual environment.

## Authentication

```{important}
Accessing backends requires authentication.
To use IBMQ services, the credentials are expected to be stored in a file ` $HOME/.qiskit/qiskit-ibm.json` for more see the [Qiskit Documentation](https://quantum.cloud.ibm.com/docs/en/guides/cloud-setup)

```

Tasks that require authentication are marked as such in the task list below.

## Elementary tasks

The IBMQ worker exposes the following elementary tasks to the user:

- `get_backend_info` retrieves the backend info given a configuration dict. **Requires authentication**.
- `backend_pass_from_info` constructs a compilation pass using a backend info object.
- `backend_default_compilation_pass` fetches the default pass given the device name. **Requires authentication**.
- `compile` fetches and applies the default compilation pass. **Requires authentication**.
- `compile_circuit_ibmq` and `compile_circuits_ibmq` applies a predefined compilation pass to a (list of) circuits.
- `run_circuit` Runs the circuit on the backend. **Requires authentication**.

The full api is available in the {py:mod}`API Docs <ibmq_worker>`.

## Example

```python
class IBMQInput(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821
    n_shots: TKR[int]
    backend: TKR[str]


def compile_run_single():
    g = GraphBuilder(
        IBMQInput, TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]
    )

    compiled_circuit = g.task(
        compile(
            circuit=g.inputs.circuit,
            device_name=g.inputs.backend,
            optimisation_level=g.const(2),
        )
    )
    res = g.task(run_circuit(compiled_circuit, g.inputs.n_shots. g.inputs.backend))
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
        "backend": "<quantinuum_backend>", # e.g. H2-E
    },
)
res = read_outputs(g, storage)
print(res)
```
