set lists
set unstable

dev := "dev"
pytestargs := "--doctest-modules --cov=. --cov-report=html --cov-report=term"
uvrun(profile,args) := f"uv run --all-extras --config-settings-package tierkreis:build-args='--profile {{profile}}' {{args}}"

[private]
default:
  @just --list


[private]
uvrun profile +ARGS:
  uv run --all-extras --config-settings-package tierkreis:build-args='--profile {{profile}}' {{ARGS}}


[group('dev')]
setup profile=dev:
  uv sync --all-extras --config-settings-package tierkreis:build-args='--profile {{profile}}'


[group('ci')]
populate-cache profile=dev: (setup profile)
  cargo test --no-run


[group('build')]
[working-directory:'tierkreis/frontend']
build-frontend:
  pnpm i
  pnpm run build


[group('build')]
build-wheel: build-frontend
  maturin build --release --manifest-path tierkreis/Cargo.toml --out dist


# Run all linters and all tests.
check: lint test
  

# Run all test suites.
[group('test')]
test: rust-test python-test workers-test


[group('rust recipes')]
[group('test')]
rust-test: setup
  cargo test


[group('python recipes')]
[group('test')]
python-test: setup
  just uvrun dev pytest tierkreis {{pytestargs}}


[group('python recipes')]
[group('test')]
workers-test: setup
  just uvrun dev pytest tierkreis_workers/pytket_worker {{pytestargs}}


# Lint all project files.
[group('lint')]
lint: typos rust-lint python-lint


[group('lint')]
typos:
  typos

[group('rust recipes')]
[group('lint')]
rust-lint:
  cargo fmt --check
  cargo clippy --all-features --all-targets --no-deps -- -D warnings


[group('python recipes')]
[group('lint')]
python-lint: setup
  just uvrun dev ruff format --check
  just uvrun dev ruff check
  just uvrun dev pyright .


# Apply all lint fixes where possible.
[group('fix')]
fix: rust-fix python-fix


[group('rust recipes')]
[group('fix')]
rust-fix:
  cargo fmt
  cargo clippy --all-features --all-targets --no-deps --fix


[group('python recipes')]
[group('fix')]
python-fix: setup
  just uvrun dev ruff format
  just uvrun dev ruff check --fix
  

[group('docs')]
docs profile=dev: (setup profile)
  @just docs/build {{profile}}


[group('dev')]
serve:
  cargo run --manifest-path tierkreis/Cargo.toml --release --bin tkr -- serve 


[group('dev')]
openapi:
  cargo run --manifest-path tierkreis/Cargo.toml --bin tkr -- openapi tierkreis/openapi.json


# Generate the python type stubs for the Rust pyo3 bindings.
#
# They need fixing to satisfy the lint checks.
[private]
[working-directory:'tierkreis']
tierkreis-py-stubs-gen:
  maturin generate-stubs --out . --features inspect


[private]
tierkreis-py-stubs: tierkreis-py-stubs-gen python-fix


[private]
stubs-generate dir:
  #!/usr/bin/env bash
  cd {{dir}}
  @uvrun dev main.py --stubs-path ./stubs.py
  {{ uvrun("dev", "main.py --stubs-path ./stubs.py") }}


workers := ["aer", "guppy", "ibmq", "nexus", "pytket", "quantinuum", "qulacs"]


[private]
generate-worker-api name:
  #!/usr/bin/env bash
  cd "tierkreis_workers/{{name}}_worker/tkr_{{name}}_worker_impl"
  {{ uvrun("dev", "main.py --stubs-path ../api/api.py") }}
  cd ../../..
  cp 'tierkreis_workers/{{name}}_worker/api/api.py' 'tierkreis/tierkreis/{{name}}_worker.py'


[parallel]
generate-worker-apis: *(generate-worker-api *workers) 


docs-workers := ["auth", "error", "hello_world", "multiple_outputs", "my_example", "qsci", "scipy", "substitution"]


[private]
generate-docs-worker-api name:
  #!/usr/bin/env bash
  cd "docs/source/examples/example_workers/{{name}}_worker/tkr_{{name}}_worker_impl"
  {{ uvrun("dev", "main.py --stubs-path ../api/api.py") }}


[parallel]
generate-docs-worker-apis: *(generate-docs-worker-api *docs-workers) 


[group('dev')]
generate: setup tierkreis-py-stubs generate-worker-apis generate-docs-worker-apis
  @just stubs-generate 'tierkreis/tests/workers/graph'


# Check if there are any uncommitted changes in the working directory.
[private]
check-uncommitted:
  git update-index --refresh
  git diff-index --quiet HEAD --


# Check stubs for differences against source control after regenerating them.
[group('dev')]
check-stubs: generate check-uncommitted


# Generate changelog entiries for a single sub-project from git history.
[private]
[working-directory:'tierkreis']
generate-tierkreis-changelog:
  git cliff -o CHANGELOG.md


[private]
generate-worker-changelog name:
  #!/usr/bin/env bash
  cd tierkreis_workers/{{name}}_worker
  git cliff -o CHANGELOG.md


[private]
[parallel]
generate-worker-changelogs: *(generate-worker-changelog *workers) 


# Generate changelog entries for all sub-projects from git history.
[group('dev')]
generate-changelogs: generate-tierkreis-changelog generate-worker-changelogs


# Check that the CHANGELOG has been updated to the latest version.
[group('dev')]
check-changelogs: generate-changelogs check-uncommitted
