"""Bash library generated from a TypeSpec worker interface.

The library is sourced by a shell worker which defines one `tkr_<function>`
bash function per task and calls `tkr_main "$@"`. It reads the `TKR_*` port
environment of the subprocess and HPC executors and handles the decorators:

- `@file("name")` inputs are linked into a fresh working directory under
  `name`; `@file("name")` outputs are copied from it when requested.
- `@args` on an `Array<string>` input decodes the JSON array into the bash
  array `TKR_ARGS_<port>`, e.g. to pass it on as `"${TKR_ARGS_flags[@]}"`.

`tkr_<function>` runs inside the working directory, available as
`TKR_WORKDIR`. Set `TKR_WORKDIR_ROOT` to place working directories, e.g. on
shared storage for multi-node MPI programs. The working directory is removed
on success and kept for inspection on failure.
"""

import re
from collections.abc import Iterable

from tierkreis.exceptions import TierkreisError
from tierkreis.idl.models import GenericType, Method, Model, TypedArg

_FILE_NAME = re.compile(r"[A-Za-z0-9_.+-]+")


def port_variable(prefix: str, port: str) -> str:
    """Name of the environment variable for a port, as exported by the executors.

    :param prefix: `TKR_INPUT_` or `TKR_OUTPUT_`.
    :param port: The port name.
    :return: The variable name.
    """
    return prefix + re.sub(r"[^A-Za-z0-9_]", "_", port)


def output_model(method: Method, models: Iterable[Model]) -> Model | None:
    """The portmapping model returned by a method, if any."""
    if not method.return_type_is_portmapping:
        return None
    return next((model for model in models if model.t == method.return_type), None)


def validate_method(method: Method, models: Iterable[Model]) -> None:
    """Check the shell decorators of a method.

    :raises TierkreisError: If a decorator is used on an unsupported port.
    """
    outputs = output_model(method, models)
    ports = method.args + (outputs.decls if outputs is not None else [])
    for port in method.args:
        if port.args and not _is_string_array(port):
            msg = f"@args on {port.name} requires type Array<string>"
            raise TierkreisError(msg)
    if outputs is not None and (
        port := next((d for d in outputs.decls if d.args), None)
    ):
        msg = f"@args on output {port.name} is not supported"
        raise TierkreisError(msg)
    files: set[str] = set()
    for port in ports:
        if port.file is None:
            continue
        if port.t.origin is not bytes:
            msg = f"@file on {port.name} requires type bytes"
            raise TierkreisError(msg)
        if port.file in {".", ".."} or not _FILE_NAME.fullmatch(port.file):
            msg = f"@file name {port.file!r} on {port.name} must be a plain file name"
            raise TierkreisError(msg)
        if port.file in files:
            msg = f"@file name {port.file!r} is used twice in {method.name.origin}"
            raise TierkreisError(msg)
        files.add(port.file)


def _is_string_array(port: TypedArg) -> bool:
    return (
        port.t.origin is list
        and len(port.t.args) == 1
        and isinstance(element := port.t.args[0], GenericType)
        and element.origin is str
    )


def validate_models(methods: Iterable[Method], models: Iterable[Model]) -> None:
    """Reject decorated fields on models that are not returned as portmappings.

    :raises TierkreisError: If a decorator cannot take effect.
    """
    returned = {output_model(method, models) for method in methods}
    for model in models:
        if model in returned:
            continue
        for decl in model.decls:
            if decl.args:
                msg = f"@args on {model.t.origin}.{decl.name} is not supported"
                raise TierkreisError(msg)
            if decl.file is not None:
                msg = (
                    f"@file on {model.t.origin}.{decl.name} requires a "
                    "@portmapping model returned by a function"
                )
                raise TierkreisError(msg)


