from pathlib import Path


def add_std_handlers(workflow_logs: Path, node_logs: Path, command: str) -> str:
    """Pipe stdout and stderr to `workflow_logs` and `node_logs`.
    If the `command` returns with a non-zero exit code then touch the appropriate _error file.
    """

    _error_path = node_logs.parent / "_error"
    tee_str = f">(tee -a {str(node_logs)} {str(workflow_logs)} >/dev/null)"
    return f"({command} > {tee_str} 2> {tee_str} || touch {_error_path})"
