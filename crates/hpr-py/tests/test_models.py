"""A drag and a wind written as Python functions, flown by the library, and their exceptions."""

import gc
import math
import threading
import weakref

import numpy as np
import pytest

import hpr
from test_drag_table import SITE, rocket


def flight(site=SITE, **options) -> hpr.Flight:
    return hpr.Flight(rocket(), site, 1.8, interval_s=0.05, **options)


def test_a_constant_python_drag_flies_as_a_table_of_it():
    table = flight(drag_table=hpr.DragTable([(0.0, 0.5), (3.0, 0.5)]))
    function = flight(drag=lambda mach, thrusting: 0.5)
    assert function.columns == table.columns
    for name in table.columns:
        assert np.array_equal(function[name], table[name], equal_nan=True), name
    assert function.apogee_m == table.apogee_m
    # And the drag is the function's, not hpr's own.
    assert function.apogee_m != flight().apogee_m


@pytest.mark.parametrize("cd0", [0.3, 0.45])
def test_other_constants_fly_as_a_table_to_its_rounding(cd0):
    # A table's linear interpolation, (1 - t) y0 + t y1, can round a constant row to a
    # neighbouring float, so a table of 0.3 or 0.45 is not always exactly 0.3 or 0.45: the two
    # flights agree to that rounding, not to the bit as they do at 0.5.
    table = flight(drag_table=hpr.DragTable([(0.0, cd0), (3.0, cd0)]))
    function = flight(drag=lambda mach, thrusting: cd0)
    assert function.apogee_m == pytest.approx(table.apogee_m, rel=1e-10)


def test_a_drag_function_is_asked_the_mach_number_and_whether_a_motor_burns():
    asked = []

    def drag(mach, thrusting):
        asked.append((mach, thrusting))
        return 0.45 if thrusting else 0.55

    flown = flight(drag=drag)
    assert all(isinstance(mach, float) and mach >= 0.0 for mach, _ in asked)
    assert {thrusting for _, thrusting in asked} == {True, False}
    assert 0.3 < max(mach for mach, _ in asked) < 1.2 * flown.max_mach
    # Its power-on and power-off numbers fly as a table with both curves, to the table's
    # rounding of 0.45 (above).
    table = flight(
        drag_table=hpr.DragTable([(0.0, 0.55), (3.0, 0.55)], [(0.0, 0.45), (3.0, 0.45)])
    )
    assert flown.apogee_m == pytest.approx(table.apogee_m, rel=1e-10)


def test_a_python_wind_flies_as_the_same_constant_wind():
    speed_m_s, from_deg = 6.0, 250.0
    east = -speed_m_s * math.sin(math.radians(from_deg))
    north = -speed_m_s * math.cos(math.radians(from_deg))
    constant = hpr.Environment(32.99, -106.97, 1400.0, wind_speed_m_s=speed_m_s, wind_from_deg=from_deg)
    function = hpr.Environment(32.99, -106.97, 1400.0, wind=lambda height_m: (east, north))
    a, b = flight(constant), flight(function)
    assert b.apogee_m == pytest.approx(a.apogee_m, rel=1e-12)
    for name, x in a.landing.items():
        assert b.landing[name] == pytest.approx(x, rel=1e-9, abs=1e-9), name
    # Any sequence of two numbers will do: a list, a NumPy array.
    listed = hpr.Environment(32.99, -106.97, 1400.0, wind=lambda height_m: [east, north])
    arrayed = hpr.Environment(32.99, -106.97, 1400.0, wind=lambda height_m: np.array([east, north]))
    assert flight(listed).apogee_m == b.apogee_m == flight(arrayed).apogee_m


def test_a_wind_function_is_asked_heights_above_sea_level():
    heights = []

    def wind(height_m):
        heights.append(height_m)
        # Stronger with height: 2 m/s at the ground, 8 m/s 1000 m up, from the west.
        return (2.0 + 0.006 * max(height_m - 1400.0, 0.0), 0.0)

    windy = flight(hpr.Environment(32.99, -106.97, 1400.0, wind=wind))
    # First on the rail, at the launch site's 1400 m; the landing's search dips a few metres
    # below the ground.
    assert heights[0] == pytest.approx(1400.0, abs=1.0)
    assert min(heights) > 1400.0 - 10.0
    assert max(heights) == pytest.approx(1400.0 + windy.apogee_m, abs=50.0)
    # Blown east, as a west wind blows.
    assert windy.landing["east_m"] > 10.0


