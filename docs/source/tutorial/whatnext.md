# What Next?

You've reached the end of the tutorial.  You have learned how to
define a graph, fill it with tasks, and run the workflow.  We only
scratched the surface of what you can do with Tierkreis, but you know
enough to begin using it for your own work.

If you want to learn more details to fully leverage the power of
Tierkreis, the advanced user guide available
[here](../tutorial_advanced/index.md).  It includes further tutorials,
and detailed descriptions on how to write graphs, workers and
executors.

You might find the follow topics useful next:

* Defining a worker from a shell command
* Using HPC executors
* Working with the source distribution of Tierkreis

**TODO : add links to pages once they exist**

<!-- this should be a page of its own -->
## Building the examples from source

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


