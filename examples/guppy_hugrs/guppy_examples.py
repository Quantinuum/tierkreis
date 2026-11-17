# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "guppylang==1.2.1",
#     "guppylang-internals==1.2.1",
# ]
# [tool.uv.sources]
# # Patch in https://github.com/Quantinuum/guppylang/pull/2311
# guppylang = { git = "https://github.com/Quantinuum/guppylang", subdirectory = "guppylang", rev = "57ef78030cb85686b923258ee654efc70bda2b6e" }
# guppylang-internals = { git = "https://github.com/Quantinuum/guppylang", subdirectory = "guppylang-internals", rev = "57ef78030cb85686b923258ee654efc70bda2b6e" }
# ///

from collections.abc import Callable
from pathlib import Path

from guppylang import guppy
from guppylang.std.lang import Function
from guppylang.experimental import (
    disable_experimental_features,
    enable_experimental_features,
)

def write_hugrs(func, file_prefix: str, pdfs:bool = False):
    pkg = func.with_minimal_opt().compile_function()
    with (Path(__file__).parent / f"{file_prefix}_minopt.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
    if pdfs:
        with (Path(__file__).parent / f"{file_prefix}_minopt.pdf").open("wb") as f:
            f.write(pkg.modules[0].render_dot().pipe("pdf"))

    pkg = func.compile_function()
    with (Path(__file__).parent / f"{file_prefix}.hugr").open("wb") as f:
        f.write(pkg.to_bytes())
    if pdfs:
        with (Path(__file__).parent / f"{file_prefix}.pdf").open("wb") as f:
            f.write(pkg.modules[0].render_dot().pipe("pdf"))

def doubler():
    @guppy
    def typed_doubler(x: int) -> int:
        return 2 * x

    @guppy
    def typed_doubler_plus(x: int, intercept: int) -> int:
        return x * 2 + intercept

    @guppy
    def main(a: int, b: int) -> int:
        return typed_doubler_plus(a, b) + typed_doubler(a)

    write_hugrs(main, "doubler")


def doubler_indirect():
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

    write_hugrs(main, "doubler_indirect")

def simple_arith():
    @guppy
    def simple_arith(x: int, y: int) -> int:
        prod = x * y
        diff = y - x
        return (prod * diff) + prod

    write_hugrs(simple_arith, "simple_arith")

def simple_if():
    @guppy
    def simple_if(x: int) -> int:
        if x > 10:
            return x
        else:
            return x * 2

    write_hugrs(simple_if, "simple_if")


def shortcircuit_if():
    @guppy
    def shortcircuit_if(x: int) -> int:
        if x > 10 and x < 20:
            return x
        else:
            return x * 2

    write_hugrs(shortcircuit_if, "shortcircuit_if")

def multi_if():
    @guppy
    def multi_if(x: int) -> int:
        if x > 10:
            return x
        elif x > 5:
            return x * 2
        else:
            return x * 3

    write_hugrs(multi_if, "multi_if")

def tierkreis_if():
    @guppy
    def main(x: int, y: int) -> int:
        (a, b) = (x, y) if x<y else (y, x)
        return (b - a) * (3 if 5 < x + y < 10 and x < 20 else 7)

    write_hugrs(main, "tierkreis_if")

def map():
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

    write_hugrs(invoke_map, "tierkreis_map")
        
    disable_experimental_features()


def loop():
    @guppy
    def loop_multiple_acc(acc1: int, acc2: int) -> tuple[int, int]:
        acc3 = 0
        while True:
            should_continue = 5 > acc1
            acc1 += 1
            acc2 += 2
            acc3 += 3
            # Or:
            # (acc1, acc2, acc3) = (acc1 + 1, acc2 + 2, acc3 + 3)
            if not should_continue:
                break
        return (acc2, acc3)

    write_hugrs(loop_multiple_acc, "tierkreis_loop")

def nested_loop():
    @guppy
    def nested_loop(i: int, j: int) -> int:
        while i < 10 and j != 0:
            i += j
            while j > 0:
                j -= i
            # J negative
            j = -j
        return i

    write_hugrs(nested_loop, "nested_loop")

def multi_exit_loop():
    @guppy
    def multi_exit_loop(i: int, j: int) -> int:
        while i < 10:
            if i + j > 20:
                i *= j
                break
            i += j
            j -= 1
        return i + 7

    write_hugrs(multi_exit_loop, "multi_exit_loop")

def early_return_loop():
    @guppy
    def early_return(i: int, j: int) -> int:
        while i < 10:
            if i + j > 20:
                return i * j
            i += j
            j -= 1
        return i

    write_hugrs(early_return, "early_return_loop")

if __name__ == "__main__":
    doubler()
    doubler_indirect()
    simple_if()
    shortcircuit_if()
    multi_if()
    tierkreis_if()
    map()
    loop()
    nested_loop()
    multi_exit_loop()
    early_return_loop()
    