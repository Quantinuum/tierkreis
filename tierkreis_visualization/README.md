# Tierkreis visualizer

To run with fast api:

```bash
just serve
```

To apply changes made in the frontend

```bash
just prod serve
```

will bundle the js before running the serve.

## Dev server

To run a updating dev server you can run

```bash
uv run tkr-vis-dev
```

from the `tierkreis/tierkreis_visualization` and

```bash
npm run dev
```

from `tierkreis/tierkreis_visualization/frontend`.

## OpenAPI

To regenerated the openapi spec run

```bash
uv run tkr-vis-openapi
```

Running `npm run dev` afterward will automatically update the api stubs.
