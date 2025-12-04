uvrun := "uv run --all-extras"

default:
  @just --list

setup:
    uv sync --all-extras

test:
    {{uvrun}} pytest tierkreis --doctest-modules --cov=. --cov-report=html --cov-report=term

test-workers:
    {{uvrun}} pytest tierkreis_workers/pytket_worker --doctest-modules --cov=. --cov-report=html --cov-report=term

test-slow:
    {{uvrun}} pytest tierkreis --doctest-modules --cov=. --cov-report=html --cov-report=term --runslow

lint:
  {{uvrun}} ruff format --check
  {{uvrun}} ruff check
  {{uvrun}} pyright .

fix:
  {{uvrun}} ruff format
  {{uvrun}} ruff check --fix

docs:
  just docs/build	

[working-directory:'tierkreis_visualization']
serve:
	{{uvrun}} python tierkreis_visualization/main.py

[working-directory:'tierkreis_visualization/frontend']
prod:
  bun install
  bunx vite build .


examples:
  {{uvrun}} examples/hello_world_graph.py
  {{uvrun}} examples/defaults.py
  {{uvrun}} examples/error_handling_graph.py
  {{uvrun}} examples/simulate_parallel.py
  {{uvrun}} examples/symbolic_circuits.py
  {{uvrun}} examples/hamiltonian_graph.py
  {{uvrun}} examples/non_standard_tkr_dir.py
  {{uvrun}} examples/qsci_graph.py
  {{uvrun}} examples/scipy_graph.py
  SER_METHOD=dumps {{uvrun}} examples/scipy_graph.py
  SER_METHOD=tolist {{uvrun}} examples/scipy_graph.py
  SER_METHOD=save {{uvrun}} examples/scipy_graph.py
  {{uvrun}} examples/signing_graph.py

stubs-generate dir:
  #!/usr/bin/env bash
  cd {{dir}}
  uv run main.py --stubs-path ./stubs.py

generate: 
  just stubs-generate 'tierkreis/tierkreis/builtins'
  just stubs-generate 'tierkreis_workers/aer_worker'
  just stubs-generate 'tierkreis_workers/ibmq_worker'
  just stubs-generate 'tierkreis_workers/nexus_worker'
  just stubs-generate 'tierkreis_workers/pytket_worker'
  just stubs-generate 'tierkreis_workers/quantinuum_worker'
  just stubs-generate 'tierkreis_workers/qulacs_worker'

  just stubs-generate 'examples/example_workers/error_worker'
  just stubs-generate 'examples/example_workers/hello_world_worker'
  just stubs-generate 'examples/example_workers/substitution_worker'
  just stubs-generate 'examples/example_workers/chemistry_worker'
  just stubs-generate 'examples/example_workers/qsci_worker'
  just stubs-generate 'examples/example_workers/scipy_worker'

  mkdir -p examples/example_workers/aer_worker
  mkdir -p examples/example_workers/nexus_worker
  mkdir -p examples/example_workers/pytket_worker
  cp 'tierkreis_workers/aer_worker/stubs.py' tierkreis/tierkreis/aer_worker.py
  cp 'tierkreis_workers/ibmq_worker/stubs.py' tierkreis/tierkreis/ibmq_worker.py
  cp 'tierkreis_workers/nexus_worker/stubs.py' tierkreis/tierkreis/nexus_worker.py
  cp 'tierkreis_workers/pytket_worker/stubs.py' tierkreis/tierkreis/pytket_worker.py
  cp 'tierkreis_workers/quantinuum_worker/stubs.py' tierkreis/tierkreis/quantinuum_worker.py
  cp 'tierkreis_workers/qulacs_worker/stubs.py' tierkreis/tierkreis/qulacs_worker.py

check-stubs:
  just generate
  git update-index --refresh
  git diff-index --quiet HEAD --
  