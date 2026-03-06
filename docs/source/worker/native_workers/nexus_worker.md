# Nexus worker

A Tierkreis worker that interacts with the Quantinuum Nexus API.

The Nexus worker largely wraps the functionality from [the qnexus library](https://pypi.org/project/qnexus/).
In addition to the elementary tasks exposed, there are also prepackaged graphs to make using the Nexus worker more convenient.

## Installation

```sh
pip install tkr-nexus-worker
```

will install an executable Python script `tkr_nexus_worker` into your virtual environment.

## Authentication

The worker uses the default mechanism provided by the `qnexus` Python package.

```bash
uv run python -c "from qnexus.client.auth import login; login()"
```

will put the a token in the appropriate filesystem location for subsequent operations to use.

## Elementary tasks

The Nexus worker exposes the following elementary tasks to the user.

- `upload_circuit`. A wrapper around `qnexus.circuits.upload`, which is intended to be parallelised using a Tierkreis `map`.
- `start_execute_job`. A wrapper around `qnuexs.jobs.start_execute_job` that takes a `BackendConfig` to specify what hardware the circuits should be run on.
- `is_running`. A wrapper around `qnexus.jobs.status` that will fail if an unsuccessful terminal state is received, return `False` if the job is successfully completed and return `True` otherwise.

The full api is available in the {py:mod}`API Docs <nexus_worker>`.

## Prepackaged graphs

The Tierkreis Python package provides a couple of prepackaged graphs to make it easier to interact with the Nexus API.

`tierkreis.graphs.nexus.submit_poll.nexus_submit_and_poll` is intended to automate the whole process of (parallelised) circuit upload, submission, status polling and result retrieval.
It can be included within a custom graph using `GraphBuilder.eval` or run as a standalone graph.
The function `nexus_submit_and_poll` takes an optional argument to specify the minimum delay between successive polls in the status polling loop.
The default is to poll every `30` seconds.

An example use is in `docs/source/examples/polling_and_dir.ipynb` in the [Tierkreis repo](https://github.com/Quantinuum/tierkreis), which looks like:

```python
backend_config = ...select the right hardware by specifying the BackendConfig instance...
circuits = ...your circuits here...

storage = FileStorage(UUID(int=107), do_cleanup=True)
executor = UvExecutor(PACKAGE_PATH / ".." / "tierkreis_workers", storage.logs_path)

run_graph(
    storage,
    executor,
    nexus_submit_and_poll(),
    {
        "project_name": "2025-tkr-test",
        "job_name": "job-1",
        "circuits": circuits,
        "n_shots": [30] * len(circuits),
        "backend_config": backend_config,
    },
    polling_interval_seconds=1,
)
res = read_outputs(g, storage)
print(res)
```

The subgraphs `tierkreis.graphs.nexus.submit_poll.upload_circuit_graph` and `tierkreis.graphs.nexus.submit_poll.polling_loop_body` can also be used if the user wants to customise the main submit and poll graph.
