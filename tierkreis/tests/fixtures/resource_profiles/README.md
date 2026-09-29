# Resource Profile Fixtures

`cluster/` is the complete example profile used by the Rust resource-profile tests. It inherits the abstract `base/` profile, assigns `gpu_large` to `tkr-qulacs-worker`, and maps both `gpu_large` and `gpu` to `gpu/a100-4node.toml`. The base context defines `modules = ["openmpi/5", "legacy"]`; the GPU context removes `legacy` with `"!legacy"` and adds CUDA. Tests assert the aliases, resulting module list, inherited CPU settings, and executor-specific native options.

`retarget/` uses project and home roots together to test root precedence and profile-parent selection. The remaining fixtures cover `invalid_concurrency/`, `mapping_collision/`, `cycles/`, and `invalid_settings/`.

The test suite loads these directories directly with `ResourceProfiles::load`, so the files are test inputs and readable examples of the expected profile layout.
