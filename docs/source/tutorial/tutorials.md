# Advanced Examples

Once you have completed the beginners tutorial, you can look at advanced topics.

```{toctree}
:hidden:
:maxdepth: 1
../examples/types_and_defaults.ipynb
../examples/parallelism.ipynb
../examples/polling_and_dir.ipynb
../examples/errors_and_debugging.ipynb
../examples/restart.ipynb
../examples/signing_graph.ipynb
../examples/hpc.ipynb
../examples/scipy.ipynb
../examples/qsci.ipynb
```

## Building the examples

In the given examples you will be developing code involving Tierkreis workers.
Whenever you see an import containing `*_worker` this means one of the workers will be invoked (except for the `builtin`s).
```{important}
To run the examples you will need to have the worker code available too.
The simplest way to set this up is to clone the entire repository before running any of the examples.
```

```bash
git clone https://github.com/Quantinuum/tierkreis.git
```

To set up the environment we use uv:

```bash
cd tierkreis && uv sync --all-extras
```

Note that some third-party packages e.g. qulacs, automatically included via `uv sync --all-extras`, have other dependencies that you'll need to have installed on your system first: CMake and Boost (>=1.71).
When running the notebooks select the kernel corresponding to the uv environment.

## Runtime compatibility

Most examples use the Rust runtime. `parallelism` relies on lazy loading of the
selected conditional branch, and `restart` demonstrates attempt-based task
restarts. `errors_and_debugging` demonstrates pausing, inspection, and resuming
at breakpoints. Simulator-heavy parallelism, credentialed Nexus, and HPC
examples require deployment-specific resources and are not executed by the
documentation build.

Two examples remain on the legacy controller for a specific reason:

- `scipy` compares binary NumPy serializers and still needs typed or raw output
  handling in the example;
- `signing_graph` uses arbitrary shell and stdin/stdout adapters that do not
  have Rust-runtime equivalents.

The default-argument portion of `types_and_defaults` supplies the value
explicitly because runtime support for omitted worker defaults is still pending.
