def apply_permutation_in_place(data, perm):
    """
    Reorders `data` in place according to permutation `perm`.
    NOTE: `perm` will be modified (destroyed).
    """
    if len(data) != len(perm):
        raise ValueError("data and perm must have the same length")

    n = len(data)

    for i in range(n):
        current = i

        # While element not in correct position
        while perm[current] != current:
            next_index = perm[current]

            # swap elements
            data[current], data[next_index] = data[next_index], data[current]
            perm[current], perm[next_index] = perm[next_index], perm[current]


def invert_permutation(perm):
    n = len(perm)
    inv = [0] * n
    for i, p in enumerate(perm):
        inv[p] = i
    return inv

data = ["a", "b", "c"]
perm = [2, 0, 1]  # reverse

print(data)

iperm = invert_permutation(perm)
apply_permutation_in_place(data, iperm)

print(data)