from typing import Optional, cast

import numpy as np
from numpy.typing import NDArray
from pyscf import ao2mo, gto, scf


def _extract_hamiltonian_rhf(
    mol: gto.Mole, frozen: Optional[list[int]] = None
) -> tuple[float, np.ndarray, np.ndarray]:
    """Extract the fermionic Hamiltonian from a mean-field calculation.

    Args:
        mol: PySCF molecule object.
        hf: Mean-field calculation class.
        frozen: Orbital indices to freeze.

    Returns:
        Constant term, one-body Hamiltonian, and two-body Hamiltonian.
    """
    # Run the mean-field calculation and convert it to GHF
    mf = scf.RHF(mol)
    mf.kernel()

    # Get the MOs
    mo = cast(NDArray, mf.mo_coeff)
    if frozen:
        mo = np.delete(mo, frozen, axis=1)
    nmo = mo.shape[1]

    # Get the constant term
    h0 = mf.energy_nuc()

    # Get the one-body Hamiltonian
    h1e = mo.T.conj() @ mf.get_hcore() @ mo

    if frozen:
        assert mf.mo_occ is not None, "molecule mo_occ is None"
        occ_froz = np.where([i in frozen for i in range(mf.mo_occ.size)], mf.mo_occ, 0)
        dm_froz = mf.make_rdm1(mf.mo_coeff, occ_froz)
        h0 += mf.energy_elec(dm=dm_froz)[0]

        veff = mf.get_veff(dm=dm_froz)
        h1e += mo.T.conj() @ veff @ mo

    # Get the two-body Hamiltonian
    h2e = ao2mo.kernel(mol, mo)
    h2e = ao2mo.restore(1, h2e, nmo)

    return h0, h1e, h2e


def extract_hamiltonian_rhf(
    atom: list[tuple[str, list[float]]],
    basis: str,
    charge: int = 0,
    spin: int = 0,
    frozen: Optional[list[int]] = None,
) -> tuple[float, np.ndarray, np.ndarray]:
    """Generate the Hamiltonian in a qubit representation.

    Args:
        atom: String of atoms and coordinates. Can also be the path to a `.xyz` file.
        basis: Name of the basis-set.
        charge: Molecular charge.
        spin: Spin multiplicity in the form of 2S (i.e. `nalpha - nbeta`).
        restricted: Whether the Hartree--Fock calculation should have restricted symmetry. If
            `False`, unrestricted symmetry is assumed.
        frozen: Orbital indices to freeze.
        which: The code to use to generate the qubit Hamiltonian. Can be one of
            `{'pyscf', 'ntchem'}`.

    Returns:
        Hamiltonian in a qubit representation, represented as a dictionary with `QubitPauliString`
        keys and `float` values.
    """
    # TODO: This docstring is wrong now
    mol = gto.M(
        atom=atom,
        basis=basis,
        charge=charge,
        spin=spin,
        verbose=0,
    )

    return _extract_hamiltonian_rhf(mol, frozen=frozen)
