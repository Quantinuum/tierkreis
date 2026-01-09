import pytest

from tierkreis_core import NodeStep, Loc

node_location_1 = Loc()
node_location_1 = node_location_1.N(1)
node_location_1 = node_location_1.L(0)
node_location_1 = node_location_1.N(3)
node_location_1 = node_location_1.L(2)
node_location_1 = node_location_1.N(0)
node_location_1 = node_location_1.M(7)
node_location_1 = node_location_1.N(10)


node_location_2 = Loc()
node_location_2 = node_location_2.N(0)
node_location_2 = node_location_2.L(0)
node_location_2 = node_location_2.N(3)
node_location_2 = node_location_2.N(8)
node_location_2 = node_location_2.N(0)

node_location_3 = Loc()
node_location_3 = node_location_3.N(0)

node_location_4 = Loc()


@pytest.mark.parametrize(
    ["node_location", "loc_str"],
    [
        (node_location_1, "-.N1.L0.N3.L2.N0.M7.N10"),
        (node_location_2, "-.N0.L0.N3.N8.N0"),
        (node_location_3, "-.N0"),
        (node_location_4, "-"),
    ],
)
def test_to_from_str(node_location: Loc, loc_str: str):
    node_location_str = str(node_location)
    assert node_location_str == loc_str

    new_loc = Loc(node_location_str)
    assert new_loc == node_location


@pytest.mark.parametrize(
    ["node_location", "loc_str"],
    [
        (node_location_1, "-.N1.L0.N3.L2.N0.M7"),
        (node_location_2, "-.N0.L0.N3.N8"),
        (node_location_3, "-"),
        (node_location_4, ""),
        (Loc().N(1).L(3), "-.N1.L2"),
        (Loc().N(1).L(2), "-.N1.L1"),
        (Loc().N(1).L(0), "-.N1"),
    ],
)
def test_parent(node_location: Loc, loc_str: str) -> None:
    parent = node_location.parent()
    assert parent == Loc(loc_str)


@pytest.mark.parametrize(
    ["node_location", "node_step", "loc_str"],
    [
        (node_location_1, NodeStep("N1"), "-.L0.N3.L2.N0.M7.N10"),
        (node_location_2, NodeStep("N0"), "-.L0.N3.N8.N0"),
        (node_location_3, NodeStep("N0"), "-"),
        (node_location_4, NodeStep("-"), ""),
    ],
)
def test_pop_first(node_location: Loc, node_step: NodeStep, loc_str: str) -> None:
    pop = node_location.pop_first()
    (step, remainder) = pop
    assert step == node_step
    assert remainder == Loc(loc_str)


@pytest.mark.parametrize(
    ["node_location", "node_step", "loc_str"],
    [
        (node_location_1, NodeStep("N10"), "-.N1.L0.N3.L2.N0.M7"),
        (node_location_2, NodeStep("N0"), "-.N0.L0.N3.N8"),
        (node_location_3, NodeStep("N0"), "-"),
        (node_location_4, NodeStep("-"), ""),
    ],
)
def test_pop_last(node_location: Loc, node_step: NodeStep, loc_str: str) -> None:
    pop = node_location.pop_last()
    (step, remainder) = pop
    assert step == node_step
    assert remainder == Loc(loc_str)


def test_pop_empty() -> None:
    loc = Loc("")
    with pytest.raises(ValueError):
        loc.pop_first()
    with pytest.raises(ValueError):
        loc.pop_last()


def test_pop_first_multiple() -> None:
    loc = node_location_2
    pop = loc.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("N0")
    assert remainder == Loc("-.L0.N3.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("L0")
    assert remainder == Loc("-.N3.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("N3")
    assert remainder == Loc("-.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("N8")
    assert remainder == Loc("-.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("N0")
    assert remainder == Loc("-")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == NodeStep("-")
    assert remainder == Loc("")


def test_pop_last_multiple() -> None:
    loc = node_location_2
    pop = loc.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("N0")
    assert remainder == Loc("-.N0.L0.N3.N8")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("N8")
    assert remainder == Loc("-.N0.L0.N3")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("N3")
    assert remainder == Loc("-.N0.L0")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("L0")
    assert remainder == Loc("-.N0")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("N0")
    assert remainder == Loc("-")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == NodeStep("-")
    assert remainder == Loc("")


@pytest.mark.parametrize(
    ["node_location", "expectation"],
    [
        (node_location_1, False),
        (node_location_2, False),
        (node_location_3, False),
        (node_location_4, False),
        (Loc().exterior(), True),
    ],
)
def test_last_step_exterior(node_location: Loc, expectation: bool) -> None:
    assert node_location.last_step_exterior() == expectation


@pytest.mark.parametrize(
    ["node_location", "index"],
    [
        (node_location_1, None),
        (node_location_2, None),
        (node_location_3, None),
        (node_location_4, None),
        (Loc().exterior(), None),
        (Loc().L(1), 1),
        (Loc().L(4), 4),
    ],
)
def test_get_last_index(node_location: Loc, index: int | None) -> None:
    assert node_location.peek_index() == index


@pytest.mark.parametrize(
    ["node_location", "expected"],
    [
        (
            node_location_1,
            [
                Loc(),
                Loc().N(1),
                Loc().N(1).L(0),
                Loc().N(1).L(0).N(3),
                Loc().N(1).L(0).N(3).L(2),
                Loc().N(1).L(0).N(3).L(2).N(0),
                Loc().N(1).L(0).N(3).L(2).N(0).M(7),
                Loc().N(1).L(0).N(3).L(2).N(0).M(7).N(10),
            ],
        ),
        (
            node_location_2,
            [
                Loc(),
                Loc().N(0),
                Loc().N(0).L(0),
                Loc().N(0).L(0).N(3),
                Loc().N(0).L(0).N(3).N(8),
                Loc().N(0).L(0).N(3).N(8).N(0),
            ],
        ),
        (node_location_3, [Loc(), Loc().N(0)]),
        (node_location_4, [Loc()]),
    ],
)
def test_partial_paths(node_location: Loc, expected: list[Loc]):
    assert expected == node_location.partial_locs()
