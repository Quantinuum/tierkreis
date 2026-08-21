# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Upgrade devenv by @johnchildren in [#607](https://github.com/Quantinuum/tierkreis/pull/607)
- Update ruff and fix errors by @johnchildren in [#603](https://github.com/Quantinuum/tierkreis/pull/603)
- Reorganise imports in codegen by @johnchildren in [#601](https://github.com/Quantinuum/tierkreis/pull/601)
- Generate pipe characters for Union types by @johnchildren in [#600](https://github.com/Quantinuum/tierkreis/pull/600)
- Instrumentation by @philipp-seitz in [#595](https://github.com/Quantinuum/tierkreis/pull/595)
- Monitoring by @philipp-seitz in [#594](https://github.com/Quantinuum/tierkreis/pull/594)
- Add a Nexus executor by @johnchildren in [#579](https://github.com/Quantinuum/tierkreis/pull/579)
- Add hugr i64_s and f64 operations by @johnchildren in [#587](https://github.com/Quantinuum/tierkreis/pull/587)
- Add a Nexus HTTP client by @johnchildren in [#578](https://github.com/Quantinuum/tierkreis/pull/578)
- Rust api by @philipp-seitz in [#576](https://github.com/Quantinuum/tierkreis/pull/576)
- Add config struct for the runtime by @johnchildren in [#564](https://github.com/Quantinuum/tierkreis/pull/564)
- Allow reading state of multiple nodes by @johnchildren in [#567](https://github.com/Quantinuum/tierkreis/pull/567)
- Simplify state handing by @johnchildren in [#566](https://github.com/Quantinuum/tierkreis/pull/566)
- Make RuntimeState dyn compatible by @johnchildren in [#563](https://github.com/Quantinuum/tierkreis/pull/563)
- Support multiple concurrent workflows by @johnchildren in [#542](https://github.com/Quantinuum/tierkreis/pull/542)
- Async AssetStorage methods by @johnchildren in [#549](https://github.com/Quantinuum/tierkreis/pull/549)
- Re-enable WAL mode for sqlite by @johnchildren in [#555](https://github.com/Quantinuum/tierkreis/pull/555)
- Normalize license name by @johnchildren in [#561](https://github.com/Quantinuum/tierkreis/pull/561)
- Update package metadata by @johnchildren in [#554](https://github.com/Quantinuum/tierkreis/pull/554)
- Regenerate lockfiles by @johnchildren in [#551](https://github.com/Quantinuum/tierkreis/pull/551)
- Prepare for 2.1.0 release by @johnchildren in [#550](https://github.com/Quantinuum/tierkreis/pull/550)

### Fixed
- Flush files in FileAssetStorage on write by @johnchildren in [#605](https://github.com/Quantinuum/tierkreis/pull/605)
- Increase default n_iterations by @johnchildren in [#570](https://github.com/Quantinuum/tierkreis/pull/570)

## [2.1.0] - 2026-07-07

### Added
- Add fallback for filestorage by @philipp-seitz in [#540](https://github.com/Quantinuum/tierkreis/pull/540)
- Add sqlite build dependency by @philipp-seitz in [#539](https://github.com/Quantinuum/tierkreis/pull/539)
- Add GraphBuilder.embed (refactor init_tmodel) by @acl-cqc in [#342](https://github.com/Quantinuum/tierkreis/pull/342)

### Changed
- HPC improvements by @philipp-seitz in [#546](https://github.com/Quantinuum/tierkreis/pull/546)
- Revert CLI tool to python implementation by @johnchildren in [#544](https://github.com/Quantinuum/tierkreis/pull/544)
- Expand state interface by @johnchildren in [#543](https://github.com/Quantinuum/tierkreis/pull/543)
- Add wheels release for tierkreis by @johnchildren in [#538](https://github.com/Quantinuum/tierkreis/pull/538)
- Add new resource types to slurm by @philipp-seitz in [#533](https://github.com/Quantinuum/tierkreis/pull/533)
- Rename WorkflowState -> WorkflowRunState by @johnchildren in [#530](https://github.com/Quantinuum/tierkreis/pull/530)
- Implement AssetKind str format by @johnchildren in [#521](https://github.com/Quantinuum/tierkreis/pull/521)
- Disallow optional default inputs in graphs by @johnchildren in [#523](https://github.com/Quantinuum/tierkreis/pull/523)
- Clean up sqlite connection string logic by @johnchildren in [#524](https://github.com/Quantinuum/tierkreis/pull/524)
- Bump pyo3 from 0.28.3 to 0.29.0 in the cargo group across 1 directory by @dependabot[bot] in [#527](https://github.com/Quantinuum/tierkreis/pull/527)
- Change complex serialization format by @johnchildren in [#522](https://github.com/Quantinuum/tierkreis/pull/522)
- Use diesel_async and improve event handling by @johnchildren in [#520](https://github.com/Quantinuum/tierkreis/pull/520)
- Improve sqlite query efficiency by @johnchildren in [#516](https://github.com/Quantinuum/tierkreis/pull/516)
- Bump the third-party-minor group across 1 directory with 9 updates by @dependabot[bot] in [#517](https://github.com/Quantinuum/tierkreis/pull/517)
- Multiple locations in events by @johnchildren in [#515](https://github.com/Quantinuum/tierkreis/pull/515)
- Expand error handling and testing by @johnchildren in [#513](https://github.com/Quantinuum/tierkreis/pull/513)
- Expose the new orchestrator to python by @johnchildren in [#500](https://github.com/Quantinuum/tierkreis/pull/500)
- Implement all remaining nodes for nextgen by @johnchildren in [#492](https://github.com/Quantinuum/tierkreis/pull/492)
- Sql state by @philipp-seitz in [#501](https://github.com/Quantinuum/tierkreis/pull/501)
- Expand event types and add Updater by @johnchildren in [#498](https://github.com/Quantinuum/tierkreis/pull/498)
- Expand graph definition and add conversion by @johnchildren in [#497](https://github.com/Quantinuum/tierkreis/pull/497)
- Expand location implementation by @johnchildren in [#495](https://github.com/Quantinuum/tierkreis/pull/495)
- Better error handling and new asset funcs by @johnchildren in [#494](https://github.com/Quantinuum/tierkreis/pull/494)
- Add orchestrator dependencies by @johnchildren in [#493](https://github.com/Quantinuum/tierkreis/pull/493)
- Bump dependencies by @philipp-seitz in [#486](https://github.com/Quantinuum/tierkreis/pull/486)
- SQLite workflowstate by @philipp-seitz in [#483](https://github.com/Quantinuum/tierkreis/pull/483)
- Legacy graph conversion by @johnchildren in [#474](https://github.com/Quantinuum/tierkreis/pull/474)
- Add State storage component by @johnchildren in [#467](https://github.com/Quantinuum/tierkreis/pull/467)
- Enable forward refs with toposort by @philipp-seitz in [#477](https://github.com/Quantinuum/tierkreis/pull/477)
- Add itimes builtin to inmemory executor by @johnchildren in [#475](https://github.com/Quantinuum/tierkreis/pull/475)
- Add multiple frontend improvements by @philipp-seitz in [#468](https://github.com/Quantinuum/tierkreis/pull/468)
- Add rudimentary Location struct by @johnchildren in [#466](https://github.com/Quantinuum/tierkreis/pull/466)
- Get eval working (in principle) by @johnchildren in [#460](https://github.com/Quantinuum/tierkreis/pull/460)
- Refactor and expand orchestrator by @johnchildren in [#457](https://github.com/Quantinuum/tierkreis/pull/457)
- Refactor Executors and Events by @johnchildren in [#458](https://github.com/Quantinuum/tierkreis/pull/458)
- Next gen workers by @philipp-seitz in [#447](https://github.com/Quantinuum/tierkreis/pull/447)
- Rust orchestrator by @johnchildren in [#433](https://github.com/Quantinuum/tierkreis/pull/433)
- Psij executors by @philipp-seitz in [#426](https://github.com/Quantinuum/tierkreis/pull/426)
- Release 2.0.12 by @philipp-seitz in [#417](https://github.com/Quantinuum/tierkreis/pull/417)
- Pretty print json values to checkpoints by @johnchildren in [#414](https://github.com/Quantinuum/tierkreis/pull/414)
- Add breakpoints by @philipp-seitz in [#404](https://github.com/Quantinuum/tierkreis/pull/404)
- Change to new tutorial and user guide by @philipp-seitz in [#397](https://github.com/Quantinuum/tierkreis/pull/397)
- GraphBuilder->Graph with separate Workflow type by @acl-cqc in [#381](https://github.com/Quantinuum/tierkreis/pull/381)
- Improve qol (docs+cli) by @philipp-seitz in [#395](https://github.com/Quantinuum/tierkreis/pull/395)
- Update_worker_structure by @philipp-seitz in [#386](https://github.com/Quantinuum/tierkreis/pull/386)
- Update tierkreis package README by @johnchildren in [#388](https://github.com/Quantinuum/tierkreis/pull/388)
- Remove unused docs files by @johnchildren in [#387](https://github.com/Quantinuum/tierkreis/pull/387)
- Update pytket worker to new worker structure by @philipp-seitz in [#383](https://github.com/Quantinuum/tierkreis/pull/383)
- Fix CLI example in the README by @johnchildren in [#384](https://github.com/Quantinuum/tierkreis/pull/384)
- Improve some CLI docstrings by @johnchildren in [#380](https://github.com/Quantinuum/tierkreis/pull/380)
- Update workers and executors by @philipp-seitz in [#379](https://github.com/Quantinuum/tierkreis/pull/379)
- Add missing docstrings for the worker module by @johnchildren in [#364](https://github.com/Quantinuum/tierkreis/pull/364)
- Add missing docstrings for the graphs module by @johnchildren in [#365](https://github.com/Quantinuum/tierkreis/pull/365)
- Add missing docstrings for the controller by @johnchildren in [#357](https://github.com/Quantinuum/tierkreis/pull/357)
- Apply lint and docstrings to tests by @johnchildren in [#355](https://github.com/Quantinuum/tierkreis/pull/355)
- Add missing docstrings and fix lint errors by @johnchildren in [#352](https://github.com/Quantinuum/tierkreis/pull/352)

### Fixed
- Correct locking for sqlite workflows by @johnchildren in [#528](https://github.com/Quantinuum/tierkreis/pull/528)
- Additional checks to avoid running tasks by @johnchildren in [#531](https://github.com/Quantinuum/tierkreis/pull/531)
- Project initialization by @philipp-seitz in [#490](https://github.com/Quantinuum/tierkreis/pull/490)
- Small fixes and improvements by @philipp-seitz in [#470](https://github.com/Quantinuum/tierkreis/pull/470)
- Correct assertions in flakey subprocess test by @johnchildren in [#451](https://github.com/Quantinuum/tierkreis/pull/451)
- Cli template by @philipp-seitz in [#432](https://github.com/Quantinuum/tierkreis/pull/432)
- Cli init by @philipp-seitz in [#412](https://github.com/Quantinuum/tierkreis/pull/412)
- More fun with workers making graphs by @acl-cqc in [#392](https://github.com/Quantinuum/tierkreis/pull/392)
- Improve project init by @philipp-seitz in [#351](https://github.com/Quantinuum/tierkreis/pull/351)
- Fix lint everywhere else by @johnchildren in [#366](https://github.com/Quantinuum/tierkreis/pull/366)

### Removed
- Remove unused generics_in_ptype/model, put TypedGraphRef.inputs_type before outputs_type by @acl-cqc in [#391](https://github.com/Quantinuum/tierkreis/pull/391)

## [2.0.11] - 2026-02-24

### Changed
- Bump tkr minor version for release by @philipp-seitz in [#347](https://github.com/Quantinuum/tierkreis/pull/347)
- Add typos action on pull request by @johnchildren in [#345](https://github.com/Quantinuum/tierkreis/pull/345)
- Debug data by @philipp-seitz in [#325](https://github.com/Quantinuum/tierkreis/pull/325)
- Examples by @philipp-seitz in [#314](https://github.com/Quantinuum/tierkreis/pull/314)

### Fixed
- Restart diamond graph by @mwpb in [#323](https://github.com/Quantinuum/tierkreis/pull/323)
- Vis for graphdata by @philipp-seitz in [#334](https://github.com/Quantinuum/tierkreis/pull/334)
- Use in_edges in GraphData.add by @mwpb in [#322](https://github.com/Quantinuum/tierkreis/pull/322)
- Frontend logs by @philipp-seitz in [#324](https://github.com/Quantinuum/tierkreis/pull/324)

## [2.0.10] - 2026-02-10

### Added
- Support complex numbers by @mwpb in [#255](https://github.com/Quantinuum/tierkreis/pull/255)

### Changed
- Fix paths in cli by @philipp-seitz in [#319](https://github.com/Quantinuum/tierkreis/pull/319)
- Bump versions for release by @mwpb in [#318](https://github.com/Quantinuum/tierkreis/pull/318)
- Tierkreis riken starter by @mwpb in [#290](https://github.com/Quantinuum/tierkreis/pull/290)
- Extended cli by @philipp-seitz in [#251](https://github.com/Quantinuum/tierkreis/pull/251)
- Error handling by @philipp-seitz in [#261](https://github.com/Quantinuum/tierkreis/pull/261)
- Restart dependent nodes by @mwpb in [#274](https://github.com/Quantinuum/tierkreis/pull/274)
- Update quantinuum worker by @mwpb in [#269](https://github.com/Quantinuum/tierkreis/pull/269)
- Named loops by @philipp-seitz in [#259](https://github.com/Quantinuum/tierkreis/pull/259)
- Ndarray and custom serialization by @mwpb in [#257](https://github.com/Quantinuum/tierkreis/pull/257)
- Refactor and get hot reloading working by @mwpb in [#248](https://github.com/Quantinuum/tierkreis/pull/248)
- New builtins by @philipp-seitz in [#247](https://github.com/Quantinuum/tierkreis/pull/247)
- Generic pytket worker functionality by @philipp-seitz in [#235](https://github.com/Quantinuum/tierkreis/pull/235)
- Simplify path based storage by @mwpb in [#217](https://github.com/Quantinuum/tierkreis/pull/217)

### Fixed
- Optional outputs in read_outputs by @mwpb in [#309](https://github.com/Quantinuum/tierkreis/pull/309)
- Fix test IDs by @mwpb in [#277](https://github.com/Quantinuum/tierkreis/pull/277)
- Minor test fixes by @johnchildren in [#275](https://github.com/Quantinuum/tierkreis/pull/275)
- Enable slow test by @philipp-seitz in [#270](https://github.com/Quantinuum/tierkreis/pull/270)
- Include nested structs in stubs by @mwpb in [#250](https://github.com/Quantinuum/tierkreis/pull/250)

## [2.0.9] - 2025-11-05

### Changed
- Bump version for release by @mwpb in [#244](https://github.com/Quantinuum/tierkreis/pull/244)

## [2.0.8] - 2025-11-04

### Fixed
- Empty list map input by @mwpb in [#243](https://github.com/Quantinuum/tierkreis/pull/243)

## [2.0.7] - 2025-11-03

### Changed
- Release env var change by @mwpb in [#242](https://github.com/Quantinuum/tierkreis/pull/242)

### Fixed
- User environment in Executors by @philipp-seitz in [#241](https://github.com/Quantinuum/tierkreis/pull/241)

## [2.0.6] - 2025-10-28

### Changed
- New qulacs worker by @mwpb in [#240](https://github.com/Quantinuum/tierkreis/pull/240)
- Default arguments python workers by @mwpb in [#239](https://github.com/Quantinuum/tierkreis/pull/239)
- Improve error handling by @mwpb in [#238](https://github.com/Quantinuum/tierkreis/pull/238)
- Pytket worker by @philipp-seitz in [#232](https://github.com/Quantinuum/tierkreis/pull/232)
- Aer worker by @mwpb in [#228](https://github.com/Quantinuum/tierkreis/pull/228)
- To_qasm3_str by @philipp-seitz in [#229](https://github.com/Quantinuum/tierkreis/pull/229)

## [2.0.5] - 2025-10-01

### Changed
- Worker dir bug fix release by @mwpb in [#227](https://github.com/Quantinuum/tierkreis/pull/227)
- Fix/pytket_worker by @philipp-seitz in [#211](https://github.com/Quantinuum/tierkreis/pull/211)

### Fixed
- Worker tierkreis directory by @mwpb in [#226](https://github.com/Quantinuum/tierkreis/pull/226)

## [2.0.4] - 2025-09-30

### Added
- Add more debug logging by @mwpb in [#224](https://github.com/Quantinuum/tierkreis/pull/224)

### Changed
- Workflows based on trigger by @mwpb in [#222](https://github.com/Quantinuum/tierkreis/pull/222)
- Move write stubs to namespace by @mwpb in [#212](https://github.com/Quantinuum/tierkreis/pull/212)
- Include frontend assets in PyPI package by @mwpb in [#215](https://github.com/Quantinuum/tierkreis/pull/215)
- Improve pytket worker and docs by @philipp-seitz in [#203](https://github.com/Quantinuum/tierkreis/pull/203)
- Nexus worker improvements, graphs and docs by @mwpb in [#207](https://github.com/Quantinuum/tierkreis/pull/207)
- Include more justfile commands in ci by @mwpb in [#192](https://github.com/Quantinuum/tierkreis/pull/192)
- Improve frontend by @philipp-seitz in [#195](https://github.com/Quantinuum/tierkreis/pull/195)

### Fixed
- Write _error on non-zero exit code by @mwpb in [#218](https://github.com/Quantinuum/tierkreis/pull/218)
- Make copy of executor spec for each call by @mwpb in [#199](https://github.com/Quantinuum/tierkreis/pull/199)
- Environment variables by @mwpb in [#198](https://github.com/Quantinuum/tierkreis/pull/198)

## [2.0.2] - 2025-09-11

### Changed
- Bump versions by @mwpb in [#193](https://github.com/Quantinuum/tierkreis/pull/193)
- Executors by @philipp-seitz in [#190](https://github.com/Quantinuum/tierkreis/pull/190)
- Shell script worker by @mwpb in [#176](https://github.com/Quantinuum/tierkreis/pull/176)
- Hpc executor by @philipp-seitz in [#167](https://github.com/Quantinuum/tierkreis/pull/167)
- Use newer syntax internally by @johnchildren in [#177](https://github.com/Quantinuum/tierkreis/pull/177)
- Type checked GCD example by @mwpb in [#179](https://github.com/Quantinuum/tierkreis/pull/179)
- Recursion in typed builder by @mwpb in [#178](https://github.com/Quantinuum/tierkreis/pull/178)
- Use idl to generate stubs by @mwpb in [#170](https://github.com/Quantinuum/tierkreis/pull/170)
- Simplify loc by @philipp-seitz in [#172](https://github.com/Quantinuum/tierkreis/pull/172)
- Graphdata visualizer by @philipp-seitz in [#148](https://github.com/Quantinuum/tierkreis/pull/148)

### Fixed
- Pjsub executor by @mwpb in [#191](https://github.com/Quantinuum/tierkreis/pull/191)
- Idls without model definitions by @philipp-seitz in [#185](https://github.com/Quantinuum/tierkreis/pull/185)
- Graph visualizer bugs by @philipp-seitz in [#175](https://github.com/Quantinuum/tierkreis/pull/175)
- Align Python version correctly by @mwpb in [#174](https://github.com/Quantinuum/tierkreis/pull/174)
- Fix typo in docstring by @quantinuum-richard-morrison in [#161](https://github.com/Quantinuum/tierkreis/pull/161)

### Removed
- Remove extra read output ports by @mwpb in [#154](https://github.com/Quantinuum/tierkreis/pull/154)

## [2.0.1] - 2025-08-07

### Added
- Add tutorials for eval and loop by @mwpb in [#147](https://github.com/Quantinuum/tierkreis/pull/147)
- Add list to PType by @mwpb in [#133](https://github.com/Quantinuum/tierkreis/pull/133)
- Support generics in workers by @mwpb in [#129](https://github.com/Quantinuum/tierkreis/pull/129)

### Changed
- Bump versions for release by @mwpb in [#155](https://github.com/Quantinuum/tierkreis/pull/155)
- Core concepts by @philipp-seitz in [#151](https://github.com/Quantinuum/tierkreis/pull/151)
- Insist MAP nodes have integer idx by @mwpb in [#153](https://github.com/Quantinuum/tierkreis/pull/153)
- Python worker library by @mwpb in [#152](https://github.com/Quantinuum/tierkreis/pull/152)
- Graphbuilder using builtins by @mwpb in [#146](https://github.com/Quantinuum/tierkreis/pull/146)
- In Memory Execution by @philipp-seitz in [#136](https://github.com/Quantinuum/tierkreis/pull/136)
- Input protocols by @mwpb in [#145](https://github.com/Quantinuum/tierkreis/pull/145)
- Use PTypes in the inputs to run_graph by @johnchildren in [#143](https://github.com/Quantinuum/tierkreis/pull/143)
- Rename @worker.function to @worker.task by @mwpb in [#140](https://github.com/Quantinuum/tierkreis/pull/140)
- Create class portmapping by @mwpb in [#137](https://github.com/Quantinuum/tierkreis/pull/137)
- Coercion from annotations for serialisation by @johnchildren in [#139](https://github.com/Quantinuum/tierkreis/pull/139)
- Change order of args in map by @mwpb in [#134](https://github.com/Quantinuum/tierkreis/pull/134)
- Update hamiltonian example by @mwpb in [#131](https://github.com/Quantinuum/tierkreis/pull/131)
- Extend types allowed in workers and graph builder by @mwpb in [#126](https://github.com/Quantinuum/tierkreis/pull/126)
- Map example by @mwpb in [#122](https://github.com/Quantinuum/tierkreis/pull/122)
- Generate type safe graphbuilder from worker by @mwpb in [#117](https://github.com/Quantinuum/tierkreis/pull/117)
- Add fluent graphbuilder methods by @johnchildren in [#119](https://github.com/Quantinuum/tierkreis/pull/119)
- Use node def to indicate a node has started by @mwpb in [#118](https://github.com/Quantinuum/tierkreis/pull/118)
- Revert "Draft codegen" by @mwpb
- Draft codegen by @mwpb
- Implement Partial by @mwpb in [#93](https://github.com/Quantinuum/tierkreis/pull/93)
- Update readme by @philipp-seitz in [#99](https://github.com/Quantinuum/tierkreis/pull/99)
- Eager ifelse by @philipp-seitz in [#90](https://github.com/Quantinuum/tierkreis/pull/90)

### Fixed
- Map visualization by @philipp-seitz in [#135](https://github.com/Quantinuum/tierkreis/pull/135)
- Resolve overlapping instances for `map` by @johnchildren in [#138](https://github.com/Quantinuum/tierkreis/pull/138)
- Keep order of generics in format_pnamedmodel by @mwpb in [#132](https://github.com/Quantinuum/tierkreis/pull/132)
- Vis for ifelse by @mwpb in [#108](https://github.com/Quantinuum/tierkreis/pull/108)
- Remove surplus lines on builtin logging by @johnchildren in [#101](https://github.com/Quantinuum/tierkreis/pull/101)

## [2.0.0] - 2025-05-15

### Added
- Add a cli and graph restarting by @philipp-seitz in [#75](https://github.com/Quantinuum/tierkreis/pull/75)

### Changed
- Add a release workflow by @johnchildren in [#94](https://github.com/Quantinuum/tierkreis/pull/94)
- Further refactoring of workers by @johnchildren in [#88](https://github.com/Quantinuum/tierkreis/pull/88)
- Add a multiple executor by @johnchildren in [#85](https://github.com/Quantinuum/tierkreis/pull/85)
- Allow multiple acc_ports for LOOP by @mwpb in [#80](https://github.com/Quantinuum/tierkreis/pull/80)
- Test/add factorial test by @mwpb in [#74](https://github.com/Quantinuum/tierkreis/pull/74)
- Add worker shims by @johnchildren in [#48](https://github.com/Quantinuum/tierkreis/pull/48)
- Get the example working by @johnchildren in [#76](https://github.com/Quantinuum/tierkreis/pull/76)
- Add hamiltonian example by @johnchildren in [#73](https://github.com/Quantinuum/tierkreis/pull/73)
- Rename python -> tierkreis by @johnchildren in [#65](https://github.com/Quantinuum/tierkreis/pull/65)
- Move python to python subdirectory by @ss2165

### Fixed
- Cli by @philipp-seitz in [#86](https://github.com/Quantinuum/tierkreis/pull/86)

## [0.7.2] - 2024-06-26

### Added
- Add missing python package docstrings by @ss2165

### Changed
- Bump version to 0.7.2 by @ss2165 in [#17](https://github.com/Quantinuum/tierkreis/pull/17)
- Compress pytket types in wrapper by @ss2165

### Fixed
- Type annotation for `UnionConst` too restrictive by @ss2165

## [0.7.1] - 2024-04-26

### Changed
- Bump version to 0.7.1 by @ss2165 in [#15](https://github.com/Quantinuum/tierkreis/pull/15)

### Fixed
- Use print_exception in worker to print full traceback by @ss2165

## [0.7.0] - 2024-04-15

### Changed
- Bump python version to 0.7.0 by @ss2165 in [#14](https://github.com/Quantinuum/tierkreis/pull/14)
- Extract all metadata not just stack trace by @ss2165

### Removed
- Remove builder type annotations by @ss2165

## [0.6.1] - 2024-04-15

### Added
- Add builtin sleep operation by @ss2165

### Changed
- Worker dumps exception trace to stderr by @acl-cqc
- [Fix] Union of two different instantations of same Generic BaseModel by @acl-cqc
- [Fix] Optional enums by @acl-cqc

### Fixed
- Make readme work again by @ss2165

### Removed
- Remove authentication vestigial code by @ss2165

## [0.5.3] - 2024-03-08

### Changed
- Bump to version 0.5.3 by @ss2165 in [#11](https://github.com/Quantinuum/tierkreis/pull/11)
- Handle union of generic BaseModel by @acl-cqc
- [fix] Fix from_python for Union types using __pydantic_generic_metadata__ by @acl-cqc
- Update ruff to 0.3 and reformat by @ss2165
- Simpler handling of generic fields in classes by @ss2165

## [0.5.0] - 2024-02-22

### Added
- Add 'retry_secs' to Function nodes, bump {graph, signature}.proto to v1alpha1 by @acl-cqc
- Support pydantic basedmodels in conversions by @ss2165

### Changed
- Bump version to 0.5.0 by @ss2165 in [#10](https://github.com/Quantinuum/tierkreis/pull/10)
- Take optional python type annotation in `from_python` by @ss2165
- Allow registration of alternate convertible types by @ss2165
- OpaqueModel base class for deferring serialisation to pydantic by @ss2165
- Version bump to 0.4.1 by @ss2165
- Bump version to 0.4.0 and add changelog by @ss2165
- Simplify Circuit to just be a newtype for the json string by @ss2165
- Pull out job tracking in to new server interface by @ss2165
- Graph resumption by rewriting by @acl-cqc
- Avoid calling AbstractContextManager.__exit__ as it's 'abstract but defaults to None' by @acl-cqc

### Fixed
- Dont assume dataclass in `val_known_tk_type` by @ss2165

### Removed
- Remove betterproto monkeypatch by @ss2165

## [0.3.0] - 2023-12-19

### Added
- Support pydantic.Field and `init=False` by @ss2165
- Support more automated python type conversions by @ss2165
- Support union fields within structs by @ss2165

### Changed
- Bumpy python package to 0.3.0 by @ss2165 in [#7](https://github.com/Quantinuum/tierkreis/pull/7)
- More type fixes by @ss2165
- Pass stack-trace into run_function metadata by @acl-cqc
- Treat tuples as special structs by @ss2165
- Update python + use ruff, pyright for lint, type check by @ss2165

## [0.2.1] - 2023-12-01

### Added
- Add visualiser example and avoid subgraph outputs by @ss2165
- Add --py-inputs taking dict for python "eval" by @acl-cqc
- Add map builtin by @alexarice
- Add signatures and type inference to callbacks by @alexarice

### Changed
- Include py.typed in package by @ss2165 in [#6](https://github.com/Quantinuum/tierkreis/pull/6)
- Infra/nexus integration prototype by @mwpb
- Use new type annotations in pytket>=1.19 by @ss2165 in [#5](https://github.com/Quantinuum/tierkreis/pull/5)
- Bump version to 0.2 by @ss2165
- VizRuntime for use with tierkreis-viz by @ss2165
- Builder improvements: rename I/O, funcs as graphs, better graph decorator by @ss2165
- Refactor insert_graph by @acl-cqc
- Use insert graph rather than inline boxes in python_builtin by @ss2165
- Improve remove_key and insert_key functions by @alexarice
- Switch python to new callbacks by @alexarice

### Fixed
- Fix identity graph bug in insert_graph by @ss2165

### New Contributors
* @mwpb made their first contribution

## [0.1.0] - 2022-11-07

### Added
- Add option to type check at end of graph build by @ss2165
- Add basic location support by @alexarice
- Add Scoping construction to builder by @alexarice
- Add graph title if named by @ss2165
- Add more refined options for runtime type checking by @alexarice
- Add implementation and test for builtin/parallel by @alexarice
- Add kwarg run_graph overload by @ss2165
- Add isort to dev deps, pre commit and CI by @ss2165
- Add copy function for easier manual insertion of copies by @acl-cqc
- Tksl run/submit: add tksl-parsed inputs by @acl-cqc
- Add --proto flag to tksl to provide binary input by @ss2165
- Add tksl-start command to start local server by @ss2165
- Add tksl parsing for generics syntax by @ss2165
- Added type constraints to the python side. by @zrho
- Add block function to runtime client by @ss2165
- Add url scheme config variable by @ss2165
- Add optional name field to StructType by @ss2165
- Add qinclude! for circuit loading by @ss2165
- Add if, loop, struct type by @ss2165
- Add ability to inline boxes recursively in python graph by @ss2165
- Support myqos hosted runtime by @ss2165

### Changed
- Merge pull request #2 from CQCL/v0.1.0 by @ss2165 in [#2](https://github.com/Quantinuum/tierkreis/pull/2)
- Bump version to 0.1.0 by @ss2165
- Monkeypatch betterproto by @ss2165
- Split out type check and intgration from python/ by @ss2165
- Improve type errors by @ss2165
- Tidy+make deterministic node-renumbering in inline_boxes by @acl-cqc
- Mypy no longer complains by @alexarice
- Switch to integer node identifiers by @ss2165
- Review changes by @ss2165
- Don't update graph after type check by @ss2165
- Json Config file by @alexarice
- Make pylint happy with cli.py by @ss2165
- Rename frontend to client and consolidate by @ss2165
- Move builder.py to top level by @ss2165
- Export render_graph at top level by @ss2165
- Move pyruntime to own directory by @ss2165
- Move cli.py out of tksl folder by @ss2165
- Simplify feature dependency flags by @ss2165
- Move pyruntime to core by @alexarice
- Let pyruntime typecheck graph with inputs by @alexarice
- Enable runtime service for workers by @alexarice
- Update graphs correctly after typechecking by @alexarice
- Make visualisation print location name by @alexarice
- Take circuit.py from mushroom_dataclasses by @acl-cqc
- Use "poetry build" to combine pytket_worker dependencies by @acl-cqc
- Subclass builder for capturing inputs by @ss2165
- Allow worker namespaces to be created by [] by @alexarice
- Enable scoped execution by @alexarice
- Allow server_runtimes to be started with remote workers from python by @alexarice
- Refactor workers to not need to initialise worker object by @alexarice
- Fix integration test + document cli by @alexarice
- Make python ServerClient.run_graph use new run_graph interface by @alexarice
- Allow setting and preserving or graph i/o order by @ss2165
- Don't break words in viz annotation wrap by @ss2165
- Validate rust symbol names by @alexarice
- Hierarchical namespaces by @alexarice
- Run SC22 paper example (Rust + Python) as new workflow by @acl-cqc
- Ability to only unbox specific graphs in visualisation, by name by @ss2165
- Make visualisation annotations nicer by @ss2165
- Update betterproto to master by @alexarice
- Update mypy and use namespace_packages by @alexarice
- Store optional graph name in proto by @ss2165
- Merge copy nodes in viz and draw as points by @ss2165
- Register named structs as convertible by @ss2165
- Typo by @ss2165
- Improve visualisation of constants and thunks by @ss2165
- Capture more thunk names automatically by @ss2165
- Smaller capture port prefix by @ss2165
- New graph builder fronted using context managers and decorators by @ss2165
- Require use of TierkreisStruct or register_struct_convertible by @acl-cqc
- Fix handling of cyclic types containing "Optional" by @acl-cqc
- Trivial refactor: set_outputs/_to_nodeport use IncomingWireType for arg type by @acl-cqc
- Revert #168 (copy_value inserts and returns unused NodePort) by @acl-cqc
- Simple, asynchronous, python-only runtime by @ss2165
- Sort all imports with isort and run black by @ss2165
- Gitignore antlr generated folder by @ss2165
- Check edges are between nodes in the graph; fix MismatchedGraphs by @acl-cqc
- [Trivial][Refactor] TierkreisGraph: simplify add_const by @acl-cqc
- Steal TierkreisEdge.to_edge_handle; rename copy_n=>cp for black by @acl-cqc
- Copy_port => copy_value; use _to_nodeport by @acl-cqc
- Rename NodePort.{copy=>copy_value} by @acl-cqc
- Update messages a bit, add more tests by @acl-cqc
- Copy -> copy_port, add types, rename vars in test by @acl-cqc
- Shorten error messages to make pylint happy by @acl-cqc
- Distinguish error messages wrt. discard, make more like previous add_edge by @acl-cqc
- [Refactor] Inline _inline_box into inline_boxes by @acl-cqc
- Builtin/loop using Variant break|continue by @acl-cqc
- Stricter add edge, refactor inline_boxes by @acl-cqc
- Store reference to graph in NodeRef by @acl-cqc
- Use ports as networkx graph "edge keys" by @ss2165
- [Refactor] Make some more dataclasses frozen by @acl-cqc
- TKSL support for variant types, tag and match by @acl-cqc
- Make frozen; drop pointless __eq__ by @acl-cqc
- [refactor]TierkreisGraph c'tor: use self.add_node by @acl-cqc
- Allow set_outputs with constant values too by factoring out _to_nodeport by @acl-cqc
- In visit_Outport, raise an exception if no outports by @acl-cqc
- Check that there really is exactly one outport by @acl-cqc
- Parse negative float constants (fix KARL-197) by @acl-cqc
- Echo stdout/stderr via threads (fix KARL-196) by @acl-cqc
- Variant types by @acl-cqc
- [Refactor] python: rename add_node to add func, _add_node to add_node by @acl-cqc
- Enable try_autopython for container types by @acl-cqc
- Run antlr during wheel-building by @acl-cqc
- Allow profiling worker with cProfile by @acl-cqc
- Python library wrapping Rust type inference by @acl-cqc
- Decouple local_runtime, respect --server-logs by @acl-cqc
- Force separate identifiers for input and output port identifiers in visualisation by @ss2165
- Common up socket_address() by @acl-cqc
- Make mypy happy by @ss2165
- Update linting tool versions and apply black format by @ss2165
- Check worker types by unification with "Rigid" types by @acl-cqc
- Tksl-start docker command by @ss2165
- Rename `_get_edge` to `_to_tierkreis_type` by @ss2165
- Include tksl type annotations in graph by @ss2165
- Update to betterproto 2.0.0b4 by @ss2165
- Move test worker to python/tests/ by @ss2165
- Review suggestions by @ss2165
- Mypy and pylint fixes by @ss2165
- Source and changes for cqconf demo by @ss2165
- Define and test common types with by @ss2165
- Improve visualisation with unboxing by @ss2165
- 'simpler' loop which returns  option<struct<data>> by @ss2165
- Merge branch 'main' of https://github.com/CQCL-DEV/tierkreis by @ss2165
- Merge pull request #97 from CQCL-DEV/feature/bounded-types by @zrho
- Store extracted auth credentials in keyring by @ss2165
- [KARL-124] allow worker functions to make run graph requests to runtime by @ss2165
- Minor tidying by @ss2165
- Tksl cli submit/retrieve/status commands by @ss2165
- Allow python testing against remote server by @ss2165
- Optionally return different function from parser by @ss2165
- Set myqos runtime as tksl default by @ss2165
- Minor changes for release by @ss2165
- [KARL-117] include! preprocessing macro by @ss2165
- [KARL-116] add numerical comparison and logic operations by @ss2165
- [KARL-66] refactor worker python by @ss2165
- [KARL-115] add literal syntax for pairs by @ss2165
- [KARL-107] add ability to define and send type aliases by @ss2165
- [KARL-112] use circuit dataclass as structure by @ss2165
- Improves authentication error by @ss2165
- [KARL-113] add option type by @ss2165
- Minor lint fixes by @ss2165
- Parse fixes by @ss2165
- [KARL-70] add unit type by @ss2165
- Review suggestions by @ss2165
- Update worker dependencies by @ss2165
- Cli signature print by @ss2165
- Allow use imports of namespace functions by @ss2165
- Coloured cli output by @ss2165
- Optional tksl feature + commit antlr files by @ss2165
- Enable runtime cli option by @ss2165
- Tksl cli by @ss2165
- Use laeding capital for type names by @ss2165
- Organise tksl parse code by @ss2165
- Reverse node binding syntax by @ss2165
- Experiment with antlr grammar by @ss2165
- Map example by @ss2165
- Struct check by @ss2165
- Sequencing example by @ss2165
- Vec experiments by @ss2165
- Use port order by @ss2165
- Circuits and arrays by @ss2165
- Alises by @ss2165
- True/False + implicit eval by @ss2165
- Experiment with frontend language by @ss2165
- Separate myqos-worker initialisation by @ss2165
- Pipe tracing to devnull to prevent hangs by @ss2165
- Update docker image to full grpc by @ss2165
- Encode myqos authentication /me endpoint by @ss2165
- Authenticate on all endpoints by @ss2165
- Move to fully async grpc only client by @ss2165
- Optionally authenticate w/ mushroom by @ss2165
- Make python mypy and pylint compliant by @ss2165
- Rename delete function to discard by @ss2165
- Preserve port order in function declarations by @ss2165
- Better relative worker paths in local_runtime by @ss2165
- Utility function to render graph viz to file by @ss2165
- Automate more value conversions by @ss2165
- Rename runtime python package to worker for clarity by @ss2165
- Better docker error handling and cleanup by @ss2165
- Allow workers to specified as container images by @ss2165
- Run default workers automatically by @ss2165
- Propogate type of python error in worker by @ss2165
- More structured type errors in type inference API. by @zrho
- Tracing with opentelemetry. by @zrho
- Enable use of local workers as external workers. by @ss2165
- Docker server by @ss2165
- Consolidate in to one python package by @ss2165

### Fixed
- Fix bug where box output_order not used by @ss2165
- Fix cli runtime default docstring by @ss2165
- Fix type issues by @ss2165

### Removed
- Remove http server by @alexarice
- Remove incremental type checking by @ss2165
- Remove obsolete TKSL stuff, comments, docs by @acl-cqc
- Remove tksl by @alexarice
- Remove special-case for bool by @acl-cqc
- Remove Option, including from TKSL by @acl-cqc
- Remove _singleton by @acl-cqc
- Remove TierkreisGraph.copy_value (keeping test_commontypes.py, oops) by @acl-cqc
- [Refactor] insert_graph: remove node_refs dict by @acl-cqc
- Remove semicolon from 'if' syntax when no inputs by @acl-cqc
- Remove (and error on) unused "type: ignore" by @acl-cqc
- Cli improvements: delete, help text, extra options by @ss2165
- Remove unit type and add Some(T), None syntax for Option by @ss2165
- Remove circuit basic type by @ss2165
- Remove array type and add Vec type by @ss2165
- Removed actix dependency from runtime. by @zrho

[unreleased]: https://github.com/Quantinuum/tierkreis/compare/v2.1.0...HEAD
[2.1.0]: https://github.com/Quantinuum/tierkreis/compare/v2.0.11...v2.1.0
[2.0.11]: https://github.com/Quantinuum/tierkreis/compare/v2.0.10...v2.0.11
[2.0.10]: https://github.com/Quantinuum/tierkreis/compare/v2.0.9...v2.0.10
[2.0.9]: https://github.com/Quantinuum/tierkreis/compare/v2.0.8...v2.0.9
[2.0.8]: https://github.com/Quantinuum/tierkreis/compare/v2.0.7...v2.0.8
[2.0.7]: https://github.com/Quantinuum/tierkreis/compare/v2.0.6...v2.0.7
[2.0.6]: https://github.com/Quantinuum/tierkreis/compare/v2.0.5...v2.0.6
[2.0.5]: https://github.com/Quantinuum/tierkreis/compare/v2.0.4...v2.0.5
[2.0.4]: https://github.com/Quantinuum/tierkreis/compare/v2.0.2...v2.0.4
[2.0.2]: https://github.com/Quantinuum/tierkreis/compare/v2.0.1...v2.0.2
[2.0.1]: https://github.com/Quantinuum/tierkreis/compare/v2.0.0...v2.0.1
[2.0.0]: https://github.com/Quantinuum/tierkreis/compare/v0.7.2...v2.0.0
[0.7.2]: https://github.com/Quantinuum/tierkreis/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/Quantinuum/tierkreis/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/Quantinuum/tierkreis/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/Quantinuum/tierkreis/compare/v0.5.3...v0.6.1
[0.5.3]: https://github.com/Quantinuum/tierkreis/compare/v0.5.0...v0.5.3
[0.5.0]: https://github.com/Quantinuum/tierkreis/compare/v0.3.0...v0.5.0
[0.3.0]: https://github.com/Quantinuum/tierkreis/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Quantinuum/tierkreis/compare/v0.1.0...v0.2.1

<!-- generated by git-cliff -->
