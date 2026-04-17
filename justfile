uvrun := "uv run --all-extras"

default:
  @just --list

setup:
    uv sync --all-extras

test:
    cd tierkreis && cargo test
    {{uvrun}} pytest tierkreis --doctest-modules --cov=. --cov-report=html --cov-report=term

test-workers:
    {{uvrun}} pytest tierkreis_workers/pytket_worker --doctest-modules --cov=. --cov-report=html --cov-report=term

test-slow:
    {{uvrun}} pytest tierkreis --doctest-modules --cov=. --cov-report=html --cov-report=term --runslow

lint:
  cd tierkreis && cargo fmt --check
  cd tierkreis && cargo clippy --all-features --all-targets -- -D warnings
  {{uvrun}} ruff format --check
  {{uvrun}} ruff check
  {{uvrun}} pyright .

fix:
  cd tierkreis && cargo fmt
  cd tierkreis && cargo clippy --all-features --all-targets --fix
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
  uv run main.py --stubs-path ../api/api.py

generate: 
  just stubs-generate 'tierkreis/tierkreis/builtins'
  just stubs-generate 'tierkreis/tests/workers/graph'

  just stubs-generate-api 'tierkreis_workers/aer_worker/tkr_aer_worker_impl'
  just stubs-generate-api 'tierkreis_workers/ibmq_worker/tkr_ibmq_worker_impl'
  just stubs-generate-api 'tierkreis_workers/nexus_worker/tkr_nexus_worker_impl'
  just stubs-generate-api 'tierkreis_workers/pytket_worker/tkr_pytket_worker_impl'
  just stubs-generate-api 'tierkreis_workers/quantinuum_worker/tkr_quantinuum_worker_impl'
  just stubs-generate-api 'tierkreis_workers/qulacs_worker/tkr_qulacs_worker_impl'

  just stubs-generate-api 'docs/source/examples/example_workers/auth_worker/tkr_auth_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/error_worker/tkr_error_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/hello_world_worker/tkr_hello_world_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/multiple_outputs_worker/tkr_multiple_outputs_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/my_example_worker/tkr_my_example_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/qsci_worker/tkr_qsci_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/scipy_worker/tkr_scipy_worker_impl'
  just stubs-generate-api 'docs/source/examples/example_workers/substitution_worker/tkr_substitution_worker_impl'

  cp 'tierkreis_workers/aer_worker/api/api.py' tierkreis/tierkreis/aer_worker.py
  cp 'tierkreis_workers/ibmq_worker/api/api.py' tierkreis/tierkreis/ibmq_worker.py
  cp 'tierkreis_workers/nexus_worker/api/api.py' tierkreis/tierkreis/nexus_worker.py
  cp 'tierkreis_workers/pytket_worker/api/api.py' tierkreis/tierkreis/pytket_worker.py
  cp 'tierkreis_workers/quantinuum_worker/api/api.py' tierkreis/tierkreis/quantinuum_worker.py
  cp 'tierkreis_workers/qulacs_worker/api/api.py' tierkreis/tierkreis/qulacs_worker.py

check-stubs:
  just generate
  git update-index --refresh
  git diff-index --quiet HEAD --
