# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis"]
#
# [tool.uv.sources]
# tierkreis = { path = "../tierkreis", editable = true }
# ///
from pathlib import Path

from tierkreis.builder import GraphBuilder
from tierkreis.cli.run_workflow import run_workflow
from tierkreis.controller.data.models import TKR

from example_workers.hello_world_worker.stubs import greet


def hello_graph() -> GraphBuilder:
    """A graph that greets the subject."""

    # We build a graph that takes a single string as input
    # and produces a single string as output.
    g = GraphBuilder(TKR[str], TKR[str])

    # We add a constant that yields the string "hello ".
    hello = g.const("hello ")

    # We import and call the "greet" task from our worker.
    output = g.task(greet(greeting=hello, subject=g.inputs))

    # We assign the output of the greet task to the output of the whole graph.
    g.outputs(output)

    return g


def main() -> None:
    """Configure our workflow execution and run it to completion."""
    run_workflow(
        hello_graph().data,
        {"value": "world!"},
        name="hello_world",
        run_id=100,  # Assign a fixed uuid for our workflow.
        registry_path=Path(__file__).parent
        / "example_workers",  # Look for workers in the `example_workers` directory.
        use_uv_worker=True,
        polling_interval_seconds=0.1,
        print_output=True,
    )


if __name__ == "__main__":
    main()
