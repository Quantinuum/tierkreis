# How to run the slurm test

For setup see the [README](README.md).
In short:

- install docker
- From `infra/slurm_local`
  - make the local sbatch script executable `chmod +x sbatch squeue`
  - build the containers with `docker compose build`
  - ensure the containers are running `docker compose up -d`
  - This mounts `~/.tierkreis` inside the containers and the tierkreis directory at `/tierkreis`
  - To emulate the root node you can `docker exec -it slurmctld bash`

To run the test we emulate `sbatch` and `squeue`.
The devenv contains a script for this (same as the sbatch script), if you want, link it to a location in your path to make it available elsewhere.

By default the test is disabled with `pytest.mark.skip()` uncomment before testing.
Run the test:

```
pytest tierkreis/tests/controller/test_runtime_profiles.py -k hpc_docker_worker
```

**Caveats**:

- mpi workers is nondeterministic due to allocation from slurm
- mpi is run as root and therefore needs additional variables
