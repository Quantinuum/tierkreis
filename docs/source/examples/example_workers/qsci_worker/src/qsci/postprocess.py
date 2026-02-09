from copy import deepcopy

import numpy as np
from scipy.sparse import csr_array


def get_phase(phi_i, phi_j) -> int:
    """phase factor.

    Note:
        See section 18.8 of the pink book.
    """
    diff = [(j - i) for i, j in zip(phi_i, phi_j)]
    phase = 1
    for i, p in enumerate(diff):
        if p == -1:
            phase *= (-1) ** sum(phi_i[i + 1 :])
        if p == 1:
            phase *= (-1) ** sum(phi_j[i + 1 :])
    return phase


def get_h1(h1: np.ndarray, index: tuple[int, ...]):
    if (index[0] % 2) == (index[1] % 2):
        i = tuple(i // 2 for i in index)
        val = h1[i]
    else:
        val = 0.0
    return val


def get_h2(h2: np.ndarray, index: tuple[int, ...]):
    if (index[0] % 2 == index[1] % 2) and (index[2] % 2 == index[3] % 2):
        i = tuple(i // 2 for i in index)
        val = h2[i]
    else:
        val = 0.0
    return val


def eval_hele(
    phi_i: tuple[int, ...],
    phi_j: tuple[int, ...],
    h1: np.ndarray,
    h2: np.ndarray,
    enuc: float,
) -> float:
    """Get the matrix element <I|H|J> based on the Slater-Condon rule."""
    # Return if the particle number is not conserving.
    if sum(phi_i) != sum(phi_j):
        val = 0.0
        return val
    # Identify the excitation type.
    config_diff = np.array([(j - i) for i, j in zip(phi_i, phi_j)])
    n_excitation = np.sum(np.abs(config_diff)) // 2
    # Triple or higher excitation returns zero.
    if n_excitation > 2:
        val = 0.0
    # Double excitation.
    elif n_excitation == 2:
        occ = np.where(config_diff == -1)[0]
        vir = np.where(config_diff == 1)[0]
        j_index = (occ[0], vir[0], occ[1], vir[1])
        k_index = (occ[0], vir[1], occ[1], vir[0])
        val = get_h2(h2, j_index) - get_h2(h2, k_index)
        val *= get_phase(phi_i, phi_j)
    # Single excitation.
    elif n_excitation == 1:
        occ = np.where(config_diff == -1)[0][0]
        vir = np.where(config_diff == 1)[0][0]
        fzc = [i for i, j in enumerate(phi_i) if j == 1 and i != occ]
        h_index = (occ, vir)
        val = get_h1(h1, h_index)
        for i in fzc:
            j_index = (occ, vir, i, i)
            k_index = (occ, i, i, vir)
            val += get_h2(h2, j_index) - get_h2(h2, k_index)
        val *= get_phase(phi_i, phi_j)
    # Diagonal elements.
    elif n_excitation == 0:
        occ = [i for i, j in enumerate(phi_i) if j == 1]
        val = 0.0
        for i in occ:
            val += get_h1(h1, (i, i))
        for ni, i in enumerate(occ):
            for j in occ[:ni]:
                j_index = (i, i, j, j)
                k_index = (i, j, j, i)
                val += get_h2(h2, j_index) - get_h2(h2, k_index)
        val += enuc
    else:
        raise RuntimeError()
    return val


def get_ci_matrix(
    phis: list[tuple[int, ...]],
    h1: np.ndarray,
    h2: np.ndarray,
    enuc: float,
    atol: float = 1e-10,
) -> csr_array:
    """Get the CI matrix for the given SD bais."""
    raw: list[int] = []
    col: list[int] = []
    data: list[float] = []
    for i, phi_i in enumerate(phis):
        for j in range(i + 1):
            phi_j = phis[j]
            val = eval_hele(phi_i, phi_j, h1, h2, enuc)
            if abs(val) > atol:
                raw.append(i)
                col.append(j)
                data.append(val)
                if i != j:
                    raw.append(j)
                    col.append(i)
                    data.append(val)
    hij = csr_array((data, (raw, col)))
    return hij


def postprocess_configs(
    reference: tuple[int, ...],
    configs: list[tuple[int, ...]],
) -> list[tuple[int, ...]]:
    """Post select the configurations."""
    nea = sum(reference[0::2])
    neb = sum(reference[1::2])
    # new_configs: list[tuple[int, ...]] = []
    new_configs: set[tuple[int, ...]] = set([])
    for config in configs:
        occ = [i + j for i, j in zip(config[0::2], config[1::2])]
        ls: list[list[int]] = [[]]
        # print("occ", occ)
        for on in occ:
            if on == 2:
                for lsx in ls:
                    lsx += [1, 1]
            elif on == 1:
                tmp = deepcopy(ls)
                for lsx in ls:
                    lsx += [1, 0]
                for lsx in tmp:
                    lsx += [0, 1]
                ls += tmp
            elif on == 0:
                for lsx in ls:
                    lsx += [0, 0]
        # print("orig:", config)
        for lsx in ls:
            if sum(lsx[0::2]) == nea and sum(lsx[1::2]) == neb:
                # print("new: ", lsx)
                new_configs.add(tuple(lsx))
        # if sum(config[0::2]) == nea and sum(config[1::2]) == neb:
        #     new_config = [0 for _ in config]
        #     new_config[0::2] = config[1::2]
        #     new_config[1::2] = config[0::2]
        #     new_configs.add(config)
        #     new_configs.add(tuple(new_config))
    return list(new_configs)
