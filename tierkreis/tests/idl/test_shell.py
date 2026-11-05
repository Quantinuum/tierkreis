import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

from tierkreis.exceptions import TierkreisError
from tierkreis.idl.spec import spec
from tierkreis.namespace import Namespace

SPEC = Path(__file__).parent / "shell_worker.tsp"
WORKER = """\
source "$1"
tkr_diagonalize() {
    [[ $PWD == "$TKR_WORKDIR" ]]
    cat fcidump.txt fcidump.txt >carryover.bin
    cat "$TKR_INPUT_block" >"$TKR_OUTPUT_energy"
    [[ ${#TKR_ARGS_flags[@]} -eq 2 && ${TKR_ARGS_flags[1]} == "b c" ]]
    [[ -z ${FAIL:-} ]]
}
tkr_main
"""


def _bash() -> str:
    bash = shutil.which("bash")
    if bash is None:
        pytest.skip("bash is not available")
    version = subprocess.run(
        [bash, "-c", "echo $((BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1]))"],
        capture_output=True,
        text=True,
        check=True,
    )
    if int(version.stdout) < 404:
        pytest.skip("bash >= 4.4 is required")
    return bash


def test_file_decorator() -> None:
    namespace = Namespace.from_spec_file(SPEC)
    args = namespace.methods[0].args
    assert [arg.file for arg in args] == ["fcidump.txt", None, None]
    assert [arg.args for arg in args] == [False, False, True]
    result = next(m for m in namespace.models if m.t.origin == "Result")
    assert [decl.file for decl in result.decls] == ["carryover.bin", None]


@pytest.mark.parametrize(
    "source",
    [
        'interface w { f(@file("a") x: string): bytes; }',
        'interface w { f(@file("a") x: bytes, @file("a") y: bytes): bytes; }',
        'interface w { f(@file("../a") x: bytes): bytes; }',
        'interface w { f(@file("a") @file("b") x: bytes): bytes; }',
        'model M { @file("a") x: bytes; } interface w { f(): bytes; }',
        "interface w { f(@args x: bytes): bytes; }",
        "interface w { f(@args x: Array<integer>): bytes; }",
        "interface w { f(@args @args x: Array<string>): bytes; }",
        "@portmapping model M { @args x: Array<string>; } interface w { f(): M; }",
        "model M { @args x: Array<string>; } interface w { f(): bytes; }",
    ],
)
def test_decorator_rejected(source: str) -> None:
    with pytest.raises(TierkreisError):
        Namespace._from_spec(spec(source)[0])


def _run(
    tmp_path: Path, outputs: list[str], *, fail: bool = False
) -> tuple[subprocess.CompletedProcess[str], dict[str, Path]]:
    library = tmp_path / "api.sh"
    Namespace.from_spec_file(SPEC).write_shell_stubs(library)
    (tmp_path / "fcidump").write_text("ab")
    (tmp_path / "block").write_text("10")
    (tmp_path / "flags").write_text(json.dumps(["a", "b c"]))
    paths = {name: tmp_path / f"out_{name}" for name in outputs}
    workdirs = tmp_path / "work"
    workdirs.mkdir()
    env = {
        **os.environ,
        "TKR_WORKDIR_ROOT": str(workdirs),
        "TKR_FUNCTION_NAME": "diagonalize",
        "TKR_INPUT_fcidump": str(tmp_path / "fcidump"),
        "TKR_INPUT_block": str(tmp_path / "block"),
        "TKR_INPUT_flags": str(tmp_path / "flags"),
        **{f"TKR_OUTPUT_{name}": str(path) for name, path in paths.items()},
    }
    if fail:
        env["FAIL"] = "1"
    process = subprocess.run(
        [_bash(), "-c", WORKER, "worker", str(library)],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    return process, paths


def test_shell_library_stages_files(tmp_path: Path) -> None:
    process, paths = _run(tmp_path, ["carryover", "energy"])
    assert process.returncode == 0, process.stderr
    assert paths["carryover"].read_text() == "abab"
    assert paths["energy"].read_text() == "10"
    assert not any((tmp_path / "work").iterdir())


def test_shell_library_skips_unrequested_outputs(tmp_path: Path) -> None:
    process, paths = _run(tmp_path, ["energy"])
    assert process.returncode == 0, process.stderr
    assert paths["energy"].read_text() == "10"


def test_shell_library_keeps_workdir_on_failure(tmp_path: Path) -> None:
    process, _ = _run(tmp_path, ["carryover"], fail=True)
    assert process.returncode != 0
    assert "Working directory kept" in process.stderr
    assert any((tmp_path / "work").iterdir())


def _utf8_locale() -> str | None:
    locales = subprocess.run(
        ["locale", "-a"], capture_output=True, text=True, check=False
    ).stdout.split()
    preferred = ["C.UTF-8", "C.utf8", "en_US.UTF-8", "en_US.utf8"]
    return next((name for name in preferred if name in locales), None)


@pytest.mark.parametrize(
    "values",
    [
        [],
        ["--block", "10"],
        ["", "with space", "  padded  "],
        ['quote " and \\ backslash / slash', "tab\tnew\nline\r", "\b\f"],
        ["$HOME", "*", "'single'", "-n", "-e"],
        ["gr\u00fc\u00dfe", "\U0001f600"],
    ],
)
@pytest.mark.parametrize("ensure_ascii", [True, False])
def test_shell_library_decodes_args(
    tmp_path: Path, values: list[str], *, ensure_ascii: bool
) -> None:
    env = dict(os.environ)
    if any(not value.isascii() for value in values):
        locale = _utf8_locale()
        if locale is None:
            pytest.skip("A UTF-8 locale is required")
        env["LC_ALL"] = locale
    library = tmp_path / "api.sh"
    Namespace.from_spec_file(SPEC).write_shell_stubs(library)
    source = tmp_path / "flags"
    source.write_text(json.dumps(values, indent=2, ensure_ascii=ensure_ascii))
    script = """\
source "$1"
set -euo pipefail
_tkr_json_strings "$2" decoded
for value in ${decoded[@]+"${decoded[@]}"}; do printf '%s\\0' "$value"; done
"""
    process = subprocess.run(
        [_bash(), "-c", script, "decode", str(library), str(source)],
        env=env,
        capture_output=True,
        check=False,
    )
    assert process.returncode == 0, process.stderr
    decoded = process.stdout.decode().split("\0")[:-1]
    assert decoded == values


@pytest.mark.parametrize(
    "source",
    ['{"a": 1}', '["a"', '["a" "b"]', "[1]", '["\\x"]', '["\\ud83d"]', '["a"] x'],
)
def test_shell_library_rejects_invalid_args(tmp_path: Path, source: str) -> None:
    library = tmp_path / "api.sh"
    Namespace.from_spec_file(SPEC).write_shell_stubs(library)
    (tmp_path / "flags").write_text(source)
    script = 'source "$1"; set -euo pipefail; _tkr_json_strings "$2" decoded'
    process = subprocess.run(
        [_bash(), "-c", script, "decode", str(library), str(tmp_path / "flags")],
        capture_output=True,
        text=True,
        check=False,
    )
    assert process.returncode != 0
    assert process.stderr
