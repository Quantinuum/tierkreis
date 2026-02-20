from pytket.circuit import OpType
from pytket.passes import (
    AutoRebase,
    AutoSquash,
    BasePass,
    DecomposeBoxes,
    DecomposeTK2,
    FlattenRelabelRegistersPass,
    GreedyPauliSimp,
    NormaliseTK2,
    RemoveBarriers,
    RemovePhaseOps,
    RemoveRedundancies,
    SequencePass,
)
from pytket.passes.resizeregpass import scratch_reg_resize_pass


def _gate_set() -> set[OpType]:
    return {OpType.ZZPhase, OpType.TK2, OpType.Rz, OpType.PhasedX, OpType.ZZMax}


def two_qubit_gate_set() -> set[OpType]:
    """Returns the set of supported two-qubit gates.

    Submitted circuits must contain only one of these.
    """
    return _gate_set() & {OpType.ZZPhase, OpType.ZZMax, OpType.TK2}


def default_compilation_pass() -> BasePass:
    passlist = [
        DecomposeBoxes(),
        scratch_reg_resize_pass(),
    ]
    squash = AutoSquash({OpType.PhasedX, OpType.Rz})
    decomposition_passes = [
        NormaliseTK2(),
        DecomposeTK2(allow_swaps=True, ZZPhase_fidelity=1.0),
    ]

    passlist.extend(
        [
            RemoveBarriers(),
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
            GreedyPauliSimp(
                allow_zzphase=True,
                only_reduce=True,
                thread_timeout=300,
                trials=10,
            ),
        ],
    )
    passlist.extend(decomposition_passes)
    rebase_pass = AutoRebase(
        (_gate_set() - two_qubit_gate_set()) | {OpType.ZZPhase},
        allow_swaps=True,
    )
    passlist.extend(
        [
            rebase_pass,
            RemoveRedundancies(),
            squash,
            RemoveRedundancies(),
        ],
    )
    passlist.append(RemovePhaseOps())
    passlist.append(FlattenRelabelRegistersPass("q"))
    return SequencePass(passlist, strict=False)
