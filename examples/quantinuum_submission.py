from pathlib import Path
from uuid import UUID
from tierkreis import run_graph
from tierkreis.builder import GraphBuilder
from tierkreis.storage import FileStorage, read_outputs
from tierkreis.executor import UvExecutor
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.quantinuum_worker import (
    get_backend_info,
    compile_using_info,
    run_circuit,
)
from pytket.qasm.qasm import circuit_from_qasm


Circuit = OpaqueType["pytket._tket.circuit.Circuit"]
BackendResult = OpaqueType["pytket.backends.backendresult.BackendResult"]
circuit = circuit_from_qasm(Path(__file__).parent / "data" / "ghz_state_n23.qasm")


g = GraphBuilder(TKR[Circuit], TKR[BackendResult])
info = g.task(get_backend_info(g.const("H2-1")))
compiled_circuit = g.task(compile_using_info(g.inputs, info))
results = g.task(run_circuit(compiled_circuit, g.const(10), g.const("H2-1SC")))
g.outputs(results)


if __name__ == "__main__":
    storage = FileStorage(UUID(int=209), do_cleanup=True, name="quantinuum_submission")
    executor = UvExecutor(
        Path(__file__).parent.parent / "tierkreis_workers", storage.logs_path
    )
    run_graph(storage, executor, g, circuit)

    outputs = read_outputs(g, storage)
    print(outputs)
