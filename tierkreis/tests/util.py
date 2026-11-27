import asyncio
from contextlib import chdir
from pathlib import Path

from tests.controller.typed_graphdata import typed_eval, typed_map
from tierkreis.runtime import load_runtime_config, new_from_config


async def main() -> None:
    runtime = await new_from_config(load_runtime_config("tierkreis.toml"))
    with runtime:
        workflow_id = await runtime.save_workflow("simple_eval", typed_eval())
        _ = await runtime.save_workflow(
            "simple_map", typed_map()
        )  # workflow without runs

        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, timeout=30)
        output = await runtime.get_outputs(run_id)
        assert isinstance(output, dict)
        if output["typed_eval_output"] != 12:
            raise RuntimeError(f"Unexpected output for attempt 0: {output!r}")

        attempt, _ = await runtime.restart_task(run_id, ["N0"])
        await runtime.wait_for(run_id, attempt, timeout=30)
        output = await runtime.get_outputs(run_id, attempt)
        assert isinstance(output, dict)
        if attempt != 1 or output["typed_eval_output"] != 12:
            raise RuntimeError(f"Unexpected attempt/output: {attempt}/{output!r}")


if __name__ == "__main__":
    with chdir(Path(__file__).parent.parent.parent):
        asyncio.run(main())
