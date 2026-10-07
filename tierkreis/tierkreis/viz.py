"""Rust-backed workflow visualization helpers."""

from __future__ import annotations

import json
import time
import webbrowser
from threading import Thread
from urllib.error import URLError
from urllib.request import urlopen

from tierkreis._tierkreis import serve as _serve
from tierkreis._tierkreis import serve_graph as _serve_graph
from tierkreis.builder import Graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.models import Workflow


def serve(port: int = 3000) -> None:
    """Start the Rust visualization server and block until it exits."""
    _serve(port)


def visualize_graph(graph_data: GraphData | Workflow | Graph, port: int = 3000) -> None:
    """Serve a static workflow graph and open it in the browser."""
    if isinstance(graph_data, (Workflow, Graph)):
        graph_data = graph_data.data

    server_errors: list[ValueError] = []

    def run_server() -> None:
        try:
            _serve_graph(graph_data, port)
        except ValueError as error:
            server_errors.append(error)

    server_thread = Thread(target=run_server, daemon=True)
    server_thread.start()
    base_url = f"http://127.0.0.1:{port}"

    for _ in range(300):
        if not server_thread.is_alive():
            if server_errors:
                raise RuntimeError(
                    "Failed to start Tierkreis visualization"
                ) from server_errors[0]
            raise RuntimeError("Tierkreis visualization server stopped during startup")
        try:
            with urlopen(f"{base_url}/api/workflows/", timeout=1) as response:
                workflows = json.load(response)
            break
        except (OSError, URLError):
            time.sleep(0.1)
    else:
        raise TimeoutError("Timed out waiting for the Tierkreis visualization server")

    if not workflows:
        raise RuntimeError("The Rust server did not register the graph")

    webbrowser.open(f"{base_url}/workflows/{workflows[0]['id']}/nodes/-")
    server_thread.join()
