# TKR: Tierkreis cli tool

Tierkreis includes tools to manage various aspects through the cli.
This is a comprehensive summary of the available commands and options.


## tkr init

## tkr run

```bash
uv run tkr run -o
> value: 1
```
This will use the following default locations:
1. The graph definition is used from `tkr/graphs/main.py:workflow`
2. The inputs are taken from `workflow_inputs.json`
3. `-o` enables the printing of outputs

This is the same as 
```bash
uv run tkr/graphs/main.py
> Value is: 1
```

## tkr vis