def test_exceptions_reach_python_as_they_were_raised():
    class Refused(Exception):
        pass

    def drag(mach, thrusting):
        if mach > 0.2:
            raise Refused(f"no data at Mach {mach}")
        return 0.5

    with pytest.raises(Refused, match="no data at Mach"):
        flight(drag=drag)

    def wind(height_m):
        if height_m > 1500.0:
            raise KeyError("above the sounding")
        return (0.0, 0.0)

    site = hpr.Environment(32.99, -106.97, 1400.0, wind=wind)
    with pytest.raises(KeyError, match="above the sounding") as raised:
        flight(site)
    # The traceback reaches into the function.
    assert raised.traceback[-1].name == "wind"
    # A wrong type is Python's TypeError, a number hpr refuses is hpr's error.
    with pytest.raises(TypeError):
        flight(drag=lambda mach, thrusting: "0.5")
    with pytest.raises(TypeError):
        flight(hpr.Environment(0.0, 0.0, 0.0, wind=lambda height_m: 3.0))
    with pytest.raises(hpr.HprError, match="drag"):
        flight(drag=lambda mach, thrusting: -0.1)
    with pytest.raises(hpr.HprError, match="east"):
        flight(hpr.Environment(0.0, 0.0, 0.0, wind=lambda height_m: (math.nan, 0.0)))


def test_an_exception_stops_the_flight_even_where_a_step_would_be_retried():
    # The integrator retries a step whose evaluation failed with a shorter one. An exception
    # raised once, at any call, still stops the flight and is raised.
    for once_at in (1, 5, 40, 200):
        calls = []

        def drag(mach, thrusting, once_at=once_at, calls=calls):
            calls.append(mach)
            if len(calls) == once_at:
                raise ArithmeticError(f"call {once_at}")
            return 0.5

        with pytest.raises(ArithmeticError, match=f"call {once_at}"):
            flight(drag=drag)
        # Not called again once it has raised.
        assert len(calls) == once_at
    # Ctrl-C too.
    with pytest.raises(KeyboardInterrupt):
        flight(drag=lambda mach, thrusting: (_ for _ in ()).throw(KeyboardInterrupt()))


def test_the_first_exception_is_raised_the_drags_or_the_winds():
    def wind(height_m):
        raise KeyError("wind")

    def drag(mach, thrusting):
        raise ValueError("drag")

    # The wind is asked first, on the rail.
    with pytest.raises(KeyError):
        flight(hpr.Environment(0.0, 0.0, 0.0, wind=wind), drag=drag)


def test_an_environment_flies_again_and_in_threads_each_flight_raising_its_own():
    raising = {"on": True}

    def wind(height_m):
        if raising["on"]:
            raise KeyError("off the table")
        return (1.0, 0.0)

    site = hpr.Environment(0.0, 0.0, 0.0, wind=wind)
    with pytest.raises(KeyError):
        flight(site)
    raising["on"] = False
    assert flight(site).apogee_m > 0.0

    # Each thread's drag raises its own exception; the windy environment is shared.
    raised = {}

    def fly(name):
        def drag(mach, thrusting):
            if mach > 0.1:
                raise LookupError(name)
            return 0.5

        try:
            flight(site, drag=drag)
        except LookupError as error:
            raised[name] = str(error)

    threads = [threading.Thread(target=fly, args=(f"t{i}",)) for i in range(8)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert raised == {f"t{i}": f"t{i}" for i in range(8)}


def test_a_function_holding_its_environment_is_collected():
    class Site:
        def wind(self, height_m):
            return (0.0, 0.0)

    site = Site()
    site.environment = hpr.Environment(0.0, 0.0, 0.0, wind=site.wind)
    gone = weakref.ref(site)
    del site
    gc.collect()
    assert gone() is None


def test_a_function_and_its_constant_counterpart_are_not_both_taken():
    with pytest.raises(hpr.HprError, match="give one"):
        flight(drag=lambda mach, thrusting: 0.5, drag_table=hpr.DragTable([(0.0, 0.5), (1.0, 0.5)]))
    with pytest.raises(hpr.HprError, match="give one"):
        hpr.Environment(0.0, 0.0, 0.0, wind_speed_m_s=3.0, wind=lambda height_m: (0.0, 0.0))
    with pytest.raises(hpr.HprError, match="give one"):
        hpr.Environment(0.0, 0.0, 0.0, wind_from_deg=90.0, wind=lambda height_m: (0.0, 0.0))
    # Something that isn't a function is refused as it is given.
    with pytest.raises(hpr.HprError, match="not callable"):
        flight(drag=0.5)
    with pytest.raises(hpr.HprError, match="not callable"):
        hpr.Environment(0.0, 0.0, 0.0, wind=(3.0, 0.0))
