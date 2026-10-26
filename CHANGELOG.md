# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Update QRMI and disable pyO3 feature by @johnchildren in [#728](https://github.com/Quantinuum/tierkreis/pull/728)
- Bump the uv group across 4 directories with 1 update by @dependabot[bot] in [#732](https://github.com/Quantinuum/tierkreis/pull/732)
- Implement resume by @johnchildren in [#723](https://github.com/Quantinuum/tierkreis/pull/723)
- Add pausing to the Orchestrator by @johnchildren in [#722](https://github.com/Quantinuum/tierkreis/pull/722)
- Add node and workflow pause states by @johnchildren in [#721](https://github.com/Quantinuum/tierkreis/pull/721)
- Add breakpoint metadata to rust graphs by @johnchildren in [#718](https://github.com/Quantinuum/tierkreis/pull/718)
- Add get_raw_outputs to support bytes outputs by @johnchildren in [#716](https://github.com/Quantinuum/tierkreis/pull/716)
- Update docs to the new runtime API by @johnchildren in [#714](https://github.com/Quantinuum/tierkreis/pull/714)
- Fix argument handling and reduce cloning by @johnchildren in [#713](https://github.com/Quantinuum/tierkreis/pull/713)
- Update dependencies by @johnchildren in [#712](https://github.com/Quantinuum/tierkreis/pull/712)
- Lazily load subgraph inputs by @johnchildren in [#700](https://github.com/Quantinuum/tierkreis/pull/700)
- Bump vite from 7.3.5 to 7.3.7 in /tierkreis/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#711](https://github.com/Quantinuum/tierkreis/pull/711)
- Context orchestration by @philipp-seitz in [#709](https://github.com/Quantinuum/tierkreis/pull/709)
- Swap cli code gen to new runtime by @philipp-seitz in [#710](https://github.com/Quantinuum/tierkreis/pull/710)
- Allow build caching and deduplicate work by @philipp-seitz in [#707](https://github.com/Quantinuum/tierkreis/pull/707)
- Profile loading by @philipp-seitz in [#702](https://github.com/Quantinuum/tierkreis/pull/702)
- Update maturin requirement from <2.0,>=1.0 to >=1.15.0,<2.0 by @dependabot[bot] in [#706](https://github.com/Quantinuum/tierkreis/pull/706)
- Add QRMI support via an executor by @johnchildren in [#697](https://github.com/Quantinuum/tierkreis/pull/697)
- Execution context by @philipp-seitz in [#701](https://github.com/Quantinuum/tierkreis/pull/701)
- Remove more allocations by @johnchildren in [#696](https://github.com/Quantinuum/tierkreis/pull/696)
- Upgrade Guppy version by @philipp-seitz in [#695](https://github.com/Quantinuum/tierkreis/pull/695)
- Add AGENTS.md by @johnchildren in [#693](https://github.com/Quantinuum/tierkreis/pull/693)
- Remove lock on AssetStorageRegistry by @johnchildren in [#689](https://github.com/Quantinuum/tierkreis/pull/689)
- Reduce string clones in HPC wrapper by @johnchildren in [#694](https://github.com/Quantinuum/tierkreis/pull/694)
- Add loop traces by @johnchildren in [#684](https://github.com/Quantinuum/tierkreis/pull/684)
- Fix vulnerabilities. by @aidanCQ in [#691](https://github.com/Quantinuum/tierkreis/pull/691)
- Bump uv from 0.12.9 to 0.12.18 by @dependabot[bot] in [#688](https://github.com/Quantinuum/tierkreis/pull/688)
- Move frontend into rust by @philipp-seitz in [#679](https://github.com/Quantinuum/tierkreis/pull/679)
- Update cli by @philipp-seitz in [#667](https://github.com/Quantinuum/tierkreis/pull/667)
- Refactor restarts by @philipp-seitz in [#682](https://github.com/Quantinuum/tierkreis/pull/682)
- Refactor NodeEvent -> NodeEvents by @johnchildren in [#683](https://github.com/Quantinuum/tierkreis/pull/683)
- Migrate restarts by @philipp-seitz in [#670](https://github.com/Quantinuum/tierkreis/pull/670)
- Port hpc executors by @philipp-seitz in [#666](https://github.com/Quantinuum/tierkreis/pull/666)
- Bump urllib3 from 2.7.0 to 2.8.0 by @dependabot[bot] in [#677](https://github.com/Quantinuum/tierkreis/pull/677)
- Bump pyjwt from 2.13.0 to 2.15.0 by @dependabot[bot] in [#673](https://github.com/Quantinuum/tierkreis/pull/673)
- Python runtime interface by @johnchildren in [#651](https://github.com/Quantinuum/tierkreis/pull/651)
- Remove use of reserved port in tests by @johnchildren in [#658](https://github.com/Quantinuum/tierkreis/pull/658)
- Wrap test workers for tests by @johnchildren in [#657](https://github.com/Quantinuum/tierkreis/pull/657)
- Expand workflow state methods by @johnchildren in [#655](https://github.com/Quantinuum/tierkreis/pull/655)
- Change-up monitoring setup code by @johnchildren in [#653](https://github.com/Quantinuum/tierkreis/pull/653)
- New hpc executors by @philipp-seitz in [#637](https://github.com/Quantinuum/tierkreis/pull/637)
- Set 7 day cooldown period for dependabot by @johnchildren in [#645](https://github.com/Quantinuum/tierkreis/pull/645)
- Upgrade dependencies by @johnchildren in [#640](https://github.com/Quantinuum/tierkreis/pull/640)
- Add workflow names to RuntimeState by @johnchildren in [#636](https://github.com/Quantinuum/tierkreis/pull/636)
- Config files by @philipp-seitz in [#631](https://github.com/Quantinuum/tierkreis/pull/631)
- Simplify and fix race condition in tests by @johnchildren in [#625](https://github.com/Quantinuum/tierkreis/pull/625)
- Add Changelog generation from git history by @johnchildren in [#615](https://github.com/Quantinuum/tierkreis/pull/615)
- Refactor AssetStorage for better AssetKind by @johnchildren in [#628](https://github.com/Quantinuum/tierkreis/pull/628)
- Fix loop notebook timeout by @johnchildren in [#630](https://github.com/Quantinuum/tierkreis/pull/630)
- Bump tornado from 6.5.7 to 6.5.8 by @dependabot[bot] in [#629](https://github.com/Quantinuum/tierkreis/pull/629)
- Use assert_matches in tests by @johnchildren in [#608](https://github.com/Quantinuum/tierkreis/pull/608)
- Delay initial websocket ping by @johnchildren in [#606](https://github.com/Quantinuum/tierkreis/pull/606)
- Add missing builtin functions by @johnchildren in [#616](https://github.com/Quantinuum/tierkreis/pull/616)
- Add state restoration on runtime restart by @philipp-seitz in [#591](https://github.com/Quantinuum/tierkreis/pull/591)
- Upgrade devenv by @johnchildren in [#607](https://github.com/Quantinuum/tierkreis/pull/607)
- Update pyright by @johnchildren in [#604](https://github.com/Quantinuum/tierkreis/pull/604)
- Update ruff and fix errors by @johnchildren in [#603](https://github.com/Quantinuum/tierkreis/pull/603)
- Reorganise imports in codegen by @johnchildren in [#601](https://github.com/Quantinuum/tierkreis/pull/601)
- Generate pipe characters for Union types by @johnchildren in [#600](https://github.com/Quantinuum/tierkreis/pull/600)
- Instrumentation by @philipp-seitz in [#595](https://github.com/Quantinuum/tierkreis/pull/595)
- Monitoring by @philipp-seitz in [#594](https://github.com/Quantinuum/tierkreis/pull/594)
- Add a Nexus executor by @johnchildren in [#579](https://github.com/Quantinuum/tierkreis/pull/579)
- Add hugr i64_s and f64 operations by @johnchildren in [#587](https://github.com/Quantinuum/tierkreis/pull/587)
- Add a Nexus HTTP client by @johnchildren in [#578](https://github.com/Quantinuum/tierkreis/pull/578)
- Link tutorial in docs by @philipp-seitz in [#583](https://github.com/Quantinuum/tierkreis/pull/583)
- Fix link by @philipp-seitz in [#581](https://github.com/Quantinuum/tierkreis/pull/581)
- Rust api by @philipp-seitz in [#576](https://github.com/Quantinuum/tierkreis/pull/576)
- Add config struct for the runtime by @johnchildren in [#564](https://github.com/Quantinuum/tierkreis/pull/564)
- Allow reading state of multiple nodes by @johnchildren in [#567](https://github.com/Quantinuum/tierkreis/pull/567)
- Simplify state handing by @johnchildren in [#566](https://github.com/Quantinuum/tierkreis/pull/566)
- Make RuntimeState dyn compatible by @johnchildren in [#563](https://github.com/Quantinuum/tierkreis/pull/563)
- Support multiple concurrent workflows by @johnchildren in [#542](https://github.com/Quantinuum/tierkreis/pull/542)
- Async AssetStorage methods by @johnchildren in [#549](https://github.com/Quantinuum/tierkreis/pull/549)
- Re-enable WAL mode for sqlite by @johnchildren in [#555](https://github.com/Quantinuum/tierkreis/pull/555)
- Normalize license name by @johnchildren in [#561](https://github.com/Quantinuum/tierkreis/pull/561)
- Include guppy worker in release step by @johnchildren in [#560](https://github.com/Quantinuum/tierkreis/pull/560)

### Fixed
- Frontend websocket by @philipp-seitz in [#719](https://github.com/Quantinuum/tierkreis/pull/719)
- Ruff check by @philipp-seitz in [#698](https://github.com/Quantinuum/tierkreis/pull/698)
- Abort workflows on failure by @johnchildren in [#656](https://github.com/Quantinuum/tierkreis/pull/656)
- Correct error message for file saving by @johnchildren in [#654](https://github.com/Quantinuum/tierkreis/pull/654)
- Set stdout to null in subprocess executor by @johnchildren in [#639](https://github.com/Quantinuum/tierkreis/pull/639)
- Improve error handling for subprocess exec by @johnchildren in [#618](https://github.com/Quantinuum/tierkreis/pull/618)
- Abort background tasks on Executor drop by @johnchildren in [#624](https://github.com/Quantinuum/tierkreis/pull/624)
- Remove extra lock in SqlRuntimeState by @johnchildren in [#623](https://github.com/Quantinuum/tierkreis/pull/623)
- Flush files in FileAssetStorage on write by @johnchildren in [#605](https://github.com/Quantinuum/tierkreis/pull/605)
- Increase default n_iterations by @johnchildren in [#570](https://github.com/Quantinuum/tierkreis/pull/570)

### Removed
- Remove python server by @philipp-seitz in [#685](https://github.com/Quantinuum/tierkreis/pull/685)

## [2.1.1] - 2026-07-07

### Added
- Add version to action by @philipp-seitz in [#558](https://github.com/Quantinuum/tierkreis/pull/558)

### Changed
- Update package metadata by @johnchildren in [#554](https://github.com/Quantinuum/tierkreis/pull/554)
- Regenerate lockfiles by @johnchildren in [#551](https://github.com/Quantinuum/tierkreis/pull/551)
- Prepare for 2.1.0 release by @johnchildren in [#550](https://github.com/Quantinuum/tierkreis/pull/550)

### Fixed
- Add frontend package manager by @philipp-seitz in [#553](https://github.com/Quantinuum/tierkreis/pull/553)
- Frontend build by @philipp-seitz in [#548](https://github.com/Quantinuum/tierkreis/pull/548)

## [2.1.0] - 2026-07-07

### Added
- Add fallback for filestorage by @philipp-seitz in [#540](https://github.com/Quantinuum/tierkreis/pull/540)
- Add sqlite build dependency by @philipp-seitz in [#539](https://github.com/Quantinuum/tierkreis/pull/539)

### Changed
- HPC improvements by @philipp-seitz in [#546](https://github.com/Quantinuum/tierkreis/pull/546)
- Normalise release dispatch by @johnchildren in [#545](https://github.com/Quantinuum/tierkreis/pull/545)
- Revert CLI tool to python implementation by @johnchildren in [#544](https://github.com/Quantinuum/tierkreis/pull/544)
- Expand state interface by @johnchildren in [#543](https://github.com/Quantinuum/tierkreis/pull/543)
- Add wheels release for tierkreis by @johnchildren in [#538](https://github.com/Quantinuum/tierkreis/pull/538)
- Add new resource types to slurm by @philipp-seitz in [#533](https://github.com/Quantinuum/tierkreis/pull/533)
- Rename WorkflowState -> WorkflowRunState by @johnchildren in [#530](https://github.com/Quantinuum/tierkreis/pull/530)
- Implement AssetKind str format by @johnchildren in [#521](https://github.com/Quantinuum/tierkreis/pull/521)
- Upgrade python dependencies by @johnchildren in [#529](https://github.com/Quantinuum/tierkreis/pull/529)
- Disallow optional default inputs in graphs by @johnchildren in [#523](https://github.com/Quantinuum/tierkreis/pull/523)
- Clean up sqlite connection string logic by @johnchildren in [#524](https://github.com/Quantinuum/tierkreis/pull/524)
- Bump pyo3 from 0.28.3 to 0.29.0 in the cargo group across 1 directory by @dependabot[bot] in [#527](https://github.com/Quantinuum/tierkreis/pull/527)
- Bump vite from 7.3.3 to 7.3.5 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#511](https://github.com/Quantinuum/tierkreis/pull/511)
- Bump tornado from 6.5.6 to 6.5.7 by @dependabot[bot] in [#509](https://github.com/Quantinuum/tierkreis/pull/509)
- Bump starlette from 1.2.1 to 1.3.1 by @dependabot[bot] in [#518](https://github.com/Quantinuum/tierkreis/pull/518)
- Change complex serialization format by @johnchildren in [#522](https://github.com/Quantinuum/tierkreis/pull/522)
- Use diesel_async and improve event handling by @johnchildren in [#520](https://github.com/Quantinuum/tierkreis/pull/520)
- Improve sqlite query efficiency by @johnchildren in [#516](https://github.com/Quantinuum/tierkreis/pull/516)
- Multiple locations in events by @johnchildren in [#515](https://github.com/Quantinuum/tierkreis/pull/515)
- Expand error handling and testing by @johnchildren in [#513](https://github.com/Quantinuum/tierkreis/pull/513)
- Expose the new orchestrator to python by @johnchildren in [#500](https://github.com/Quantinuum/tierkreis/pull/500)
- Supply chain attack hardening with pnpm + config. by @aidanCQ in [#505](https://github.com/Quantinuum/tierkreis/pull/505)
- Implement all remaining nodes for nextgen by @johnchildren in [#492](https://github.com/Quantinuum/tierkreis/pull/492)
- Sql state by @philipp-seitz in [#501](https://github.com/Quantinuum/tierkreis/pull/501)
- Expand event types and add Updater by @johnchildren in [#498](https://github.com/Quantinuum/tierkreis/pull/498)
- Expand graph definition and add conversion by @johnchildren in [#497](https://github.com/Quantinuum/tierkreis/pull/497)
- Expand location implementation by @johnchildren in [#495](https://github.com/Quantinuum/tierkreis/pull/495)
- Better error handling and new asset funcs by @johnchildren in [#494](https://github.com/Quantinuum/tierkreis/pull/494)
- Add orchestrator dependencies by @johnchildren in [#493](https://github.com/Quantinuum/tierkreis/pull/493)
- Bump pytest-asyncio from 1.3.0 to 1.4.0 in the third-party-minor group across 1 directory by @dependabot[bot] in [#487](https://github.com/Quantinuum/tierkreis/pull/487)
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
- Bump cryptography from 46.0.6 to 46.0.7 by @dependabot[bot] in [#442](https://github.com/Quantinuum/tierkreis/pull/442)
- Bump python-multipart from 0.0.22 to 0.0.26 by @dependabot[bot] in [#444](https://github.com/Quantinuum/tierkreis/pull/444)
- Bump the third-party-minor group across 1 directory with 3 updates by @dependabot[bot] in [#441](https://github.com/Quantinuum/tierkreis/pull/441)
- Bump tornado from 6.5.4 to 6.5.5 in the uv group across 1 directory by @dependabot[bot] in [#406](https://github.com/Quantinuum/tierkreis/pull/406)
- Bump rollup from 4.44.2 to 4.59.0 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#361](https://github.com/Quantinuum/tierkreis/pull/361)
- Group dependabot updates by @johnchildren in [#440](https://github.com/Quantinuum/tierkreis/pull/440)
- Psij executors by @philipp-seitz in [#426](https://github.com/Quantinuum/tierkreis/pull/426)

### Fixed
- Top level workspace dependency by @philipp-seitz in [#534](https://github.com/Quantinuum/tierkreis/pull/534)
- Workaround for missing pyqir by @philipp-seitz in [#535](https://github.com/Quantinuum/tierkreis/pull/535)
- Uv dependency by @philipp-seitz in [#532](https://github.com/Quantinuum/tierkreis/pull/532)
- Correct locking for sqlite workflows by @johnchildren in [#528](https://github.com/Quantinuum/tierkreis/pull/528)
- Additional checks to avoid running tasks by @johnchildren in [#531](https://github.com/Quantinuum/tierkreis/pull/531)
- Project initialization by @philipp-seitz in [#490](https://github.com/Quantinuum/tierkreis/pull/490)
- Small fixes and improvements by @philipp-seitz in [#470](https://github.com/Quantinuum/tierkreis/pull/470)
- Devenv by @philipp-seitz in [#453](https://github.com/Quantinuum/tierkreis/pull/453)
- Correct assertions in flakey subprocess test by @johnchildren in [#451](https://github.com/Quantinuum/tierkreis/pull/451)
- Cli template by @philipp-seitz in [#432](https://github.com/Quantinuum/tierkreis/pull/432)

### Removed
- Remove issue templates by @philipp-seitz in [#446](https://github.com/Quantinuum/tierkreis/pull/446)

## [2.0.12-post.2] - 2026-04-10

### Fixed
- Explicitly state packages by @philipp-seitz in [#427](https://github.com/Quantinuum/tierkreis/pull/427)

## [2.0.12-post.1] - 2026-04-09

### Added
- Add GraphBuilder.embed (refactor init_tmodel) by @acl-cqc in [#342](https://github.com/Quantinuum/tierkreis/pull/342)

### Changed
- Release 2.0.12 by @philipp-seitz in [#417](https://github.com/Quantinuum/tierkreis/pull/417)
- Pretty print json values to checkpoints by @johnchildren in [#414](https://github.com/Quantinuum/tierkreis/pull/414)
- Add breakpoints by @philipp-seitz in [#404](https://github.com/Quantinuum/tierkreis/pull/404)
- Change to new tutorial and user guide by @philipp-seitz in [#397](https://github.com/Quantinuum/tierkreis/pull/397)
- GraphBuilder->Graph with separate Workflow type by @acl-cqc in [#381](https://github.com/Quantinuum/tierkreis/pull/381)
- Change docs ordering and improve links by @johnchildren in [#396](https://github.com/Quantinuum/tierkreis/pull/396)
- Improve qol (docs+cli) by @philipp-seitz in [#395](https://github.com/Quantinuum/tierkreis/pull/395)
- Update_worker_structure by @philipp-seitz in [#386](https://github.com/Quantinuum/tierkreis/pull/386)
- Re-order chapters and add link by @johnchildren in [#394](https://github.com/Quantinuum/tierkreis/pull/394)
- Portmapping for tasks docs by @mwpb in [#389](https://github.com/Quantinuum/tierkreis/pull/389)
- Update visualization README by @johnchildren in [#390](https://github.com/Quantinuum/tierkreis/pull/390)
- Update tierkreis package README by @johnchildren in [#388](https://github.com/Quantinuum/tierkreis/pull/388)
- Remove unused docs files by @johnchildren in [#387](https://github.com/Quantinuum/tierkreis/pull/387)
- Add a quickstart to root README by @johnchildren in [#385](https://github.com/Quantinuum/tierkreis/pull/385)
- Update pytket worker to new worker structure by @philipp-seitz in [#383](https://github.com/Quantinuum/tierkreis/pull/383)
- Fix CLI example in the README by @johnchildren in [#384](https://github.com/Quantinuum/tierkreis/pull/384)
- Make capitalisation consistent by @johnchildren in [#382](https://github.com/Quantinuum/tierkreis/pull/382)
- Improve some CLI docstrings by @johnchildren in [#380](https://github.com/Quantinuum/tierkreis/pull/380)
- Update workers and executors by @philipp-seitz in [#379](https://github.com/Quantinuum/tierkreis/pull/379)
- Add missing docstrings for the worker module by @johnchildren in [#364](https://github.com/Quantinuum/tierkreis/pull/364)
- Add missing docstrings for the graphs module by @johnchildren in [#365](https://github.com/Quantinuum/tierkreis/pull/365)
- Add missing docstrings for the controller by @johnchildren in [#357](https://github.com/Quantinuum/tierkreis/pull/357)
- Apply lint and docstrings to tests by @johnchildren in [#355](https://github.com/Quantinuum/tierkreis/pull/355)
- Add missing docstrings and fix lint errors by @johnchildren in [#353](https://github.com/Quantinuum/tierkreis/pull/353)
- Add missing docstrings and fix lint errors by @johnchildren in [#352](https://github.com/Quantinuum/tierkreis/pull/352)

### Fixed
- Update release command by @philipp-seitz in [#418](https://github.com/Quantinuum/tierkreis/pull/418)
- Cli init by @philipp-seitz in [#412](https://github.com/Quantinuum/tierkreis/pull/412)
- More fun with workers making graphs by @acl-cqc in [#392](https://github.com/Quantinuum/tierkreis/pull/392)
- Improve project init by @philipp-seitz in [#351](https://github.com/Quantinuum/tierkreis/pull/351)
- Fix lint everywhere else by @johnchildren in [#366](https://github.com/Quantinuum/tierkreis/pull/366)

### Removed
- Remove confusing BaseModel from docs by @mwpb in [#402](https://github.com/Quantinuum/tierkreis/pull/402)
- Remove unused generics_in_ptype/model, put TypedGraphRef.inputs_type before outputs_type by @acl-cqc in [#391](https://github.com/Quantinuum/tierkreis/pull/391)

## [2.0.10_viz_0.1.8] - 2026-02-24

### Changed
- Bump vis version for release by @philipp-seitz in [#348](https://github.com/Quantinuum/tierkreis/pull/348)

## [2.0.11] - 2026-02-24

### Changed
- Bump tkr minor version for release by @philipp-seitz in [#347](https://github.com/Quantinuum/tierkreis/pull/347)
- Enable typos precommit hook by @johnchildren in [#346](https://github.com/Quantinuum/tierkreis/pull/346)
- Add typos action on pull request by @johnchildren in [#345](https://github.com/Quantinuum/tierkreis/pull/345)
- Fix small typos and add typo config by @johnchildren in [#344](https://github.com/Quantinuum/tierkreis/pull/344)
- Debug data by @philipp-seitz in [#325](https://github.com/Quantinuum/tierkreis/pull/325)
- Examples by @philipp-seitz in [#314](https://github.com/Quantinuum/tierkreis/pull/314)
- Bump dependencies by @philipp-seitz in [#326](https://github.com/Quantinuum/tierkreis/pull/326)
- Visualizer TLC, tidy uses of `read_node_def` by @acl-cqc in [#320](https://github.com/Quantinuum/tierkreis/pull/320)

### Fixed
- Visualize loading of graphs with multiple cases by @philipp-seitz in [#341](https://github.com/Quantinuum/tierkreis/pull/341)
- Restart diamond graph by @mwpb in [#323](https://github.com/Quantinuum/tierkreis/pull/323)
- Vis for graphdata by @philipp-seitz in [#334](https://github.com/Quantinuum/tierkreis/pull/334)
- Use in_edges in GraphData.add by @mwpb in [#322](https://github.com/Quantinuum/tierkreis/pull/322)
- Frontend logs by @philipp-seitz in [#324](https://github.com/Quantinuum/tierkreis/pull/324)

## [2.0.10] - 2026-02-10

### Added
- Add screenshots to restart doc by @mwpb in [#291](https://github.com/Quantinuum/tierkreis/pull/291)
- Support complex numbers by @mwpb in [#255](https://github.com/Quantinuum/tierkreis/pull/255)

### Changed
- Fix paths in cli by @philipp-seitz in [#319](https://github.com/Quantinuum/tierkreis/pull/319)
- Bump versions for release by @mwpb in [#318](https://github.com/Quantinuum/tierkreis/pull/318)
- Tierkreis riken starter by @mwpb in [#290](https://github.com/Quantinuum/tierkreis/pull/290)
- Vis improvements by @philipp-seitz in [#297](https://github.com/Quantinuum/tierkreis/pull/297)
- Bump seroval from 1.3.2 to 1.5.0 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#302](https://github.com/Quantinuum/tierkreis/pull/302)
- Bump the uv group across 1 directory with 2 updates by @dependabot[bot] in [#299](https://github.com/Quantinuum/tierkreis/pull/299)
- Bump urllib3 from 2.5.0 to 2.6.3 in /examples/example_workers/qsci_worker in the uv group across 1 directory by @dependabot[bot] in [#300](https://github.com/Quantinuum/tierkreis/pull/300)
- Bump the npm_and_yarn group across 1 directory with 3 updates by @dependabot[bot] in [#301](https://github.com/Quantinuum/tierkreis/pull/301)
- Bump tar from 7.4.3 to 7.5.3 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#282](https://github.com/Quantinuum/tierkreis/pull/282)
- Extended cli by @philipp-seitz in [#251](https://github.com/Quantinuum/tierkreis/pull/251)
- Error handling by @philipp-seitz in [#261](https://github.com/Quantinuum/tierkreis/pull/261)
- Restart dependent nodes by @mwpb in [#274](https://github.com/Quantinuum/tierkreis/pull/274)
- Update quantinuum worker by @mwpb in [#269](https://github.com/Quantinuum/tierkreis/pull/269)
- Extra visualiser pages and nested updating by @mwpb in [#260](https://github.com/Quantinuum/tierkreis/pull/260)
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
- Readd visualize_graph by @philipp-seitz in [#254](https://github.com/Quantinuum/tierkreis/pull/254)
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
- Bump vite from 7.1.6 to 7.1.11 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#237](https://github.com/Quantinuum/tierkreis/pull/237)
- Pytket worker by @philipp-seitz in [#232](https://github.com/Quantinuum/tierkreis/pull/232)
- Aer worker by @mwpb in [#228](https://github.com/Quantinuum/tierkreis/pull/228)
- To_qasm3_str by @philipp-seitz in [#229](https://github.com/Quantinuum/tierkreis/pull/229)

### Fixed
- Update body nodes and tooltips by @philipp-seitz in [#231](https://github.com/Quantinuum/tierkreis/pull/231)

## [2.0.5] - 2025-10-01

### Changed
- Worker dir bug fix release by @mwpb in [#227](https://github.com/Quantinuum/tierkreis/pull/227)
- Fix/pytket_worker by @philipp-seitz in [#211](https://github.com/Quantinuum/tierkreis/pull/211)

### Fixed
- Worker tierkreis directory by @mwpb in [#226](https://github.com/Quantinuum/tierkreis/pull/226)

## [2.0.4_viz_0.1.7] - 2025-09-30

### Changed
- Remove double headers for JS files by @mwpb in [#225](https://github.com/Quantinuum/tierkreis/pull/225)

## [2.0.4] - 2025-09-30

### Added
- Add more debug logging by @mwpb in [#224](https://github.com/Quantinuum/tierkreis/pull/224)

## [2.0.3_1] - 2025-09-30

### Changed
- Workflows based on trigger by @mwpb in [#222](https://github.com/Quantinuum/tierkreis/pull/222)
- Checkout code first in CI by @mwpb in [#220](https://github.com/Quantinuum/tierkreis/pull/220)

### Fixed
- Include pypi environment by @mwpb in [#223](https://github.com/Quantinuum/tierkreis/pull/223)
- Ci steps not found by @mwpb in [#221](https://github.com/Quantinuum/tierkreis/pull/221)

## [2.0.3] - 2025-09-29

### Changed
- Fix direnv environments by @johnchildren in [#205](https://github.com/Quantinuum/tierkreis/pull/205)
- Move write stubs to namespace by @mwpb in [#212](https://github.com/Quantinuum/tierkreis/pull/212)
- Include frontend assets in PyPI package by @mwpb in [#215](https://github.com/Quantinuum/tierkreis/pull/215)
- Clean up docs and fix worker install by @mwpb in [#210](https://github.com/Quantinuum/tierkreis/pull/210)
- Write simple docs page for the Nexus worker by @mwpb in [#208](https://github.com/Quantinuum/tierkreis/pull/208)
- Nexus worker improvements, graphs and docs by @mwpb in [#207](https://github.com/Quantinuum/tierkreis/pull/207)
- Bump vite from 7.1.5 to 7.1.6 in /tierkreis_visualization/frontend in the npm_and_yarn group across 1 directory by @dependabot[bot] in [#206](https://github.com/Quantinuum/tierkreis/pull/206)
- Include more justfile commands in ci by @mwpb in [#192](https://github.com/Quantinuum/tierkreis/pull/192)
- Improve frontend by @philipp-seitz in [#195](https://github.com/Quantinuum/tierkreis/pull/195)

### Fixed
- Write _error on non-zero exit code by @mwpb in [#218](https://github.com/Quantinuum/tierkreis/pull/218)
- Fix idl gen by @philipp-seitz in [#209](https://github.com/Quantinuum/tierkreis/pull/209)
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
- Installation by @philipp-seitz in [#169](https://github.com/Quantinuum/tierkreis/pull/169)
- Graphdata visualizer by @philipp-seitz in [#148](https://github.com/Quantinuum/tierkreis/pull/148)
- Bump the npm_and_yarn group across 1 directory with 4 updates by @dependabot[bot] in [#128](https://github.com/Quantinuum/tierkreis/pull/128)

### Fixed
- Pjsub executor by @mwpb in [#191](https://github.com/Quantinuum/tierkreis/pull/191)
- Idls without model definitions by @philipp-seitz in [#185](https://github.com/Quantinuum/tierkreis/pull/185)
- Graph visualizer bugs by @philipp-seitz in [#175](https://github.com/Quantinuum/tierkreis/pull/175)
- Align Python version correctly by @mwpb in [#174](https://github.com/Quantinuum/tierkreis/pull/174)
- Fix assumed typo in docs by @quantinuum-richard-morrison in [#160](https://github.com/Quantinuum/tierkreis/pull/160)
- Fix typo in docstring by @quantinuum-richard-morrison in [#161](https://github.com/Quantinuum/tierkreis/pull/161)

### Removed
- Remove extra read output ports by @mwpb in [#154](https://github.com/Quantinuum/tierkreis/pull/154)

### New Contributors
* @quantinuum-richard-morrison made their first contribution in [#160](https://github.com/Quantinuum/tierkreis/pull/160)
* @dependabot[bot] made their first contribution in [#128](https://github.com/Quantinuum/tierkreis/pull/128)

## [2.0.1] - 2025-08-07

### Added
- Add tutorials for eval and loop by @mwpb in [#147](https://github.com/Quantinuum/tierkreis/pull/147)
- Add list to PType by @mwpb in [#133](https://github.com/Quantinuum/tierkreis/pull/133)
- Support generics in workers by @mwpb in [#129](https://github.com/Quantinuum/tierkreis/pull/129)
- Add new visualizer by @philipp-seitz in [#116](https://github.com/Quantinuum/tierkreis/pull/116)

### Changed
- Bump versions for release by @mwpb in [#155](https://github.com/Quantinuum/tierkreis/pull/155)
- Core concepts by @philipp-seitz in [#151](https://github.com/Quantinuum/tierkreis/pull/151)
- Insist MAP nodes have integer idx by @mwpb in [#153](https://github.com/Quantinuum/tierkreis/pull/153)
- Python worker library by @mwpb in [#152](https://github.com/Quantinuum/tierkreis/pull/152)
- Graphbuilder using builtins by @mwpb in [#146](https://github.com/Quantinuum/tierkreis/pull/146)
- In Memory Execution by @philipp-seitz in [#136](https://github.com/Quantinuum/tierkreis/pull/136)
- Input protocols by @mwpb in [#145](https://github.com/Quantinuum/tierkreis/pull/145)
- Remove type ignore in example by @johnchildren in [#142](https://github.com/Quantinuum/tierkreis/pull/142)
- Use PTypes in the inputs to run_graph by @johnchildren in [#143](https://github.com/Quantinuum/tierkreis/pull/143)
- Rename @worker.function to @worker.task by @mwpb in [#140](https://github.com/Quantinuum/tierkreis/pull/140)
- Create class portmapping by @mwpb in [#137](https://github.com/Quantinuum/tierkreis/pull/137)
- Coercion from annotations for serialisation by @johnchildren in [#139](https://github.com/Quantinuum/tierkreis/pull/139)
- Change order of args in map by @mwpb in [#134](https://github.com/Quantinuum/tierkreis/pull/134)
- Update hamiltonian example by @mwpb in [#131](https://github.com/Quantinuum/tierkreis/pull/131)
- Extend types allowed in workers and graph builder by @mwpb in [#126](https://github.com/Quantinuum/tierkreis/pull/126)
- Autodoc all tierkreis modules using autodoc2 by @johnchildren in [#124](https://github.com/Quantinuum/tierkreis/pull/124)
- Add basic sphinx configuration by @johnchildren in [#123](https://github.com/Quantinuum/tierkreis/pull/123)
- Map example by @mwpb in [#122](https://github.com/Quantinuum/tierkreis/pull/122)
- Generate type safe graphbuilder from worker by @mwpb in [#117](https://github.com/Quantinuum/tierkreis/pull/117)
- Add QSCI chemistry example by @johnchildren in [#121](https://github.com/Quantinuum/tierkreis/pull/121)
- Add fluent graphbuilder methods by @johnchildren in [#119](https://github.com/Quantinuum/tierkreis/pull/119)
- Use node def to indicate a node has started by @mwpb in [#118](https://github.com/Quantinuum/tierkreis/pull/118)
- Revert "Draft codegen" by @mwpb
- Draft codegen by @mwpb
- Use proper match in vis by @mwpb in [#115](https://github.com/Quantinuum/tierkreis/pull/115)
- Update issue templates by @philipp-seitz in [#102](https://github.com/Quantinuum/tierkreis/pull/102)
- Implement Partial by @mwpb in [#93](https://github.com/Quantinuum/tierkreis/pull/93)
- Update readme by @philipp-seitz in [#99](https://github.com/Quantinuum/tierkreis/pull/99)
- Eager ifelse by @philipp-seitz in [#90](https://github.com/Quantinuum/tierkreis/pull/90)

### Fixed
- Map visualization by @philipp-seitz in [#135](https://github.com/Quantinuum/tierkreis/pull/135)
- Resolve overlapping instances for `map` by @johnchildren in [#138](https://github.com/Quantinuum/tierkreis/pull/138)
- Keep order of generics in format_pnamedmodel by @mwpb in [#132](https://github.com/Quantinuum/tierkreis/pull/132)
- Vis for ifelse by @mwpb in [#108](https://github.com/Quantinuum/tierkreis/pull/108)
- Remove surplus lines on builtin logging by @johnchildren in [#101](https://github.com/Quantinuum/tierkreis/pull/101)

### Removed
- Remove broken link in docs by @mwpb in [#159](https://github.com/Quantinuum/tierkreis/pull/159)

## [2.0.0_viz_0.1.1] - 2025-05-15

### Changed
- Release all packages by @johnchildren in [#96](https://github.com/Quantinuum/tierkreis/pull/96)
- Fix syntax errors by @johnchildren in [#95](https://github.com/Quantinuum/tierkreis/pull/95)

### Fixed
- Ensure data files are included in the wheel by @johnchildren in [#97](https://github.com/Quantinuum/tierkreis/pull/97)

## [2.0.0] - 2025-05-15

### Added
- Add a cli and graph restarting by @philipp-seitz in [#75](https://github.com/Quantinuum/tierkreis/pull/75)
- Add all outputs by @mwpb
- Add stdout message by @philipp-seitz
- Add error message to controller by @philipp-seitz
- Add init to make pytest find modules by @philipp-seitz
- Add error path argument to workers by @philipp-seitz
- Add outputs to test graphs by @philipp-seitz
- Add empty check to error file by @philipp-seitz
- Add function template by @philipp-seitz
- Add Error status to js by @philipp-seitz
- Add read error functionality for visualization by @philipp-seitz
- Add error logging to executors by @philipp-seitz
- Add error functionality to controllers by @philipp-seitz
- Add test that can fail on demand by @philipp-seitz
- Add a simple biased coin toss as worker by @philipp-seitz
- Add envrc. by @aidanCQ in [#35](https://github.com/Quantinuum/tierkreis/pull/35)
- Add name to storage by @mwpb
- Add test for Eval by @mwpb
- Add default to NodeLocation by @mwpb
- Add MAP to visualiser by @mwpb
- Add tierkreis visualizer by @mwpb
- Add node definitions for non-function nodes by @mwpb
- Add optional logs_path to NodeDefinition by @mwpb
- Add extra line by @mwpb

### Changed
- Add a release workflow by @johnchildren in [#94](https://github.com/Quantinuum/tierkreis/pull/94)
- Further refactoring of workers by @johnchildren in [#88](https://github.com/Quantinuum/tierkreis/pull/88)
- Move common workers into a package by @johnchildren in [#87](https://github.com/Quantinuum/tierkreis/pull/87)
- Add a multiple executor by @johnchildren in [#85](https://github.com/Quantinuum/tierkreis/pull/85)
- Add tkr-vis utility by @johnchildren in [#82](https://github.com/Quantinuum/tierkreis/pull/82)
- Allow multiple acc_ports for LOOP by @mwpb in [#80](https://github.com/Quantinuum/tierkreis/pull/80)
- Test/add factorial test by @mwpb in [#74](https://github.com/Quantinuum/tierkreis/pull/74)
- Add worker shims by @johnchildren in [#48](https://github.com/Quantinuum/tierkreis/pull/48)
- Get the example working by @johnchildren in [#76](https://github.com/Quantinuum/tierkreis/pull/76)
- Add hamiltonian example by @johnchildren in [#73](https://github.com/Quantinuum/tierkreis/pull/73)
- The hello world example by @philipp-seitz in [#71](https://github.com/Quantinuum/tierkreis/pull/71)
- Set default shape to ellipse for input and output nodes. by @aidanCQ in [#68](https://github.com/Quantinuum/tierkreis/pull/68)
- Rename python -> tierkreis by @johnchildren in [#65](https://github.com/Quantinuum/tierkreis/pull/65)
- Error-visualization by @philipp-seitz in [#60](https://github.com/Quantinuum/tierkreis/pull/60)
- Merge pull request #67 from CQCL/feat/lazy-ifelse-node by @johnchildren in [#67](https://github.com/Quantinuum/tierkreis/pull/67)
- Comment failing test by @mwpb
- Before factorial working by @mwpb
- Merge pull request #52 from CQCL/push-nnsrzpyvmzuo by @johnchildren in [#52](https://github.com/Quantinuum/tierkreis/pull/52)
- Remove accidental print by @johnchildren
- Refactor and add dev dependencies by @johnchildren
- Add symbolic circuits example by @johnchildren
- Experimental JSGraph editor. by @aidanCQ in [#54](https://github.com/Quantinuum/tierkreis/pull/54)
- Merge pull request #51 from CQCL/experiment/horizontally-recursive-controller by @mwpb in [#51](https://github.com/Quantinuum/tierkreis/pull/51)
- Only link outputs on LOOP finish by @mwpb
- Fix double steps call by @mwpb
- Fix double call to .steps by @mwpb
- Test adding more than one output node to graph by @mwpb
- Fail early if multiple output nodes by @mwpb
- Tests and demo working by @mwpb
- Merge pull request #49 from CQCL/feat/error-reporting by @philipp-seitz in [#49](https://github.com/Quantinuum/tierkreis/pull/49)
- Move checking for node errors ro resume by @philipp-seitz
- Increase polling interval by @philipp-seitz
- Increase n_iterations int test by @philipp-seitz
- Lint code by @philipp-seitz
- Update example with errors by @philipp-seitz
- Improve code by @philipp-seitz
- Improve testcase by @philipp-seitz
- Make fail a named function by @philipp-seitz
- Update node_status to include error state by @philipp-seitz
- Simplify test_error by @philipp-seitz
- Simplify error checking and handling in controller by @philipp-seitz
- Rewrite and simplify worker to deterministically fail by @philipp-seitz
- Update test_error to always fail by @philipp-seitz
- Include func nodes in walking options by @philipp-seitz
- Improve styles. by @aidanCQ in [#45](https://github.com/Quantinuum/tierkreis/pull/45)
- Merge pull request #50 from CQCL/fix/remove-uv-dep-builtins by @mwpb in [#50](https://github.com/Quantinuum/tierkreis/pull/50)
- Log to file correctly by @mwpb
- Use current interpreter for builtins by @mwpb
- Merge pull request #47 from CQCL/push-pxunzxkuvlnx by @johnchildren in [#47](https://github.com/Quantinuum/tierkreis/pull/47)
- Add hello world example by @johnchildren
- Merge pull request #46 from CQCL/feat/allow-custom-uv-path-in-executor by @mwpb in [#46](https://github.com/Quantinuum/tierkreis/pull/46)
- Allow caller to configure uv path in uv executor by @mwpb
- Merge pull request #43 from CQCL/fix/use-enum-in-vis-get-node-data by @mwpb in [#43](https://github.com/Quantinuum/tierkreis/pull/43)
- Use enum for match by @mwpb
- Merge pull request #38 from CQCL/push-rvpzpynyqkvq by @johnchildren in [#38](https://github.com/Quantinuum/tierkreis/pull/38)
- Add assertion to appease typechecker by @johnchildren
- Improve visualizer layout. by @aidanCQ in [#42](https://github.com/Quantinuum/tierkreis/pull/42)
- Merge pull request #41 from CQCL/fix/update-vis-for-arbitrary-map by @mwpb in [#41](https://github.com/Quantinuum/tierkreis/pull/41)
- Satisfy ruff by @mwpb
- Update map in vis for arbitrary keys by @mwpb
- Merge pull request #39 from CQCL/push-rsmymsupmqkz by @johnchildren in [#39](https://github.com/Quantinuum/tierkreis/pull/39)
- Setup uv with direnv by @johnchildren
- Merge pull request #40 from CQCL/push-zwqoyonkqlrl by @johnchildren in [#40](https://github.com/Quantinuum/tierkreis/pull/40)
- Merge pull request #37 from CQCL/feat/update-individual-nodes-vis by @mwpb in [#37](https://github.com/Quantinuum/tierkreis/pull/37)
- Update individual nodes by @mwpb
- Fix import. by @aidanCQ in [#36](https://github.com/Quantinuum/tierkreis/pull/36)
- Merge pull request #34 from CQCL/push-lypzoktsloow by @johnchildren in [#34](https://github.com/Quantinuum/tierkreis/pull/34)
- Remove more unused things by @johnchildren
- Remove unused dependencies by @johnchildren
- Merge pull request #32 from CQCL/add-dependabot by @johnchildren in [#32](https://github.com/Quantinuum/tierkreis/pull/32)
- Remove unnecessary statement by @johnchildren
- Merge branch 'main' into add-dependabot by @johnchildren
- Add dependabot config by @johnchildren
- Merge pull request #33 from CQCL/push-nmomuloptowu by @johnchildren in [#33](https://github.com/Quantinuum/tierkreis/pull/33)
- Reorder justfile commands by @johnchildren
- Merge pull request #31 from CQCL/push-pqoknozwtllq by @johnchildren in [#31](https://github.com/Quantinuum/tierkreis/pull/31)
- Fix lint errors by @johnchildren
- Clean out unused code by @johnchildren
- Merge pull request #28 from CQCL/push-ukqowvnoksvk by @johnchildren in [#28](https://github.com/Quantinuum/tierkreis/pull/28)
- Split cases to help the type checker by @johnchildren
- Reformat files and add to check directive by @johnchildren
- Fix lint errors and simplify devenv by @johnchildren
- Clear up v1 implementation by @johnchildren
- Move PyGraph to JSGraph parsing to frontend and add initial design tokens. by @aidanCQ in [#30](https://github.com/Quantinuum/tierkreis/pull/30)
- Merge pull request #29 from CQCL/example/fix-test-outputs by @mwpb in [#29](https://github.com/Quantinuum/tierkreis/pull/29)
- Fix test outputs by @mwpb
- Merge pull request #24 from CQCL/experiment/recursive-controller by @mwpb in [#24](https://github.com/Quantinuum/tierkreis/pull/24)
- Merge pull request #27 from CQCL/push-xyptulqzqkqq by @mwpb in [#27](https://github.com/Quantinuum/tierkreis/pull/27)
- Use watchfile for visualization by @johnchildren
- Example graph for map with str keys by @mwpb
- Move builtins to Tierkreis repo by @mwpb
- Map works over arbitrary ports by @mwpb
- Clean up walk_map by @mwpb
- Descriptive name for map_eles by @mwpb
- Clean up parent location by @mwpb
- Simplify NodeType and NodeStep by @mwpb
- Sort folders by name by @mwpb
- Allow to configure in and out ports of map by @mwpb
- Test maps in series by @mwpb
- Move inputs into node def by @mwpb
- Use in_edges in vis by @mwpb
- Rename __default to __star__ by @mwpb
- Pattern match for finding parent nodes by @mwpb
- Do not assume dir exists in vis by @mwpb
- Revert linking experiment by @mwpb
- Mkdir instead of assert by @mwpb
- Update visualizer for new inputs by @mwpb
- Clean up start.py by @mwpb
- Revert magic value in EVAL by @mwpb
- Update visualiser with new Loc by @mwpb
- Test addition of Locs by @mwpb
- Put inputs into Eval by @mwpb
- Namespace archive by UID by @mwpb
- Satisfy ruff by @mwpb
- Fix NodeLocation by @mwpb
- Rename OutputLocation by @mwpb
- Simplify NodeLocation naming by @mwpb
- Update visualiser by @mwpb
- Use nodedef in walk by @mwpb
- Write nodedef files into filesystem by @mwpb
- Rename NodeDefinition to WorkerCallArgs by @mwpb
- Avoid jump when new data arrives by @mwpb
- Turn off physics in vis by @mwpb
- Fix test by @mwpb
- Improve vis js settings by @mwpb
- Implement fold/unfold functions by @mwpb
- Implement live update for LOOP by @mwpb
- Implement live update for EVAL in visualiser by @mwpb
- Execute map in parallel by @mwpb
- Satisfy ruff by @mwpb
- Implement fold_values as cheap reducer by @mwpb
- Satisfy pyright by @mwpb
- Implement map by @mwpb
- Satisfy ruff by @mwpb
- Update visualizer to use GraphData by @mwpb
- Merge pull request #25 from CQCL/test/new-graphdata by @mwpb in [#25](https://github.com/Quantinuum/tierkreis/pull/25)
- Use default where appropriate by @mwpb
- Return optional from parent by @mwpb
- Improve testing names by @mwpb
- Working with tierkreis-chemistry-demo by @mwpb
- Check exhaustiveness for node start by @mwpb
- Working GraphData by @mwpb
- Create README.md by @mwpb
- Satisfy ruff by @mwpb
- Improve function names by @mwpb
- Refactor into WalkResult by @mwpb
- Replace custom build system with hatchling by @mwpb
- Simplify user facing code by @mwpb
- Satisfy ruff by @mwpb
- Simplify test_resume by @mwpb
- Logging is done via node_definition by @mwpb
- Factor resume by get_nodes_to_start by @mwpb
- Factor out NodeRunData by @mwpb
- Satisfy ruff by @mwpb
- Fix test imports by @mwpb
- Ignore previous type errors by @mwpb
- Update models.py by @mwpb
- Revert Python version bump by @mwpb
- Revert StrEnum by @mwpb
- Bump python version by @mwpb
- Satisfy ruff by @mwpb
- Satisfy ruff by @mwpb
- Try fix CI by @mwpb
- Improve directory layout by @mwpb
- Fix typo by @mwpb
- Simplify exception handling by @mwpb
- Use StrEnum rather than overriding __str__ by @mwpb
- Revert test improvements by @mwpb
- Revert indentation by @mwpb
- Move logs into checkpoints dir by @mwpb
- Update .gitignore by @mwpb
- Fix comment by @mwpb
- Improve tests by @mwpb
- Improve tests by @mwpb
- Include old inputs in loops by @mwpb
- Take care of relative paths in generate_protos by @mwpb
- Move some files to tierkreis-chemistry-demo by @mwpb
- Create VQE graph by @mwpb
- Run worker in detached process by @mwpb
- Make stdout|err path configurable by @mwpb
- Improve debugging ShellExecutor by @mwpb
- Create executor interface and ShellExecutor by @mwpb
- Convert storage to use bytes by @mwpb
- Give up on match nodes for controller by @mwpb
- Draft nexus-worker and polling loop by @mwpb
- Move numerical-worker by @mwpb
- Link outputs through switch statements by @mwpb
- Link outputs across loop iterations by @mwpb
- Avoid extra read/write by @mwpb
- Fix typo by @mwpb
- Explicit enum for Python <3.11 by @mwpb
- Satisfy ruff by @mwpb
- Fix CI by @mwpb
- Fix CI errors by @mwpb
- Update pyproject.toml by @mwpb
- Update pyproject.toml by @mwpb
- Update pyproject.toml by @mwpb
- Update pyproject.toml by @mwpb
- Imported commits from old repo by @mwpb

### Fixed
- Cli by @philipp-seitz in [#86](https://github.com/Quantinuum/tierkreis/pull/86)
- Include visualizer in the workspace by @johnchildren
- Fix coverage report by @johnchildren
- Skip type checking tests by @johnchildren

### Removed
- Remove extra mark_finished by @mwpb
- Remove extra method by @mwpb
- Remove unreliable assertion by @philipp-seitz
- Remove function nodes from walk in favor of manual intervention by @philipp-seitz
- Remove executor from resume test, no longer needed by @philipp-seitz
- Remove unused code by @philipp-seitz
- Remove rule. by @aidanCQ in [#44](https://github.com/Quantinuum/tierkreis/pull/44)
- Remove log statements by @mwpb
- Remove extra str call by @mwpb
- Remove one instance of __star__ by @mwpb
- Remove some "value"s from tests by @mwpb
- Remove unused imports by @mwpb
- Remove linking for inputs by @mwpb
- Remove magic value in LOOP by @mwpb
- Remove magic loop VALUE by @mwpb
- Remove some linking by @mwpb
- Remove magic value from eval by @mwpb
- Remove extra models by @mwpb
- Remove extra validator by @mwpb
- Remove unused cop by @mwpb
- Remove magic value for root level graph by @mwpb
- Remove requirement to specify outputs by @mwpb
- Remove extra writing of node def by @mwpb
- Remove extra code by @mwpb
- Remove extra whitespace by @mwpb
- Remove extra method by @mwpb
- Remove extra print by @mwpb
- Remove extra line by @mwpb
- Remove extra deps by @mwpb
- Remove extra README by @mwpb
- Remove extra method by @mwpb
- Remove extra changes by @mwpb
- Remove extra imports by @mwpb

### New Contributors
* @johnchildren made their first contribution in [#94](https://github.com/Quantinuum/tierkreis/pull/94)
* @philipp-seitz made their first contribution in [#86](https://github.com/Quantinuum/tierkreis/pull/86)
* @aidanCQ made their first contribution in [#68](https://github.com/Quantinuum/tierkreis/pull/68)

## [0.7.3] - 2024-12-16

### Added
- Add rust and update python by @ss2165 in [#18](https://github.com/Quantinuum/tierkreis/pull/18)

### Changed
- Bump py version to 0.7.3 by @ss2165 in [#21](https://github.com/Quantinuum/tierkreis/pull/21)
- Move python to python subdirectory by @ss2165

### Fixed
- Fix docs building by @ss2165 in [#19](https://github.com/Quantinuum/tierkreis/pull/19)

## [0.7.2] - 2024-06-26

### Added
- Add docs optional deps by @ss2165
- Add docs build workflow by @ss2165 in [#16](https://github.com/Quantinuum/tierkreis/pull/16)
- Add missing python package docstrings by @ss2165

### Changed
- Bump version to 0.7.2 by @ss2165 in [#17](https://github.com/Quantinuum/tierkreis/pull/17)
- Compress pytket types in wrapper by @ss2165
- Install protoc in docs build by @ss2165
- Stop job request now means stop all attempts of same job id by @ss2165
- Update to checkpoint interface v3 by @ss2165
- Reinstate commented-out builder tests by @acl-cqc

### Fixed
- Type annotation for `UnionConst` too restrictive by @ss2165

## [0.7.1] - 2024-04-26

### Changed
- Bump version to 0.7.1 by @ss2165 in [#15](https://github.com/Quantinuum/tierkreis/pull/15)
- Upgrade opentelemtry and remove protobuf pin by @ss2165

### Fixed
- Use print_exception in worker to print full traceback by @ss2165

## [0.7.0] - 2024-04-15

### Changed
- Bump python version to 0.7.0 by @ss2165 in [#14](https://github.com/Quantinuum/tierkreis/pull/14)
- Extract all metadata not just stack trace by @ss2165

### Removed
- Remove builder type annotations by @ss2165

## [0.6.1] - 2024-04-15

### Changed
- Worker dumps exception trace to stderr by @acl-cqc

## [0.6.0] - 2024-04-15

### Added
- Add `DeleteCompleted` to `JobControl` by @ss2165
- Add builtin sleep operation by @ss2165

### Changed
- [triviall][refactor] Tidy sc22 example by @acl-cqc
- Update to checkpointing service v2 by @ss2165
- [Fix] Union of two different instantiations of same Generic BaseModel by @acl-cqc
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
- Move tests into test_generics.py, add test_pydantic_nested by @acl-cqc
- Update ruff to 0.3 and reformat by @ss2165
- Simpler handling of generic fields in classes by @ss2165
- Update python build workflow by @ss2165

## [0.5.0] - 2024-02-22

### Added
- Add 'retry_secs' to Function nodes, bump {graph, signature}.proto to v1alpha1 by @acl-cqc

### Changed
- Bump version to 0.5.0 by @ss2165 in [#10](https://github.com/Quantinuum/tierkreis/pull/10)
- Take optional python type annotation in `from_python` by @ss2165
- Update ruff by @ss2165
- Allow registration of alternate convertible types by @ss2165
- Version bump to 0.4.1 by @ss2165

### Fixed
- Dont assume dataclass in `val_known_tk_type` by @ss2165

## [0.4.0] - 2024-02-02

### Added
- Support pydantic basedmodels in conversions by @ss2165
- Add 'private_python' project w/ 'tierkreis_controller' module by @acl-cqc

### Changed
- Bump version to 0.4.0 and add changelog by @ss2165
- Simplify Circuit to just be a newtype for the json string by @ss2165
- Pull out job tracking in to new server interface by @ss2165
- Graph resumption by rewriting by @acl-cqc
- Fix pytket integration test by @acl-cqc
- Avoid calling AbstractContextManager.__exit__ as it's 'abstract but defaults to None' by @acl-cqc
- Pyproject.toml updates by @acl-cqc

### Fixed
- Shebang in pytket worker main by @ss2165 in [#9](https://github.com/Quantinuum/tierkreis/pull/9)
- Move jobs.proto to v1alpha1 directory by @ss2165

### Removed
- Remove betterproto monkeypatch by @ss2165
- Remove now-unnecessary CircArg wrapper by @ss2165

## [0.3.0] - 2023-12-19

### Added
- Support pydantic.Field and `init=False` by @ss2165
- Add "Any" stubs for networkx by @ss2165
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

### Changed
- Include py.typed in package by @ss2165 in [#6](https://github.com/Quantinuum/tierkreis/pull/6)
- Fixes for reproducing CI runs on MacOS by @acl-cqc
- Infra/nexus integration prototype by @mwpb
- Use new type annotations in pytket>=1.19 by @ss2165 in [#5](https://github.com/Quantinuum/tierkreis/pull/5)
- Custom skip marker for typecheck by @ss2165

### New Contributors
* @mwpb made their first contribution

## [0.2.0] - 2023-07-24

### Added
- Add appropriate test skip by @ss2165 in [#4](https://github.com/Quantinuum/tierkreis/pull/4)
- Add --py-inputs taking dict for python "eval" by @acl-cqc
- Add map builtin by @alexarice
- Add signatures and type inference to callbacks by @alexarice
- Add CITATION.bib by @ss2165 in [#3](https://github.com/Quantinuum/tierkreis/pull/3)

### Changed
- Bump version to 0.2 by @ss2165
- VizRuntime for use with tierkreis-viz by @ss2165
- Pin click version to stop ci mypy failure by @ss2165
- "Escape hatches" for running unknown functions by @acl-cqc
- [Tests] Fix --client-only by @acl-cqc
- Builder improvements: rename I/O, funcs as graphs, better graph decorator by @ss2165
- Refactor insert_graph by @acl-cqc
- Use insert graph rather than inline boxes in python_builtin by @ss2165
- Improve remove_key and insert_key functions by @alexarice
- Switch python to new callbacks by @alexarice
- Set up callbacks in rust by @alexarice

### Fixed
- Test of parallel map graph by @ss2165
- Fix identity graph bug in insert_graph by @ss2165

## [0.1.0] - 2022-11-07

### Added
- Add link to example by @ss2165
- Add run graph example by @ss2165
- Add worker section to readme by @ss2165
- Add building to readme by @ss2165
- Add paper example by @ss2165
- Add paper example by @ss2165
- Add option to type check at end of graph build by @ss2165
- Add fixture for outer_server_client by @alexarice
- Add missing 'await' to sc22_example.py by @acl-cqc
- Add pytest markers to pyproject.toml by @acl-cqc
- Add basic location support by @alexarice
- Add Scoping construction to builder by @alexarice
- Add graph title if named by @ss2165
- Add more refined options for runtime type checking by @alexarice
- Add implementation and test for builtin/parallel by @alexarice
- Add do_callback function to worker test by @alexarice
- Add kwarg run_graph overload by @ss2165
- Add isort to dev deps, pre commit and CI by @ss2165
- Add copy function for easier manual insertion of copies by @acl-cqc
- Tksl run/submit: add tksl-parsed inputs by @acl-cqc
- Add --proto flag to tksl to provide binary input by @ss2165
- Add tksl-start command to start local server by @ss2165
- Add --myqos-staging option to pytest by @acl-cqc
- Add tksl parsing for generics syntax by @ss2165
- Added type constraints to the python side. by @zrho
- Add block function to runtime client by @ss2165
- Add url scheme config variable by @ss2165
- Add optional name field to StructType by @ss2165
- Add qinclude! for circuit loading by @ss2165
- Add if, loop, struct type by @ss2165
- Add ability to inline boxes recursively in python graph by @ss2165
- Support myqos hosted runtime by @ss2165
- Added GRPC support to server. by @zrho
- Add script for rendering protos of graphs by @ss2165
- Add more graph methods by @ss2165
- Add simple test by @ss2165
- Add f64 type by @ss2165
- Add convenience methods to pgbuilder by @ss2165
- Add string type by @ss2165
- Add map type by @ss2165
- Add circuit (en/de)code by @ss2165

### Changed
- Merge pull request #2 from CQCL/v0.1.0 by @ss2165 in [#2](https://github.com/Quantinuum/tierkreis/pull/2)
- Merge branch 'main' into v0.1.0 by @ss2165
- Create LICENSE by @ss2165
- Begin readme by @ss2165
- Isort pytket_worker by @ss2165
- Up the sc22 max iterations to 100 by @ss2165
- Move sc22_worker to examples by @ss2165
- Move pytket_worker to python by @ss2165
- Bump version to 0.1.0 by @ss2165
- Build python 310/311 wheels by @ss2165
- Monkeypatch betterproto by @ss2165
- Python package readme by @ss2165
- Move sc22_worker to examples by @ss2165
- Move pytket_worker to python by @ss2165
- Split out type check and integration from python/ by @ss2165
- Improve type errors by @ss2165
- Tidy+make deterministic node-renumbering in inline_boxes by @acl-cqc
- Mypy no longer complains by @alexarice
- Switch to integer node identifiers by @ss2165
- Review changes by @ss2165
- Don't update graph after type check by @ss2165
- Json Config file by @alexarice
- Bound iterations of integration example by @ss2165
- Only import docker in test when needed by @ss2165
- Make pylint happy with cli.py by @ss2165
- Rename frontend to client and consolidate by @ss2165
- Move builder.py to top level by @ss2165
- Export render_graph at top level by @ss2165
- Use - instead of _ in sc22 flag to make pip happy by @ss2165
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
- Switch out petgraph dependency for internal portgraph by @ss2165
- Allow worker namespaces to be created by [] by @alexarice
- Enable scoped execution by @alexarice
- Allow server_runtimes to be started with remote workers from python by @alexarice
- Refactor workers to not need to initialise worker object by @alexarice
- Fix integration test + document cli by @alexarice
- No submodules and only pytket worker by @acl-cqc
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
- Make type inference only need signatures by reference by @alexarice
- Typo by @ss2165
- Improve visualisation of constants and thunks by @ss2165
- Capture more thunk names automatically by @ss2165
- Smaller capture port prefix by @ss2165
- New graph builder fronted using context managers and decorators by @ss2165
- Rename worker_test to test_worker by @alexarice
- Require use of TierkreisStruct or register_struct_convertible by @acl-cqc
- [Bugfix] Improve loop error reporting by @acl-cqc
- Fix handling of cyclic types containing "Optional" by @acl-cqc
- Update grpclib to pre-release to allow python 3.10 by @ss2165
- Trivial refactor: set_outputs/_to_nodeport use IncomingWireType for arg type by @acl-cqc
- [cleanup] test_frontend.py: don't import RuntimeClient twice by @acl-cqc
- Revert #168 (copy_value inserts and returns unused NodePort) by @acl-cqc
- Simple, asynchronous, python-only runtime by @ss2165
- Sort all imports with isort and run black by @ss2165
- Update pytket to 1.0 by @ss2165
- Gitignore antlr generated folder by @ss2165
- Check edges are between nodes in the graph; fix MismatchedGraphs by @acl-cqc
- [Trivial][Refactor] TierkreisGraph: simplify add_const by @acl-cqc
- Steal TierkreisEdge.to_edge_handle; rename copy_n=>cp for black by @acl-cqc
- Copy_port => copy_value; use _to_nodeport by @acl-cqc
- Rename NodePort.{copy=>copy_value} by @acl-cqc
- Update messages a bit, add more tests by @acl-cqc
- Improve test to check num_copies_discards; fix naming a_squared_plus_ab by @acl-cqc
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
- Surface underlying GraphBuilderError's from parsing protobufs by @acl-cqc
- In visit_Outport, raise an exception if no outports by @acl-cqc
- Check that there really is exactly one outport by @acl-cqc
- Parse negative float constants (fix KARL-197) by @acl-cqc
- Echo stdout/stderr via threads (fix KARL-196) by @acl-cqc
- Fix protobuf version by @acl-cqc
- Variant types by @acl-cqc
- [Refactor] python: rename add_node to add func, _add_node to add_node by @acl-cqc
- Enable try_autopython for container types by @acl-cqc
- Run antlr during wheel-building by @acl-cqc
- Allow profiling worker with cProfile by @acl-cqc
- Move pylint config into pyproject.toml and widen coverage by @acl-cqc
- Python library wrapping Rust type inference by @acl-cqc
- Fix CI failure in pylint by @acl-cqc
- Decouple local_runtime, respect --server-logs by @acl-cqc
- Don't use poetry by @acl-cqc
- Force separate identifiers for input and output port identifiers in visualisation by @ss2165
- Skip local runtime test on release by @ss2165
- Common up socket_address() by @acl-cqc
- Demo tksl type error fixes by @ss2165
- Update test worker lockfile by @ss2165
- Make mypy happy by @ss2165
- Update linting tool versions and apply black format by @ss2165
- Pin markupsafe version to counter incompatibility with jinja2 by @ss2165
- Use poethepoet for proto generation task by @ss2165
- Check worker types by unification with "Rigid" types by @acl-cqc
- Tksl-start docker command by @ss2165
- Rename `_get_edge` to `_to_tierkreis_type` by @ss2165
- Include tksl type annotations in graph by @ss2165
- Update to betterproto 2.0.0b4 by @ss2165
- Set 0.0.0 placeholder version for python by @ss2165
- Move test worker to python/tests/ by @ss2165
- Move pytket worker tests to submodule by @ss2165
- Poetry update by @ss2165
- Review suggestions by @ss2165
- Mypy and pylint fixes by @ss2165
- Source and changes for cqconf demo by @ss2165
- Improve visualisation with unboxing by @ss2165
- 'simpler' loop which returns  option<struct<data>> by @ss2165
- Partial application builtin by @ss2165
- Merge branch 'main' of https://github.com/CQCL-DEV/tierkreis by @ss2165
- Merge pull request #97 from CQCL-DEV/feature/bounded-types by @zrho
- Store extracted auth credentials in keyring by @ss2165
- [KARL-124] allow worker functions to make run graph requests to runtime by @ss2165
- Minor tidying by @ss2165
- Tksl cli submit/retrieve/status commands by @ss2165
- Allow python testing against remote server by @ss2165
- Higher order tksl example by @ss2165
- Optionally return different function from parser by @ss2165
- Set myqos runtime as tksl default by @ss2165
- Minor changes for release by @ss2165
- [KARL-117] include! preprocessing macro by @ss2165
- [KARL-116] add numerical comparison and logic operations by @ss2165
- [KARL-116] add int/float arithmetic operations to builtins by @ss2165
- [KARL-66] refactor worker python by @ss2165
- [KARL-115] add literal syntax for pairs by @ss2165
- [KARL-107] add ability to define and send type aliases by @ss2165
- [KARL-112] use circuit dataclass as structure by @ss2165
- Improves authentication error by @ss2165
- Update poetry lock by @ss2165
- [KARL-113] add option type by @ss2165
- Minor lint fixes by @ss2165
- Update to pre-release poetry by @ss2165
- Parse fixes by @ss2165
- [KARL-70] add unit type by @ss2165
- Review suggestions by @ss2165
- Update worker dependencies by @ss2165
- Cli signature print by @ss2165
- Allow use imports of namespace functions by @ss2165
- Coloured cli output by @ss2165
- Optional tksl feature + commit antlr files by @ss2165
- Update packages by @ss2165
- Enable runtime cli option by @ss2165
- Tksl cli by @ss2165
- More tksl parse tests by @ss2165
- Use laeding capital for type names by @ss2165
- Use click for view_graph by @ss2165
- Organise tksl parse code by @ss2165
- Reverse node binding syntax by @ss2165
- Experiment with antlr grammar by @ss2165
- Map example by @ss2165
- Struct check by @ss2165
- Sequencing example by @ss2165
- Vec experiments by @ss2165
- Use port order by @ss2165
- Circuits and arrays by @ss2165
- Aliases by @ss2165
- True/False + implicit eval by @ss2165
- Experiment with frontend language by @ss2165
- Skip local tests on release workflow by @ss2165
- Revert "release workflow adaptive tests" by @ss2165
- Show output from nested runtime by @ss2165
- Release workflow adaptive tests by @ss2165
- Separate myqos-worker initialisation by @ss2165
- Pipe tracing to devnull to prevent hangs by @ss2165
- Update docker image to full grpc by @ss2165
- Encode myqos authentication /me endpoint by @ss2165
- Authenticate on all endpoints by @ss2165
- Move to fully async grpc only client by @ss2165
- Optionally authenticate w/ mushroom by @ss2165
- Switch to manual releases from actions UI by @ss2165
- Make python mypy and pylint compliant by @ss2165
- Rename delete function to discard by @ss2165
- Preserve port order in function declarations by @ss2165
- Update lock file by @ss2165
- Upgrade to python 3.9 by @ss2165
- Better relative worker paths in local_runtime by @ss2165
- Utility function to render graph viz to file by @ss2165
- Automate more value conversions by @ss2165
- Rename runtime python package to worker for clarity by @ss2165
- Update pytket by @ss2165
- Better docker error handling and cleanup by @ss2165
- Allow workers to specified as container images by @ss2165
- Run default workers automatically by @ss2165
- Propagate type of python error in worker by @ss2165
- Report full error context to the client when running graphs. by @zrho
- More structured type errors in type inference API. by @zrho
- Tracing with opentelemetry. by @zrho
- Enable use of local workers as external workers. by @ss2165
- [bugfix] fix to include protos files in builds by @ss2165
- Release workflow by @ss2165
- Docker server by @ss2165
- Consolidate in to one python package by @ss2165
- More tracing in runtime server. by @zrho
- Use random UUIDs as task ids. by @zrho
- Tasks for long running graph computations. by @zrho
- Context manager for local server by @ss2165
- Make graphviz default display for graphs in jupyter by @ss2165
- Client provided context for type checked graph builds by @ss2165
- New visualisation using html-label nodes by @ss2165
- Replicate newnode behaviour in python frontend by @ss2165
- Type check on graph modification by @ss2165
- Syntactic sugar for dealing with arrays by @ss2165
- Runtime server now implements the worker interface itself. by @zrho
- Refactor runtime. by @zrho
- [KARL-45] [feature] insert_subgraph method by @ss2165
- Reafactor out function and viz from tierkreis_graph by @ss2165
- Python runtime error improvements by @ss2165
- TierkreisGraph pair/delete utility methods by @ss2165
- Update to pytket 0.12 by @ss2165
- Execute and compile circuits by @ss2165
- Only use typing origin when appropriate by @ss2165
- Allow specifying const value as input wire by @ss2165
- Used `classmethod` trick for conversion functions of types and nodes. by @zrho
- Allow string dict key for graph by @ss2165
- Format by @ss2165
- Stop holding Node in NodeRef by @ss2165
- Allow not specifying port for single output nodes by @ss2165
- Make add_node function, name position-only by @ss2165
- Just use dictionaries for signature by @ss2165
- Simpler interface; don't require signature in graph by @ss2165
- RuntimeClient class for all requests by @ss2165
- Load signature as nested modules by @ss2165
- Merge branch 'feature/py_graph' into feature/python-type-hints by @ss2165
- Use string map for signature entries by @ss2165
- [experimental] allow python type defs without structs by @ss2165
- [experimental] allow python type defs without structs by @ss2165
- New pytket worker by @ss2165
- Client can request the signature from the server. by @zrho
- better labels in visualisation by @ss2165
- Get frontend tests passing by @ss2165
- Start moving frontend tests to new graph by @ss2165
- Automate node naming somewhat by @ss2165
- Kill get_type by @ss2165
- Fold graph.py into tierkreis_graph by @ss2165
- Value.get_type method by @ss2165
- Poy3.8 friendly by @ss2165
- TierkreisGraph class by @ss2165
- Delete redundant requirements.txt by @ss2165
- Mypy configuration. by @zrho
- Split out `tierkreis.core.graph`. by @zrho
- Initial idea of fleshed-out python API. by @zrho
- Set up CI by @ss2165
- Let workers communicate signatures. by @zrho
- Compress idpy test by @ss2165
- Struct value type by @ss2165
- Fixed typo by @zrho
- Adjust python side to new graph representation. by @zrho
- Python compatibility with adjusted protobuf `Type`. by @zrho
- Use betterproto with frontend by @ss2165
- Unpack args in decorator by @ss2165
- Make python linting happy by @ss2165
- Renamed `module`s to `namespace`s for worker processes. by @zrho
- Readded tests for the python worker. by @zrho
- Some more cleanups. by @zrho
- Better error handling in python worker. by @zrho
- Restructured the python modules. by @zrho

### Fixed
- Fix dependency comment by @ss2165
- Fix bug where box output_order not used by @ss2165
- Fix cli runtime default docstring by @ss2165
- Fixup! move test worker to python/tests/ by @ss2165
- Fix type issues by @ss2165
- Fix some versions to work on local machine by @ss2165
- Fix CI workflow by @ss2165
- Fix bugs with graph deserialization by @ss2165
- Fixup! poy3.8 friendly by @ss2165
- Fixup! TierkreisGraph class by @ss2165
- Fixup! TierkreisGraph class by @ss2165
- Fixup! poy3.8 friendly by @ss2165

### Removed
- Remove http server by @alexarice
- Remove incremental type checking by @ss2165
- Remove obsolete TKSL stuff, comments, docs by @acl-cqc
- Remove tksl by @alexarice
- Remove pylint warnings by @ss2165
- Remove special-case for bool by @acl-cqc
- Remove duplicates; add runtime to test by @acl-cqc
- Remove Option, including from TKSL by @acl-cqc
- Remove _singleton by @acl-cqc
- Remove an unused import and unused parameter by @ss2165
- Remove TierkreisGraph.copy_value (keeping test_commontypes.py, oops) by @acl-cqc
- [Refactor] insert_graph: remove node_refs dict by @acl-cqc
- Remove semicolon from 'if' syntax when no inputs by @acl-cqc
- Remove (and error on) unused "type: ignore" by @acl-cqc
- Remove unused bell_circuit pytest fixture by @acl-cqc
- Cli improvements: delete, help text, extra options by @ss2165
- Remove unit type and add Some(T), None syntax for Option by @ss2165
- Remove circuit basic type by @ss2165
- Remove array type and add Vec type by @ss2165
- Removed actix dependency from runtime. by @zrho
- Remove port constraints on construction by @ss2165
- Remove inputs() and outputs() from TierkreisNode by @ss2165
- Remove redundant python files by @ss2165
- Remove unused imports by @ss2165
- Removed generated protobuf files. by @zrho

### New Contributors
* @ss2165 made their first contribution in [#2](https://github.com/Quantinuum/tierkreis/pull/2)
* @alexarice made their first contribution
* @acl-cqc made their first contribution
* @zrho made their first contribution

[unreleased]: https://github.com/Quantinuum/tierkreis/compare/v2.1.1...HEAD
[2.1.1]: https://github.com/Quantinuum/tierkreis/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/Quantinuum/tierkreis/compare/v2.0.12-post.2...v2.1.0
[2.0.12-post.2]: https://github.com/Quantinuum/tierkreis/compare/v2.0.12-post.1...v2.0.12-post.2
[2.0.12-post.1]: https://github.com/Quantinuum/tierkreis/compare/v2.0.10_viz_0.1.8...v2.0.12-post.1
[2.0.10_viz_0.1.8]: https://github.com/Quantinuum/tierkreis/compare/v2.0.11...v2.0.10_viz_0.1.8
[2.0.11]: https://github.com/Quantinuum/tierkreis/compare/v2.0.10...v2.0.11
[2.0.10]: https://github.com/Quantinuum/tierkreis/compare/v2.0.9...v2.0.10
[2.0.9]: https://github.com/Quantinuum/tierkreis/compare/v2.0.8...v2.0.9
[2.0.8]: https://github.com/Quantinuum/tierkreis/compare/v2.0.7...v2.0.8
[2.0.7]: https://github.com/Quantinuum/tierkreis/compare/v2.0.6...v2.0.7
[2.0.6]: https://github.com/Quantinuum/tierkreis/compare/v2.0.5...v2.0.6
[2.0.5]: https://github.com/Quantinuum/tierkreis/compare/v2.0.4_viz_0.1.7...v2.0.5
[2.0.4_viz_0.1.7]: https://github.com/Quantinuum/tierkreis/compare/v2.0.4...v2.0.4_viz_0.1.7
[2.0.4]: https://github.com/Quantinuum/tierkreis/compare/v2.0.3_1...v2.0.4
[2.0.3_1]: https://github.com/Quantinuum/tierkreis/compare/v2.0.3...v2.0.3_1
[2.0.3]: https://github.com/Quantinuum/tierkreis/compare/v2.0.2...v2.0.3
[2.0.2]: https://github.com/Quantinuum/tierkreis/compare/v2.0.1...v2.0.2
[2.0.1]: https://github.com/Quantinuum/tierkreis/compare/v2.0.0_viz_0.1.1...v2.0.1
[2.0.0_viz_0.1.1]: https://github.com/Quantinuum/tierkreis/compare/v2.0.0...v2.0.0_viz_0.1.1
[2.0.0]: https://github.com/Quantinuum/tierkreis/compare/v0.7.3...v2.0.0
[0.7.3]: https://github.com/Quantinuum/tierkreis/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/Quantinuum/tierkreis/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/Quantinuum/tierkreis/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/Quantinuum/tierkreis/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/Quantinuum/tierkreis/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/Quantinuum/tierkreis/compare/v0.5.3...v0.6.0
[0.5.3]: https://github.com/Quantinuum/tierkreis/compare/v0.5.0...v0.5.3
[0.5.0]: https://github.com/Quantinuum/tierkreis/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Quantinuum/tierkreis/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Quantinuum/tierkreis/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Quantinuum/tierkreis/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Quantinuum/tierkreis/compare/v0.1.0...v0.2.0

<!-- generated by git-cliff -->
