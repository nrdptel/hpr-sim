"""Two upstream corrections to RocketPy 1.13.0's equations of motion, which the whole-flight oracle
flies (M2.1d3, ADR-026, issue #50).

RocketPy's default 6-DOF equations, `Flight.u_dot_generalized`, take the centre of dry mass (CDM)
as their reference point and need the vectors *from* it to the instantaneous centre of mass
(`r_CM`) and to the nozzle exit (`r_NOZ`). They read `Rocket.com_to_cdm_function` and
`Rocket.nozzle_to_cdm`, which point the other way, *to* the CDM (`rocket.py:996-1025`, whose note
says `com_to_cdm_function + center_of_mass == center_of_dry_mass_position`, and `:959-972`). So
while the motor burns, RocketPy takes the moments about a point as far forward of the CDM as the
true centre of mass is behind it. For Juno III at its rail exit that point is 1.315 m from the nose
tip where the centre of mass is 1.639 m, so the margin it flies there, to its centre of pressure at
2.072 m, is 1.75 times its own `static_margin`. After burnout `r_CM` is zero and `r_NOZ` enters
only through the mass flow, so the error ends with the burn.

RocketPy's maintainers have both defects on record:

- Issue #1186 (open, 2026-08-25) reports the sign, and PR #1196 (open, head
  927e771e1a7faa2915d1f96ff48a70c544db13b4, 2026-09-16) corrects it with three edits to
  `u_dot_generalized`: negate `r_CM` and its derivatives, negate `r_NOZ`, and flip the sign of the
  `r_CM ^ w_dot` term in `v_dot`, which was written for the reversed vector.
- PR #1188 (merged into `develop` 2026-09-09, not yet released) corrects the nozzle gyration
  tensor's parallel-axis term from `0.25 * nozzle_to_cdm**2` to `nozzle_to_cdm**2`
  (`rocket.py:984-985`), so the jet damping about the CDM uses the whole lever.

This module applies exactly those changes and nothing else. `CorrectedFlight` recompiles
RocketPy's own `u_dot_generalized` source with #1196's three substitutions, each of which must
match exactly once, so an upgrade that moves the code stops the script instead of flying something
else. `correct_nozzle_gyration` sets the tensor as #1188's `evaluate_nozzle_gyration_tensor` does.

Checked before adopting them (`wind_response.py` reproduces both): at RocketPy's own states along
Juno III's windy flight, the corrected equations give the angular acceleration hpr's do to 1 to 3%,
and the centre-of-dry-mass acceleration to 0.01 m/s^2, once hpr's normal force is made linear like
RocketPy's; the released ones give 1.75 times hpr's at the rail exit, where the rotation rate is
still zero, so no damping term can be the cause.

RocketPy is MIT-licensed (`THIRD-PARTY-NOTICES.md`); the substituted lines are PR #1196's.
"""

import inspect
import textwrap

import rocketpy.simulation.flight as rocketpy_flight
from rocketpy import Flight
from rocketpy.mathutils.vector_matrix import Matrix

# What the fixture records about the corrections, in order.
CORRECTIONS = [
    {
        "upstream": "https://github.com/RocketPy-Team/RocketPy/pull/1188",
        "state": "merged into develop 2026-09-09, not in a release",
        "what": "the nozzle gyration tensor's parallel-axis term is nozzle_to_cdm**2, not "
                "0.25 * nozzle_to_cdm**2 (rocket.py:984-985), so the jet damping about the "
                "centre of dry mass uses the whole lever",
    },
    {
        "upstream": "https://github.com/RocketPy-Team/RocketPy/pull/1196",
        "issue": "https://github.com/RocketPy-Team/RocketPy/issues/1186",
        "state": "open, head 927e771e1a7faa2915d1f96ff48a70c544db13b4 (2026-09-16)",
        "what": "u_dot_generalized negates com_to_cdm_function and nozzle_to_cdm where it reads "
                "them and flips the sign of r_CM ^ w_dot in v_dot (flight.py:2509-2515, :2695): "
                "both attributes point to the centre of dry mass, the equations need them from "
                "it, so during the burn the moments were taken about a point as far forward of "
                "the dry centre of mass as the centre of mass is behind it",
    },
]

