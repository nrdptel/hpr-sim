"""Cross-check of the transcribed 1976 US Standard Atmosphere tables against two computations.

Source: U.S. Standard Atmosphere, 1976, NOAA-S/T 76-1562 (`us-std-atmosphere-1976` in
validation/refs.lock.toml). validation/fixtures/atmosphere/ussa76-table-i.json holds rows of
Table I (temperature, pressure, density) and Table III (speed of sound, viscosities), transcribed
by hand from the scan; ussa76-constants.json holds the defining constants, Table 4 and Table 8.

Every printed value is compared with
  (a) the `ambiance` package (Apache-2.0), which implements the same model from -5004 m to
      81020 m geometric, so it has no value for the rows above that; and
  (b) the equations of the Standard evaluated here in mpmath at 40 digits from the transcribed
      constants: eqs. 18, 23, 33a, 33b, 42, 50, 51 and 52.
It shares no code with hpr-atmos. ambiance follows ICAO (1993): it uses R = 287.05287 J/(kg K) and
layer-base pressures rounded to 6 digits, so near the last printed digit it can disagree with the
tables where the mpmath evaluation does not.

For each row and column it prints the relative difference (printed - computed) / computed and
whether the printed value is within one count of its last printed digit of the computed value
("1ct" yes or NO). The tables were computed without the M/M0 correction below 86 km (page 9), so
the printed kinetic temperature and the viscosities below 86 km use T = T_M; the kinetic
temperature T_M * M/M0 from Table 8 is reported beside them for 80 to 86 km. At 86 km the printed
T is the kinetic temperature of the upper model, T_M * M7/M0 (page 10).

Before comparing, it checks the fixture itself: each `si` number equals its `printed` string in SI,
and the altitudes of Table 8 agree with eqs. 18 and 19 to the printed 0.1 m.

Exits with status 1 if any printed value differs from either computation by more than 0.1%.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/ussa76/tables.py
"""

import importlib.metadata
import json
import sys
from decimal import Decimal
from pathlib import Path

from ambiance import Atmosphere
from mpmath import exp, mp, mpf, sqrt

mp.dps = 40

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "validation" / "fixtures" / "atmosphere"
TABLE = json.loads((FIXTURES / "ussa76-table-i.json").read_text(encoding="utf-8"))
CONSTANTS = json.loads((FIXTURES / "ussa76-constants.json").read_text(encoding="utf-8"))
TOLERANCE = mpf("1e-3")


def constant(name):
    # repr of a JSON float is its shortest round-trip form, i.e. the transcribed digits.
    return mpf(repr(CONSTANTS["constants"][name]["value"]))


R_STAR = constant("gas_constant_jpkmolk")
M0 = constant("sea_level_molecular_weight_kgpkmol")
G0_PRIME = constant("g0_prime_m2ps2pgpm")
R0 = constant("earth_radius_m")
P0 = constant("sea_level_pressure_pa")
T0 = constant("sea_level_temperature_k")
BETA = constant("sutherland_beta_kgpsmk12")
S = constant("sutherland_constant_k")
GAMMA = constant("ratio_of_specific_heats")
M7_OVER_M0 = constant("molecular_weight_ratio_86_km")

# Table 4, converted to m' and K/m'.
LAYERS = [
    (mpf(repr(row["base_geopotential_km"])) * 1000, None if row["gradient_kpkm"] is None
     else mpf(repr(row["gradient_kpkm"])) / 1000)
    for row in CONSTANTS["layers"]["rows"]
]


def layer_state(h_b, l_b, t_b, p_b, h):
    """T_M (eq. 23) and P (eq. 33a, or 33b when L_M,b = 0) at geopotential h inside a layer."""
    t_m = t_b + l_b * (h - h_b)
    if l_b == 0:
        p = p_b * exp(-G0_PRIME * M0 * (h - h_b) / (R_STAR * t_b))
    else:
        p = p_b * (t_b / t_m) ** (G0_PRIME * M0 / (R_STAR * l_b))
    return t_m, p


# The base values T_M,b and P_b of each layer, chained up from T0 and P0 (page 12).
BASES = []
t_b, p_b = T0, P0
for (h_b, l_b), (h_next, _) in zip(LAYERS, LAYERS[1:]):
    BASES.append((h_b, l_b, t_b, p_b))
    t_b, p_b = layer_state(h_b, l_b, t_b, p_b, h_next)


