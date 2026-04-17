from tests.executor.samples import pbs_spec, slurm_spec
from tierkreis.controller.executor.hpc.psij_conversion import psij_to_spec, spec_to_psij


def test_psij_conversion_slurm() -> None:
    spec = slurm_spec()
    spec.extra_scheduler_args = {}
    psij_spec = spec_to_psij(spec, target_scheduler="slurm")
    converted_back_spec = psij_to_spec(psij_spec)

    assert spec == converted_back_spec


def test_psij_conversion_pbs() -> None:
    spec = pbs_spec()
    spec.extra_scheduler_args = {}
    psij_spec = spec_to_psij(spec, target_scheduler="pbs")
    converted_back_spec = psij_to_spec(psij_spec)

    assert spec == converted_back_spec
