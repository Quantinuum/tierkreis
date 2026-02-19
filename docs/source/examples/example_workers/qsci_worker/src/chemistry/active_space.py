def get_frozen(
    mo_occ: list[int],
    n_cas: int,
    n_elecas: int,
) -> list[int]:
    frozen = []
    n_orbs = len(mo_occ)
    n_core = get_n_core(mo_occ, n_elecas)
    if n_core + n_cas > n_orbs:
        raise ValueError("active space is larger than basis set")
    for i in range(n_orbs):
        # print(i, i < n_core, i >= n_core + n_cas)
        if i < n_core or i >= n_core + n_cas:
            frozen.append(i)
    return frozen


def get_n_core(
    mo_occ: list[int],
    n_elecas: int,
) -> int:
    n_elec = int(sum(mo_occ))
    n_core = (n_elec - n_elecas) // 2
    return n_core


def get_n_active(
    mo_occ: list[int],
    n_cas: int,
    n_elecas: int,
) -> int:
    n_frozen = len(get_frozen(mo_occ, n_cas, n_elecas))
    n_active = len(mo_occ) - n_frozen
    return n_active


def get_n_virtual(
    mo_occ: list[int],
    n_cas: int,
    n_elecas: int,
) -> int:
    n_frozen = len(get_frozen(mo_occ, n_cas, n_elecas))
    n_core = get_n_core(mo_occ, n_elecas)
    n_virtual = n_frozen - n_core
    return n_virtual