def geopotential(z):
    """Eq. 18 with g0/g0' = 1 m'/m."""
    return R0 * z / (R0 + z)


def geometric(h):
    """Eq. 19 with Gamma = 1 m'/m."""
    return R0 * h / (R0 - h)


def ussa76(z):
    """The Standard at geometric altitude z (m), as the tables were computed below 86 km."""
    h = geopotential(z)
    # Layer 0 also covers negative altitudes (page 12); the last layer covers 84852 m' too.
    h_b, l_b, t_b, p_b = next(base for base in reversed(BASES) if h >= base[0] or base is BASES[0])
    t_m, p = layer_state(h_b, l_b, t_b, p_b, h)
    rho = p * M0 / (R_STAR * t_m)  # eq. 42
    # At 86 km the printed T comes from the upper model (eq. 25); below it, T = T_M (page 9).
    t = t_m * M7_OVER_M0 if z >= 86000 else t_m
    mu = BETA * t ** mpf("1.5") / (t + S)  # eq. 51
    return {
        "geopotential_altitude_m": h,
        "temperature_k": t,
        "molecular_scale_temperature_k": t_m,
        "pressure_pa": p,
        "density_kgpm3": rho,
        "speed_of_sound_mps": sqrt(GAMMA * R_STAR * t_m / M0),  # eq. 50
        "dynamic_viscosity_pas": mu,
        "kinematic_viscosity_m2ps": mu / rho,  # eq. 52
    }


def reference_ambiance(z):
    if z > 81020 or z < -5004:
        return None
    atm = Atmosphere(float(z))
    t = mpf(float(atm.temperature[0]))
    return {
        "geopotential_altitude_m": mpf(float(atm.H[0])),
        "temperature_k": t,
        "molecular_scale_temperature_k": t,
        "pressure_pa": mpf(float(atm.pressure[0])),
        "density_kgpm3": mpf(float(atm.density[0])),
        "speed_of_sound_mps": mpf(float(atm.speed_of_sound[0])),
        "dynamic_viscosity_pas": mpf(float(atm.dynamic_viscosity[0])),
        "kinematic_viscosity_m2ps": mpf(float(atm.kinematic_viscosity[0])),
    }


# Printed key -> SI key and the exact factor from the printed unit to SI (Table 1).
COLUMNS = [
    ("geopotential_altitude_m", "geopotential_altitude_m", 1),
    ("temperature_k", "temperature_k", 1),
    ("molecular_scale_temperature_k", "molecular_scale_temperature_k", 1),
    ("pressure_mb", "pressure_pa", 100),
    ("density_kgpm3", "density_kgpm3", 1),
    ("speed_of_sound_mps", "speed_of_sound_mps", 1),
    ("dynamic_viscosity_pas", "dynamic_viscosity_pas", 1),
    ("kinematic_viscosity_m2ps", "kinematic_viscosity_m2ps", 1),
]


def one_count(printed):
    """The value of one unit in the last printed digit, in the printed unit."""
    mantissa, _, power = printed.partition("E")
    decimals = len(mantissa.partition(".")[2])
    return Decimal(10) ** (int(power or 0) - decimals)


# 1. The fixture is self-consistent: si is the printed string in SI, exactly.
for row in TABLE["rows"]:
    for printed_key, si_key, factor in COLUMNS:
        printed, si = row["printed"][printed_key], row["si"][si_key]
        if printed is None:
            assert si is None, (row["geometric_altitude_m"], si_key)
            continue
        assert si == float(Decimal(printed) * factor), (row["geometric_altitude_m"], si_key, printed, si)

# 2. Table 8's altitudes agree with eqs. 18 and 19. Most are truncated to the printed 0.1 m, but
#    three differ by up to 0.12 m (confirmed on the page image), so a transcription slip is taken
#    to be anything beyond two counts, and the rows beyond one count are listed.
table_8 = CONSTANTS["table_8"]
table_8_checks = [
    (entry["geometric_altitude_m"], "H", geopotential(mpf(repr(entry["geometric_altitude_m"]))),
     mpf(repr(entry["geopotential_altitude_m"])))
    for entry in table_8["by_geometric"]
] + [
    (entry["geopotential_altitude_m"], "Z", geometric(mpf(repr(entry["geopotential_altitude_m"]))),
     mpf(repr(entry["geometric_altitude_m"])))
    for entry in table_8["by_geopotential"]
]
for argument, name, computed, printed in table_8_checks:
    assert abs(printed - computed) <= mpf("0.2"), (argument, name, printed, computed)
    if abs(printed - computed) > mpf("0.1"):
        print(f"Table 8 at {argument:.0f} m: printed {name} = {printed}, "
              f"eq. 18/19 gives {mp.nstr(computed, 9)}")


