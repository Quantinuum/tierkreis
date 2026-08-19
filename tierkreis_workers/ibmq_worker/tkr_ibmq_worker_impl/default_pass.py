from pytket.architecture import Architecture, FullyConnected
from pytket.circuit import OpType
from pytket.mapping import LexiLabellingMethod, LexiRouteRoutingMethod
from pytket.passes import (
    AutoRebase,
    AutoSquash,
    BasePass,
    CliffordSimp,
    DecomposeBoxes,
    FullMappingPass,
    FullPeepholeOptimise,
    GreedyPauliSimp,
    KAKDecomposition,
    RemoveBarriers,
    RemoveRedundancies,
    SequencePass,
    SynthesiseTket,
)
from pytket.placement import GraphPlacement

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


def default_compilation_pass(
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
    elif optimization_level == 2:
        passlist.append(FullPeepholeOptimise())
    elif optimization_level == 3:
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
                },
            ),
        )
        passlist.append(
            GreedyPauliSimp(thread_timeout=300, only_reduce=True, trials=10),
        )
    assert arch is not None
    if not isinstance(arch, FullyConnected):
        # Default placement and routing, for lightsabre use IBMQ compilation
        passlist.append(AutoRebase(primitive_gates))
        passlist.append(
            FullMappingPass(
                arch,
                GraphPlacement(arch),
                [LexiLabellingMethod(), LexiRouteRoutingMethod(10)],
            ),
        )
    if optimization_level == 1:
        passlist.append(SynthesiseTket())
    if optimization_level == 2:
        passlist.extend(
            [
                KAKDecomposition(allow_swaps=False),
                CliffordSimp(False),
                SynthesiseTket(),
            ],
        )
    if optimization_level == 3:
        passlist.append(SynthesiseTket())
    passlist.extend(
        [
            AutoRebase(primitive_gates),
            AutoSquash(primitive_1q_gates),
            RemoveRedundancies(),
        ],
    )
    return SequencePass(passlist)
