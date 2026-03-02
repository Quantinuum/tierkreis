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
	npm i
	npm run build

stubs-generate dir:
  #!/usr/bin/env bash
  cd {{dir}}
  uv run main.py --stubs-path ./stubs.py

stubs-generate-api dir:
  #!/usr/bin/env bash
  cd {{dir}}
  uv run src/main.py --stubs-path src/api/stubs.py

generate: 
  just stubs-generate 'tierkreis/tierkreis/builtins'
  just stubs-generate 'tierkreis_workers/aer_worker'
  just stubs-generate 'tierkreis_workers/ibmq_worker'
  just stubs-generate 'tierkreis_workers/nexus_worker'
  just stubs-generate 'tierkreis_workers/pytket_worker'
  just stubs-generate 'tierkreis_workers/quantinuum_worker'
  just stubs-generate 'tierkreis_workers/qulacs_worker'

  just stubs-generate-api 'docs/source/examples/example_workers/auth_worker'
  just stubs-generate-api 'docs/source/examples/example_workers/error_worker'
  just stubs-generate-api 'docs/source/examples/example_workers/hello_world_worker'
  just stubs-generate-api 'docs/source/examples/example_workers/qsci_worker'
  just stubs-generate-api 'docs/source/examples/example_workers/scipy_worker'
  just stubs-generate-api 'docs/source/examples/example_workers/substitution_worker'

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
