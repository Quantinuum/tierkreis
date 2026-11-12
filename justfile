profile := "dev"
pytestargs := "--doctest-modules --cov=. --cov-report=html --cov-report=term"


default:
  @just --list

uvrun profile +ARGS:
  uv run --all-extras --config-settings-package tierkreis:build-args='--profile {{profile}}' {{ARGS}}
  
setup profile=profile:
  uv sync --all-extras --config-settings-package tierkreis:build-args='--profile {{profile}}'

populate-cache profile=profile:
  @just setup
  cargo test --no-run

test:
  # We need to run a sync in the python environment to provide access to the workers
  # for the rust tests that use worker subprocesses.
  @just setup
  cargo test
  @just uvrun dev pytest tierkreis {{pytestargs}}

test-workers:
  @just setup
  @just uvrun dev pytest tierkreis_workers/pytket_worker {{pytestargs}}

test-slow:
  @just setup
  @just uvrun dev pytest tierkreis {{pytestargs}} --runslow

typos:
  typos
    
lint:
  @just setup
  cargo fmt --check
  cargo clippy --all-features --all-targets --no-deps -- -D warnings
  @just uvrun dev ruff format --check
  @just uvrun dev ruff check
  @just uvrun dev pyright .

fix:
  @just setup
  cargo fmt
  cargo clippy --all-features --all-targets --no-deps --fix
  @just uvrun dev ruff format
  @just uvrun dev ruff check --fix

docs profile=profile:
  @just setup
  just docs/build {{profile}}

serve:
  cargo run --manifest-path tierkreis/Cargo.toml --release --bin tkr -- serve 

[working-directory:'tierkreis/frontend']
prod:
  pnpm i
  pnpm run build

openapi:
  cargo run --manifest-path tierkreis/Cargo.toml --bin tkr -- openapi tierkreis/openapi.json

wheel:
  just prod
  maturin build --release --manifest-path tierkreis/Cargo.toml --out dist

bindings-generate dir:
  #!/usr/bin/env bash
  cd {{dir}}
  maturin generate-stubs --out . --features inspect
  # Unclear why but changing the order of these seems to upset the lint task.
  @just uvrun dev ruff check --fix
  @just uvrun dev ruff format

stubs-generate dir:
  #!/usr/bin/env bash
  cd {{dir}}
  @just uvrun dev main.py --stubs-path ./stubs.py

stubs-generate-api dir:
  #!/usr/bin/env bash
  cd {{dir}}
  @just uvrun dev main.py --stubs-path ../api/api.py

generate: 
  @just setup
  just bindings-generate 'tierkreis'
  just stubs-generate 'tierkreis/tests/workers/graph'

  just stubs-generate-api 'tierkreis_workers/aer_worker/tkr_aer_worker_impl'
  just stubs-generate-api 'tierkreis_workers/guppy_worker/tkr_guppy_worker_impl'
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


# Generate changelog entries for all sub-projects from git history.
generate-changelogs:
  @just generate-changelog tierkreis
  @just generate-changelog tierkreis_visualization
  @just generate-changelog tierkreis_workers/aer_worker
  @just generate-changelog tierkreis_workers/guppy_worker
  @just generate-changelog tierkreis_workers/ibmq_worker
  @just generate-changelog tierkreis_workers/nexus_worker
  @just generate-changelog tierkreis_workers/pytket_worker
  @just generate-changelog tierkreis_workers/quantinuum_worker
  @just generate-changelog tierkreis_workers/qulacs_worker

# Generate changelog entiries for a single sub-project from git history.
generate-changelog dir:
  #!/usr/bin/env bash
  cd {{dir}}
  git cliff -o CHANGELOG.md

# Check that the CHANGELOG has been updated to the latest version.
check-changelog:
  @just generate-changelogs
  git update-index --refresh
  git diff-index --quiet HEAD --
