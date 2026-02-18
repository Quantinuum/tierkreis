from enum import Enum, auto
from typing import Sequence, assert_never

from pytket._tket.circuit import Circuit
from pytket.architecture import Architecture, FullyConnected
from pytket.circuit import OpType
from pytket.mapping import LexiLabellingMethod, LexiRouteRoutingMethod
from pytket.passes import (
    AutoRebase,
    AutoSquash,
    BasePass,
    CliffordSimp,
    DecomposeBoxes,
    FullPeepholeOptimise,
    GreedyPauliSimp,
    KAKDecomposition,
    FullMappingPass,
    RemoveBarriers,
    RemoveRedundancies,
    SequencePass,
    SynthesiseTket,
)
from pytket.placement import GraphPlacement
from pytket.qasm.qasm import circuit_from_qasm_str, circuit_to_qasm_str
from tierkreis.exceptions import TierkreisError


class OptimizationLevel(Enum):
    ZERO = 0
    ONE = 1
    TWO = 2
    THREE = 3


MINIMAL_GATE_SET = {OpType.Rx, OpType.Rz, OpType.CX}
QUANTINUUM_GATE_SET = {
    OpType.ZZPhase,
    OpType.TK2,
    OpType.Rz,
    OpType.PhasedX,
    OpType.ZZMax,
}
IBMQ_GATE_SET: set[OpType] = {
    OpType.Rx,
    OpType.Rz,
    OpType.SX,
    OpType.X,
    OpType.CX,
    OpType.CZ,
    OpType.ZZPhase,
}  # Removed ECR
ALL_TKET_1Q_GATES = {
    OpType.Z,
    OpType.X,
    OpType.Y,
    OpType.S,
    OpType.Sdg,
    OpType.T,
    OpType.Tdg,
    OpType.V,
    OpType.Vdg,
    OpType.SX,
    OpType.SXdg,
    OpType.H,
    OpType.Rx,
    OpType.Ry,
    OpType.Rz,
    OpType.U1,
    OpType.U2,
    OpType.U3,
    OpType.GPI,
    OpType.GPI2,
    OpType.TK1,
    OpType.PhasedX,
}


class CircuitFormat(Enum):
    TKET = auto()
    QASM2 = auto()
    QIR = auto()


def compile_circuit(
    circuit: Circuit | str | bytes,
    input_format: CircuitFormat = CircuitFormat.TKET,
    optimization: OptimizationLevel = OptimizationLevel.TWO,
    gate_set: set[OpType] = MINIMAL_GATE_SET,
    coupling_map: Sequence[tuple[int, int]] | None = None,
    output_format: CircuitFormat = CircuitFormat.TKET,
    optimization_pass: BasePass | None = None,
) -> Circuit | str | bytes:
    # Circuit type: Circuit: TKET, str: QASM2, bytes: QIR
    if isinstance(circuit, str):
        if input_format == CircuitFormat.QASM2:
            circuit = circuit_from_qasm_str(circuit)
        else:
            raise TierkreisError("Invalid combination of input type and format.")
    if isinstance(circuit, bytes):
        if input_format == CircuitFormat.QIR:
            try:
                from pytket_qirpass import qir_to_pytket
            except ModuleNotFoundError:
                raise TierkreisError("Could not resolve pytket_qirpass")
            circuit = qir_to_pytket(circuit)
        else:
            raise TierkreisError("Invalid combination of input type and format.")

    assert isinstance(circuit, Circuit)
    qubits: set[int] = set()
    if coupling_map is not None:
        qubits = set([q for pair in coupling_map for q in pair])
        if len(qubits) < len(circuit.qubits):
            raise TierkreisError("Circuit does not fit on device.")
        arch = Architecture(coupling_map)
    else:
        arch = FullyConnected(len(qubits))

    # Provide choice of alternative optimization passes
    if optimization_pass is None:
        optimization_pass = _default_pass(optimization.value, arch, gate_set)
    optimization_pass.apply(circuit)

    match output_format:
        case CircuitFormat.TKET:
            return circuit
        case CircuitFormat.QASM2:
            return circuit_to_qasm_str(circuit)
        case CircuitFormat.QIR:
            try:
                from pytket.qir.conversion.api import pytket_to_qir
            except ModuleNotFoundError:
                raise TierkreisError("Could not resolve pytket_qirpass")
            ret = pytket_to_qir(circuit)
            if ret is None:
                raise TierkreisError("Could not transform circuit to QIR.")
            return ret
        case _:
            assert_never()


def _default_pass(
    optimization_level: int,
    arch: Architecture | FullyConnected,
    primitive_gates: set[OpType],
) -> BasePass:
    ## Modified from https://github.com/Quantinuum/pytket-qiskit/blob/main/pytket/extensions/qiskit/backends/ibm.py
    primitive_1q_gates = primitive_gates & ALL_TKET_1Q_GATES
    supports_rz = OpType.Rz in primitive_gates

    assert optimization_level in range(4)
    passlist = [DecomposeBoxes()]
    if optimization_level == 0:
        if supports_rz:
            passlist.append(AutoRebase(primitive_gates))
    elif optimization_level == 1:
        passlist.append(SynthesiseTket())
        passlist.append(AutoSquash(primitive_1q_gates))
    elif optimization_level == 2:  # noqa: PLR2004
        passlist.append(FullPeepholeOptimise())
    elif optimization_level == 3:  # noqa: PLR2004
        passlist.append(RemoveBarriers())
        passlist.append(
            AutoRebase(
                {
                    OpType.Z,
                    OpType.X,
                    OpType.Y,
                    OpType.S,
                    OpType.Sdg,
                    OpType.V,
                    OpType.Vdg,
                    OpType.H,
                    OpType.CX,
                    OpType.CY,
                    OpType.CZ,
                    OpType.SWAP,
                    OpType.Rz,
                    OpType.Rx,
                    OpType.Ry,
                    OpType.T,
                    OpType.Tdg,
                    OpType.ZZMax,
                    OpType.ZZPhase,
                    OpType.XXPhase,
                    OpType.YYPhase,
                    OpType.PhasedX,
                }
            ),
        )
        passlist.append(
            GreedyPauliSimp(thread_timeout=300, only_reduce=True, trials=10)
        )
    assert arch is not None
    if not isinstance(arch, FullyConnected):
        # Default placement and routing, for lightsabre use IBMQ compilation
        # TODO check for better passes here
        passlist.append(AutoRebase(primitive_gates))
        passlist.append(
            FullMappingPass(
                arch,
                GraphPlacement(arch),
                [LexiLabellingMethod(), LexiRouteRoutingMethod(10)],
            )
        )
    if optimization_level == 1:
        passlist.append(SynthesiseTket())
    if optimization_level == 2:  # noqa: PLR2004
        passlist.extend(
            [
                KAKDecomposition(allow_swaps=False),
                CliffordSimp(False),
                SynthesiseTket(),
            ]
        )
    if optimization_level == 3:  # noqa: PLR2004
        passlist.append(SynthesiseTket())
    passlist.extend(
        [
            AutoRebase(primitive_gates),
            AutoSquash(primitive_1q_gates),
            RemoveRedundancies(),
        ]
    )
    return SequencePass(passlist)
