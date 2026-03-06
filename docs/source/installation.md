# Installation

Tierkreis consist of multiple packages:

- `tierkreis`: the core functionality including the
- `tierkreis-visualization`: the visualization library which is necessary for the debug view
- independent [workers](./core_concepts) consisting of API and implementation packages
    - `tkr-*-worker`: **only the API**
    - `tkr-*-worker-impl`: API + Implementation 

The libraries can be installed with `pip` but we recommend using [uv](https://docs.astral.sh/uv/) read more in the [Getting Started](./getting_started.md) docs.

```
uv add tierkreis
uv add tierkreis-visualization
uv add tkr-*-worker
```
