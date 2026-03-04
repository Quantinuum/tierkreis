# Tierkreis Visualization

The Tierkreis Visualization package is a web server and frontend that can visualizer Tierkreis workflows before, during and after execution.

## Quick-start

Once you have installed the `tierkreis-visualization` package you can start the server with:

```bash
tkr vis
```

Which will start a web server that displays the contents of your Tierkreis checkpoints directory.

For more advanced usage see the [Tierkreis documentation pages for the visualizer](https://quantinuum.github.io/tierkreis/visualization.html).


## Development

To run the webserver and frontend from within this repository:

```bash
just serve
```

Note that you must first build the JavaScript bundle for the frontend.

```bash
just prod
```

Will (re)build the bundle and install the necessary dependencies.

If you wish to do both in a single command you can run:

```bash
just prod serve
```

### Hot reloading

While developing it is sometimes useful to reload the python code whenever it is changed.

To run the python web server in development mode so that changes made to it are automatically reloaded and applied.

```bash
uv run tkr-vis-dev
```

from the `tierkreis/tierkreis_visualization` directory and

```bash
npm run dev
```

from the `tierkreis/tierkreis_visualization/frontend` directory.

### OpenAPI

To regenerate the OpenAPI spec run

```bash
uv run tkr-vis-openapi
```

Running `npm run dev` afterward will automatically update the API stubs.