def _quote(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


def _stage_input(port: TypedArg) -> str:
    variable = port_variable("TKR_INPUT_", port.name)
    target = _quote(port.file or "")
    return f'    ln -s "${{{variable}:?{variable} is not set}}" "${{TKR_WORKDIR}}"/{target}'


def _decode_args(port: TypedArg) -> str:
    source = port_variable("TKR_INPUT_", port.name)
    target = port_variable("TKR_ARGS_", port.name)
    return (
        f"    declare -ga {target}=()\n"
        f'    _tkr_json_strings "${{{source}:?{source} is not set}}" {target}'
    )


def _collect_output(port: TypedArg) -> str:
    variable = port_variable("TKR_OUTPUT_", port.name)
    source = _quote(port.file or "")
    return (
        f"    if [[ -n ${{{variable}:-}} ]]; then\n"
        f'        _tkr_collect "${{TKR_WORKDIR}}"/{source} "${{{variable}}}"\n'
        "    fi"
    )


def _function(method: Method, models: Iterable[Model]) -> str:
    name = str(method.name.origin)
    outputs = output_model(method, models)
    output_ports = outputs.decls if outputs is not None else []
    lines = [
        f"_tkr_run_{name}() {{",
        f"    _tkr_require_function tkr_{name}",
        f"    _tkr_workdir {name}",
        *(_stage_input(port) for port in method.args if port.file is not None),
        *(_decode_args(port) for port in method.args if port.args),
        f'    (cd "${{TKR_WORKDIR}}"; tkr_{name})',
        *(_collect_output(port) for port in output_ports if port.file is not None),
        '    rm -rf "${TKR_WORKDIR}"',
        "}",
    ]
    return "\n".join(lines)


_PRELUDE = r"""_tkr_fail() {
    printf '%s\n' "$*" >&2
    exit 1
}

_tkr_require_function() {
    declare -F "$1" >/dev/null || _tkr_fail "Shell worker does not define $1"
}

_tkr_workdir() {
    TKR_WORKDIR=$(mktemp -d "${TKR_WORKDIR_ROOT:-${TMPDIR:-/tmp}}/tkr-$1.XXXXXX")
    export TKR_WORKDIR
    trap '[[ $? -eq 0 ]] || printf "Working directory kept at %s\n" "${TKR_WORKDIR}" >&2' EXIT
}

_tkr_json_space() {
    while ((_tkr_i < ${#_tkr_json})) && [[ ${_tkr_json:_tkr_i:1} == [[:space:]] ]]; do
        _tkr_i=$((_tkr_i + 1))
    done
}

_tkr_json_hex() {
    local hex=${_tkr_json:_tkr_i:4}
    [[ $hex =~ ^[0-9A-Fa-f]{4}$ ]] || _tkr_fail "Invalid \u escape in $1"
    _tkr_i=$((_tkr_i + 4))
    _tkr_code=$((16#$hex))
}

# Decode the JSON array of strings in file $1 into the array named $2.
_tkr_json_strings() {
    local -n _tkr_array=$2
    local _tkr_json _tkr_i=0 _tkr_char _tkr_value _tkr_code _tkr_high
    _tkr_json=$(<"$1")
    _tkr_array=()
    _tkr_json_space
    [[ ${_tkr_json:_tkr_i:1} == "[" ]] || _tkr_fail "Expected a JSON array in $1"
    _tkr_i=$((_tkr_i + 1))
    _tkr_json_space
    if [[ ${_tkr_json:_tkr_i:1} == "]" ]]; then
        _tkr_i=$((_tkr_i + 1))
    else
        while true; do
            _tkr_json_space
            [[ ${_tkr_json:_tkr_i:1} == '"' ]] || _tkr_fail "Expected a JSON string in $1"
            _tkr_i=$((_tkr_i + 1))
            _tkr_value=
            while true; do
                ((_tkr_i < ${#_tkr_json})) || _tkr_fail "Unterminated JSON string in $1"
                _tkr_char=${_tkr_json:_tkr_i:1}
                _tkr_i=$((_tkr_i + 1))
                [[ $_tkr_char == '"' ]] && break
                if [[ $_tkr_char == '\' ]]; then
                    _tkr_char=${_tkr_json:_tkr_i:1}
                    _tkr_i=$((_tkr_i + 1))
                    case $_tkr_char in
                    '"' | '\' | /) ;;
                    b) _tkr_char=$'\b' ;;
                    f) _tkr_char=$'\f' ;;
                    n) _tkr_char=$'\n' ;;
                    r) _tkr_char=$'\r' ;;
                    t) _tkr_char=$'\t' ;;
                    u)
                        _tkr_json_hex "$1"
                        if ((_tkr_code >= 0xD800 && _tkr_code < 0xDC00)); then
                            _tkr_high=$_tkr_code
                            [[ ${_tkr_json:_tkr_i:2} == '\u' ]] || _tkr_fail "Unpaired surrogate in $1"
                            _tkr_i=$((_tkr_i + 2))
                            _tkr_json_hex "$1"
                            ((_tkr_code >= 0xDC00 && _tkr_code < 0xE000)) || _tkr_fail "Unpaired surrogate in $1"
                            _tkr_code=$((0x10000 + ((_tkr_high - 0xD800) << 10) + _tkr_code - 0xDC00))
                        fi
                        ((_tkr_code != 0)) || _tkr_fail "NUL characters are not supported in $1"
                        printf -v _tkr_char '\\U%08X' "$_tkr_code"
                        printf -v _tkr_char "$_tkr_char"
                        ;;
                    *) _tkr_fail "Invalid JSON escape in $1" ;;
                    esac
                fi
                _tkr_value+=$_tkr_char
            done
            _tkr_array+=("$_tkr_value")
            _tkr_json_space
            _tkr_char=${_tkr_json:_tkr_i:1}
            _tkr_i=$((_tkr_i + 1))
            [[ $_tkr_char == "]" ]] && break
            [[ $_tkr_char == "," ]] || _tkr_fail "Expected , or ] in $1"
        done
    fi
    _tkr_json_space
    ((_tkr_i == ${#_tkr_json})) || _tkr_fail "Unexpected data after the JSON array in $1"
}

_tkr_collect() {
    [[ -f $1 ]] || _tkr_fail "Expected output file $1 was not written"
    cp "$1" "$2"
}
"""


def shell_library(name: str, methods: list[Method], models: Iterable[Model]) -> str:
    """Generate the bash library for a worker namespace.

    :param name: The namespace name.
    :param methods: The methods of the namespace.
    :param models: The models of the namespace.
    :return: The bash source.
    """
    models = list(models)
    cases = "\n".join(
        f"    {method.name.origin}) _tkr_run_{method.name.origin} ;;"
        for method in methods
    )
    functions = "\n\n".join(_function(method, models) for method in methods)
    return f"""\
# shellcheck shell=bash
# Code generated from {name} namespace. Please do not edit.
#
# Source this file, define `tkr_<function>` for each task and call
# `tkr_main "$@"`. Requires bash >= 4.4 and the TKR_* port environment set by
# the subprocess and HPC executors.

{_PRELUDE}
{functions}

tkr_main() {{
    set -euo pipefail
    case "${{TKR_FUNCTION_NAME:?TKR_FUNCTION_NAME is not set}}" in
{cases}
    *) _tkr_fail "Unknown function ${{TKR_FUNCTION_NAME}} in {name}" ;;
    esac
}}
"""
