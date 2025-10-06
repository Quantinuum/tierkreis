import pytest

from tierkreis.controller.data.location import Loc, NodeStep
from tierkreis.exceptions import TierkreisError

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
    ["node_location", "node_step", "loc_str"],
    [
        (node_location_1, ("N", 1), "-.L0.N3.L2.N0.M7.N10"),
        (node_location_2, ("N", 0), "-.L0.N3.N8.N0"),
        (node_location_3, ("N", 0), "-"),
        (node_location_4, "-", ""),
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
        (node_location_1, ("N", 10), "-.N1.L0.N3.L2.N0.M7"),
        (node_location_2, ("N", 0), "-.N0.L0.N3.N8"),
        (node_location_3, ("N", 0), "-"),
        (node_location_4, "-", ""),
    ],
)
def test_pop_last(node_location: Loc, node_step: NodeStep, loc_str: str) -> None:
    pop = node_location.pop_last()
    (step, remainder) = pop
    assert step == node_step
    assert remainder == Loc(loc_str)


def test_pop_empty() -> None:
    loc = Loc("")
    with pytest.raises(TierkreisError):
        loc.pop_first()
    with pytest.raises(TierkreisError):
        loc.pop_last()


def test_pop_first_multiple() -> None:
    loc = node_location_2
    pop = loc.pop_first()
    (step, remainder) = pop
    assert step == ("N", 0)
    assert remainder == Loc("-.L0.N3.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == ("L", 0)
    assert remainder == Loc("-.N3.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == ("N", 3)
    assert remainder == Loc("-.N8.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == ("N", 8)
    assert remainder == Loc("-.N0")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == ("N", 0)
    assert remainder == Loc("-")
    pop = remainder.pop_first()
    (step, remainder) = pop
    assert step == "-"
    assert remainder == Loc("")


def test_pop_last_multiple() -> None:
    loc = node_location_2
    pop = loc.pop_last()
    (step, remainder) = pop
    assert step == ("N", 0)
    assert remainder == Loc("-.N0.L0.N3.N8")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == ("N", 8)
    assert remainder == Loc("-.N0.L0.N3")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == ("N", 3)
    assert remainder == Loc("-.N0.L0")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == ("L", 0)
    assert remainder == Loc("-.N0")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == ("N", 0)
    assert remainder == Loc("-")
    pop = remainder.pop_last()
    (step, remainder) = pop
    assert step == "-"
    assert remainder == Loc("")


@pytest.mark.parametrize(
    ["node_location", "index"],
    [
        (node_location_1, 10),
        (node_location_2, 0),
        (node_location_3, 0),
        (node_location_4, 0),
        (Loc().N(-1), -1),
    ],
)
def test_get_last_index(node_location: Loc, index: int) -> None:
    assert node_location.peek_index() == index