# PR #1196's edits to RocketPy 1.13.0's `Flight.u_dot_generalized`, as (released, corrected).
SUBSTITUTIONS = [
    (
        "        r_CM_z = self.rocket.com_to_cdm_function\n"
        "        r_CM_t = r_CM_z.get_value_opt(t)\n"
        "        r_CM = Vector([0, 0, r_CM_t])\n"
        "        r_CM_dot = Vector([0, 0, r_CM_z.differentiate_complex_step(t)])\n"
        "        r_CM_ddot = Vector([0, 0, r_CM_z.differentiate(t, order=2)])\n",
        "        com_to_cdm = self.rocket.com_to_cdm_function\n"
        "        r_CM = Vector([0, 0, -com_to_cdm.get_value_opt(t)])\n"
        "        r_CM_dot = Vector([0, 0, -com_to_cdm.differentiate_complex_step(t)])\n"
        "        r_CM_ddot = Vector([0, 0, -com_to_cdm.differentiate(t, order=2)])\n",
    ),
    (
        "        r_NOZ = Vector([0, 0, self.rocket.nozzle_to_cdm])\n",
        "        r_NOZ = Vector([0, 0, -self.rocket.nozzle_to_cdm])\n",
    ),
    (
        "        v_dot = K @ (T20 / total_mass - (r_CM ^ w_dot)) - 2 * (w_earth ^ v)\n",
        "        v_dot = K @ (T20 / total_mass + (r_CM ^ w_dot)) - 2 * (w_earth ^ v)\n",
    ),
]


def corrected_u_dot_generalized():
    """RocketPy's own `u_dot_generalized`, recompiled with PR #1196's substitutions.

    It is compiled inside a class named `Flight`, in `rocketpy.simulation.flight`'s globals, so
    its private names (`self.__post_processed_variables`) mangle as they do upstream.
    """
    source = inspect.getsource(Flight.u_dot_generalized)
    for released, corrected in SUBSTITUTIONS:
        found = source.count(released)
        if found != 1:
            raise SystemExit(
                f"corrections.py: RocketPy's u_dot_generalized holds {found} copies of "
                f"{released!r}, not one; it has changed, so PR #1196's edits no longer apply as "
                "written"
            )
        source = source.replace(released, corrected)
    wrapped = "class Flight:\n" + textwrap.indent(textwrap.dedent(source), "    ")
    namespace = {}
    exec(compile(wrapped, "<u_dot_generalized with PR #1196>", "exec"),
         vars(rocketpy_flight), namespace)
    return namespace["Flight"].u_dot_generalized


class CorrectedFlight(Flight):
    """RocketPy 1.13.0's `Flight` with PR #1196's `u_dot_generalized`.

    `Flight.__init__` binds `self.u_dot_generalized` for the default `equations_of_motion`, so the
    corrected method is the one every phase from the rail exit to apogee integrates, and the one
    the post-processing re-evaluates.
    """

    u_dot_generalized = corrected_u_dot_generalized()


def correct_nozzle_gyration(rocket):
    """Sets `rocket.nozzle_gyration_tensor` as PR #1188's `evaluate_nozzle_gyration_tensor` does:
    the exit disc's second moment per unit area about the centre of dry mass, `r^2/4 + l^2`
    across and `r^2/2` along the axis. Call it after `add_motor`, which evaluates the tensor."""
    radius = rocket.motor.nozzle_radius
    axial = 0.5 * radius**2
    lateral = 0.5 * axial + rocket.nozzle_to_cdm**2
    rocket.nozzle_gyration_tensor = Matrix(
        [[lateral, 0, 0], [0, lateral, 0], [0, 0, axial]]
    )
