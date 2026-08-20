# Tierkreis

Quantum-classical hybrid workflow orchestration tool.
This is the top level repository containing the python packages.

## Introduction

Tierkreis allows users to define hybrid workflows as computational graphs
```python
class InParams(NamedTuple):
    a: TKR[float]
    b: TKR[float]
    c: TKR[float]


g = Graph(InParams, TKR[float])
x = g.task(add(g.inputs.a, g.inputs.b))
y = g.task(add(x, g.inputs.c))
workflow = g.finish_with_outputs(y)
```
and run them on different execution platforms

```python
storage = FileStorage(workflow_id=UUID(int=12345), name="Hello World Graph")
executor = ShellExecutor(registry_path=None, workflow_dir=storage.workflow_dir)
inputs = InParams(0, 0.25, 0.5)._asdict()
run_graph(storage, executor, workflow, inputs)
result = read_outputs(workflow, storage)
```



For a more in depth tutorial see our [full getting started guide][docs-getting-started].
A small demo tutorial presented at QCUF 2026 can be found in [this][tutorial] repository.


## Quick-start

Tierkreis works best with the [uv package manager][uv]. We strongly recommend using it as your package manager for Tierkreis projects.

To get started with Tierkreis start a new `uv` project in an empty directory with:

```bash
uv init
```

Then add Tierkreis to the project and run the project setup tool.

```bash
uv add tierkreis
uv run tkr init project
```

You can then run the generated example graph at `tkr/graphs.main.py`.

```bash
uv run tkr/graphs/main.py
```

## Packages

- **Tierkreis**: the core functionality `uv add tierkreis`
- **Tierkreis Visualization**: A simple web visualization tool for debugging workflows `uv add tierkreis-visualization`
- **Tierkreis Workers**: A collection of commonly used workers `uv add tkr-<name>-worker`
  - Qiskit Aer
  - Quantinuum Nexus
  - Quantinuum Pytket

## CLI

Tierkreis comes with a command line interface for running workflows.
To see all available options use `tkr --help`.
To run the hello world example in this repository from the cli

```bash
uv run tkr run \
  -g docs/source/examples/hello_world.py:graph \
  -i docs/source/examples/data/world.json \
  --uv \
  --registry-path docs/source/examples/example_workers/ \
  -o
```

Explanation:

- `-g` specifies the graph to run by specifying the location and function to run.
- `-i` specifies the input for the graph function. In this case it loads a json file with the contents `{"value": "World!"}`
- `--uv` enables the use of the UV executor.
- `--registry-path` specifies the location of the registry to use for the UV executor.
- `-o` enables output printing.

## Development

Requirements

- `uv` >= 0.9

See the justfile (requires `just`) for common development commands. For example:

```bash
just test
```

## Documentation

Tierkreis documentation can be found on our GitHub Pages [here][docs-home].

To build the documentation locally, run the command in the top level justfile.

```bash
just docs
```

Documentation will be built in HTML format by default and will be available in the `docs/build/html` folder.

For documentation, we use the [MyST](https://myst-parser.readthedocs.io/en/latest/index.html) parser with markdown files.

## License

This project is licensed under Apache License, Version 2.0 ([LICENSE][] or http://www.apache.org/licenses/LICENSE-2.0).

[docs-home]: https://quantinuum.github.io/tierkreis/
[docs-getting-started]: https://quantinuum.github.io/tierkreis/tutorial/index.html
[uv]: https://docs.astral.sh/uv/
[LICENSE]: https://github.com/Quantinuum/tierkreis/blob/main/LICENCE
[tutorial]: https://github.com/Quantinuum/tierkreis-tutorial
