"""hpr-sim, a flight simulator for hobby and high-power rockets.

Four types make a flight, in the order a program needs them: an ``Environment``, a ``Motor``, a
``Rocket`` and a ``Flight``. Values are in SI units, named in every argument and attribute. The
guide's Python page walks through them: https://nrdptel.github.io/hpr-sim/python.html
"""

from hpr._hpr import (
    Environment,
    Flight,
    HprError,
    Motor,
    Rocket,
    __version__,
    materials,
)

__all__ = [
    "Environment",
    "Flight",
    "HprError",
    "Motor",
    "Rocket",
    "__version__",
    "materials",
]
