# How to run the slurm test

For setup see the [README](README.md).
In short:

- install docker
- From `infra/slurm_local`
  - make the local sbatch script executable `chmod +x sbatch squeue`
  - build the containers with `docker compose build`
  - ensure the containers are running `docker compose up -d`
  - This will mount `~/.tierkreis` inside the containers on `/root/tierkreis/` and the tierkreis directory to `/tierkreis`

To run the test we emulate `sbatch`.
The devenv contains a script for this (same as the sbatch script), if you want, link it to a location in your path to make it available elsewhere.

Run the test:

```
pytest tierkreis/tests/executor/test_hpc_executor.py
```

**Caveats**:

- mpi workers is nondeterministic due to allocation from slurm
