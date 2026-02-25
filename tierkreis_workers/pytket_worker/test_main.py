from pytket._tket.circuit import Circuit
from pytket._tket.unit_id import Bit
from pytket.backends.backendresult import BackendResult
from pytket.extensions.qiskit.backends.aer import AerBackend
from pytket.utils.outcomearray import OutcomeArray

from .main import backend_result_from_dict, backend_result_to_dict


def test_backend_result_conversion() -> None:
    original_result = BackendResult(
        shots=OutcomeArray.from_readouts([[0, 1, 0], [1, 0, 1], [1, 1, 1]]),
        c_bits=[Bit("a", 0), Bit("b", 0), Bit("b", 1)],
    )
    result_dict = backend_result_to_dict(original_result)
    reconstructed_result = backend_result_from_dict(result_dict)
    assert (
        original_result.get_shots().tolist()
        == reconstructed_result.get_shots().tolist()
    )
    assert original_result.c_bits == reconstructed_result.c_bits
    assert result_dict == {
        "a": ["0", "1", "1"],
        "b": ["10", "01", "11"],
    }


def _deterministic_circuit() -> Circuit:
    circuit = Circuit(3)
    circuit.add_c_register("a", 1)
    circuit.add_c_register("b", 2)
    circuit.X(0)
    circuit.X(1)
    circuit.Measure(0, Bit("a", 0))
    circuit.Measure(1, Bit("b", 0))
    circuit.Measure(2, Bit("b", 1))
    return circuit


def test_deterministic_circuit() -> None:
    circuit = _deterministic_circuit()
    result = AerBackend().run_circuit(circuit, 10)
    result_dict = backend_result_to_dict(result)
    assert result_dict == {"a": ["1"] * 10, "b": ["10"] * 10}
    assert result == backend_result_from_dict(result_dict)