def m_over_m0(z):
    """Table 8 (geometric half), interpolated linearly in Z; 1 below 80 km."""
    points = [(mpf(repr(e["geometric_altitude_m"])), mpf(repr(e["m_over_m0"])))
              for e in table_8["by_geometric"]]
    if z <= points[0][0]:
        return mpf(1)
    for (z0, r0), (z1, r1) in zip(points, points[1:]):
        if z <= z1:
            return r0 + (r1 - r0) * (z - z0) / (z1 - z0)
    return points[-1][1]


# 3. Sutherland's constant: the printed sea-level viscosity decides between 110 K and 110.4 K.
sea_level = next(r for r in TABLE["rows"] if r["geometric_altitude_m"] == 0.0)
for s in ("110", "110.4"):
    mu0 = BETA * T0 ** mpf("1.5") / (T0 + mpf(s))
    printed_mu0 = sea_level["printed"]["dynamic_viscosity_pas"]
    print(f"sea-level mu with S = {s} K: {mp.nstr(mu0, 6)} (printed {printed_mu0})")
print(f"ambiance {importlib.metadata.version('ambiance')}, mpmath {mp.dps} digits")
print()

print(f"{'Z (m)':>7}  {'column':<30} {'printed':>11}  "
      f"{'mpmath':>13} {'rel':>10} {'1ct':>3}  {'ambiance':>13} {'rel':>10} {'1ct':>3}")
worst = {si_key: {"mpmath": mpf(0), "ambiance": mpf(0)} for _, si_key, _ in COLUMNS}
misses = []
failures = []
for row in TABLE["rows"]:
    z = mpf(repr(row["geometric_altitude_m"]))
    computed = {"mpmath": ussa76(z), "ambiance": reference_ambiance(z)}
    for printed_key, si_key, factor in COLUMNS:
        printed = row["printed"][printed_key]
        if printed is None:
            continue
        value = mpf(printed) * factor
        count = mpf(str(one_count(printed))) * factor
        cells = []
        for name in ("mpmath", "ambiance"):
            reference = computed[name]
            if reference is None:
                cells.append(f"{'-':>13} {'-':>10} {'-':>3}")
                continue
            c = reference[si_key]
            rel = (value - c) / c if c != 0 else value - c
            within = abs(value - c) <= count
            worst[si_key][name] = max(worst[si_key][name], abs(rel))
            if not within:
                misses.append((int(z), si_key, name, printed, mp.nstr(c / factor, 8)))
            if abs(rel) > TOLERANCE:
                failures.append((int(z), si_key, name, printed, mp.nstr(c / factor, 8)))
            rel_text = mp.nstr(rel, 3, min_fixed=0, max_fixed=0)
            cells.append(f"{mp.nstr(c / factor, 8):>13} {rel_text:>10} {'yes' if within else 'NO':>3}")
        print(f"{int(z):>7}  {printed_key:<30} {printed:>11}  {cells[0]}  {cells[1]}")
    if 80000 <= z < 86000:
        t_m = computed["mpmath"]["molecular_scale_temperature_k"]
        kinetic = t_m * m_over_m0(z)
        printed_t = mpf(row["printed"]["temperature_k"])
        print(
            f"{int(z):>7}  kinetic T = T_M*M/M0 = {mp.nstr(kinetic, 7)} K (M/M0 = {mp.nstr(m_over_m0(z), 7)}); "
            f"printed T differs by {mp.nstr((printed_t - kinetic) / kinetic, 3)}"
        )

print()
print("Largest |relative difference| per column (mpmath, ambiance):")
for _, si_key, _ in COLUMNS:
    print(f"  {si_key:<30} {mp.nstr(worst[si_key]['mpmath'], 3):>10}  {mp.nstr(worst[si_key]['ambiance'], 3):>10}")
print()
print(f"Printed values not within one count of the computation: {len(misses)}")
for miss in misses:
    print("  Z = {} m, {}, {}: printed {}, computed {}".format(*miss))
print(f"Printed values more than 0.1% from a computation: {len(failures)}")
for failure in failures:
    print("  Z = {} m, {}, {}: printed {}, computed {}".format(*failure))
sys.exit(1 if failures else 0)
