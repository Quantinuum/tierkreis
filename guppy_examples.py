from collections.abc import Callable
from pathlib import Path

from guppylang import guppy
from guppylang.std.lang import Function
from guppylang.experimental import (
    disable_experimental_features,
    enable_experimental_features,
)



def test_doubler(validate):
    @guppy
    def typed_doubler(x: int) -> int:
        return 2 * x

    @guppy
    def typed_doubler_plus(x: int, intercept: int) -> int:
        return x * 2 + intercept

    @guppy
    def main(a: int, b: int) -> int:
        return typed_doubler_plus(a, b) + typed_doubler(a)

    pkg = main.with_minimal_opt().compile_function()
    validate(pkg.modules[0])
    with Path("doubler_minopt.hugr").open("wb") as f:
        f.write(pkg.to_bytes())

    pkg = main.compile_function()
    validate(pkg.modules[0])
    with Path("doubler.hugr").open("wb") as f:
        f.write(pkg.to_bytes())


def test_doubler_indirect(validate):
    @guppy.struct
    class DoublerOutput:
        a: int
        value: int

    @guppy
    def typed_doubler_plus_multi(x: int, intercept: int) -> DoublerOutput:
        return DoublerOutput(x, x * 2 + intercept)

    @guppy
    def indirect_call(f: Callable[[int, int], DoublerOutput], a: int, b: int) -> int:
        d = f(a, b)
        return d.a + d.value

    @guppy
    def main(a: int, b: int) -> int:
        return indirect_call(typed_doubler_plus_multi, a, b)

    pkg = main.with_minimal_opt().compile_function()
    validate(pkg.modules[0])
    with Path("doubler_indirect_minopt.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
    
    pkg = main.compile_function()
    with Path("doubler_indirect.hugr").open("wb") as f:
        f.write(pkg.to_bytes())


def test_map(validate):
    enable_experimental_features()
    @guppy
    def map_doubler(x: int) -> int:
        return x * 2

    @guppy
    def apply_map(f: Function[[int], int]) -> list[int]:
        ys = [f(x) for x in [1, 2, 3]]
        return ys

    @guppy
    def invoke_map() -> list[int]:
        return apply_map(map_doubler)

    pkg = invoke_map.with_minimal_opt().compile_function()
    validate(pkg.modules[0])
    with Path("tierkreis_map_minopt.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
    
    pkg = invoke_map.compile_function()
    validate(pkg.modules[0])
    with Path("tierkreis_map.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
        
    disable_experimental_features()


def test_loop(validate):
    @guppy.struct
    class LoopMultipleAccOut:
        acc1: int
        acc2: int
        acc3: int

    @guppy
    def loop_multiple_acc() -> LoopMultipleAccOut:
        (acc1, acc2, acc3) = (0, 0, 0)
        while True:
            should_continue = 5 > acc1
            acc1 += 1
            acc2 += 2
            acc3 += 3
            # Or:
            # (acc1, acc2, acc3) = (acc1 + 1, acc2 + 2, acc3 + 3)
            if not should_continue:
                break
        return LoopMultipleAccOut(acc1, acc2, acc3)

    pkg = loop_multiple_acc.with_minimal_opt().compile_function()
    validate(pkg.modules[0])
    with Path("tierkreis_loop_minopt.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
    
    pkg = loop_multiple_acc.compile_function()
    validate(pkg.modules[0])
    with Path("tierkreis_loop.hugr").open("wb") as f:
        f.write(pkg.to_bytes())

if __name__ == "__main__":
    def validate(module):
        pass
    test_doubler(validate)
    test_doubler_indirect(validate)
    test_map(validate)
    test_loop(validate)