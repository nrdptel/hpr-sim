"""Reference values for moist air: CIPM-2007 density, and the checks quoted in docs/physics.

Sources:
- A. Picard, R. S. Davis, M. Gläser and K. Fujii, "Revised formula for the density of moist air
  (CIPM-2007)", Metrologia 45 (2008) 149-155, eqs. (1), (4), (A1.1) to (A1.4)
  (`picard-2008-cipm-2007` in validation/refs.lock.toml).
- WMO-No. 8, Guide to Instruments and Methods of Observation, Vol. I (2023), Annex 4.B, eq. 4.B.1
  (`wmo-no8-vol1-2023`).
- IAPWS R12-08, Release on the IAPWS Formulation 2008 for the Viscosity of Ordinary Water
  Substance, eq. (11) and Table 1 (`iapws-r12-08`); C. R. Wilke, "A viscosity equation for gas
  mixtures", J. Chem. Phys. 18 (1950) 517.

CIPM-2007 is the metrological reference: a real-gas equation with compressibility, stated for
600 to 1100 hPa and 15 to 27 °C. hpr-atmos uses an ideal mixture with the 1976 standard's dry-air
constants, and its test bounds the difference. This script shares no code with hpr-atmos.

Run from the repository root (the oracle environment has mpmath):

    refs/venv/bin/python validation/oracles/atmosphere/moist_air.py \
        > validation/fixtures/atmosphere/cipm-2007-moist-air-density.json

It writes the fixture to stdout and the documentation checks to stderr.
"""

import hashlib
import json
import sys
from pathlib import Path

import mpmath
from mpmath import exp, mp, mpf, sqrt

# Provenance written into the fixture (docs/VALIDATION.md). Update the date when regenerating.
GENERATED = "2026-09-17"
COMMAND = (
    "refs/venv/bin/python validation/oracles/atmosphere/moist_air.py "
    "> validation/fixtures/atmosphere/cipm-2007-moist-air-density.json"
)
# sha256 of the pinned source, `picard-2008-cipm-2007` in validation/refs.lock.toml.
SOURCE_SHA256 = "0266277b62e34253a36af817e70af4ffbe74f079ed7c35ce3770477c2ea822e3"

mp.dps = 30

# CIPM-2007, eq. (4) and the text after it.
R = mpf("8.314472")  # J/(mol K)
X_CO2 = mpf("0.0004")
M_A = (mpf("28.96546") + mpf("12.011") * (X_CO2 - mpf("0.0004"))) * mpf("1e-3")  # kg/mol
M_V = mpf("18.01528e-3")  # kg/mol

# (A1.1) saturation vapour pressure, (A1.2) enhancement factor.
A, B, C, D = mpf("1.2378847e-5"), mpf("-1.9121316e-2"), mpf("33.93711047"), mpf("-6.3431645e3")
ALPHA, BETA, GAMMA = mpf("1.00062"), mpf("3.14e-8"), mpf("5.6e-7")

# (A1.4) compressibility.
A0, A1, A2 = mpf("1.58123e-6"), mpf("-2.9331e-8"), mpf("1.1043e-10")
B0, B1 = mpf("5.707e-6"), mpf("-2.051e-8")
C0, C1 = mpf("1.9898e-4"), mpf("-2.376e-6")
D_Z, E_Z = mpf("1.83e-11"), mpf("-0.765e-8")


def cipm_2007_density(temperature_k, pressure_pa, relative_humidity):
    T = mpf(temperature_k)
    p = mpf(pressure_pa)
    h = mpf(relative_humidity)
    t = T - mpf("273.15")
    p_sv = exp(A * T**2 + B * T + C + D / T)  # (A1.1)
    f = ALPHA + BETA * p + GAMMA * t**2  # (A1.2)
    x_v = h * f * p_sv / p  # (A1.3)
    Z = (
        1
        - p / T * (A0 + A1 * t + A2 * t**2 + (B0 + B1 * t) * x_v + (C0 + C1 * t) * x_v**2)
        + p**2 / T**2 * (D_Z + E_Z * x_v**2)
    )  # (A1.4)
    return p * M_A / (Z * R * T) * (1 - x_v * (1 - M_V / M_A))  # (1)


def wmo_saturation_hpa(t_celsius):
    t = mpf(t_celsius)
    return mpf("6.112") * exp(mpf("17.62") * t / (mpf("243.12") + t))  # (4.B.1)


# IAPWS R12-08 eq. (11), Table 1: dilute-gas viscosity of water vapour.
H = [mpf("1.67752"), mpf("2.20462"), mpf("0.6366564"), mpf("-0.241605")]


def water_vapour_viscosity(temperature_k):
    tr = mpf(temperature_k) / mpf("647.096")
    return mpf("1e-6") * 100 * sqrt(tr) / sum(h / tr**i for i, h in enumerate(H))


def sutherland(temperature_k):
    T = mpf(temperature_k)
    return mpf("1.458e-6") * T**mpf("1.5") / (T + mpf("110.4"))


def wilke(x, mu, molar_mass):
    """Wilke's rule for the viscosity of a gas mixture."""
    total = mpf(0)
    for i in range(len(x)):
        denominator = mpf(0)
        for j in range(len(x)):
            phi = (1 + sqrt(mu[i] / mu[j]) * (molar_mass[j] / molar_mass[i]) ** mpf("0.25")) ** 2 / sqrt(
                8 * (1 + molar_mass[i] / molar_mass[j])
            )
            denominator += x[j] * phi
        total += x[i] * mu[i] / denominator
    return total


def main():
    cases = []
    for t_c in ["15", "21", "27"]:
        for p in ["60000", "85000", "101325", "110000"]:
            for h in ["0", "0.5", "1"]:
                T = mpf("273.15") + mpf(t_c)
                cases.append(
                    {
                        "temperature_k": float(T),
                        "pressure_pa": float(p),
                        "relative_humidity": float(h),
                        "cipm_2007_density_kg_m3": float(cipm_2007_density(T, p, h)),
                    }
                )
    fixture = {
        "source": "A. Picard et al., Revised formula for the density of moist air (CIPM-2007), "
        "Metrologia 45 (2008) 149-155, eqs. (1), (4), (A1.1)-(A1.4); x_CO2 = 400 umol/mol",
        "generator": "validation/oracles/atmosphere/moist_air.py (mpmath, 30 digits)",
        "tool": f"mpmath {mpmath.__version__}",
        "generated": GENERATED,
        "command": COMMAND,
        "inputs_sha256": {
            "source": SOURCE_SHA256,
            "script": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        },
        "range": "CIPM-2007's stated range: 600-1100 hPa, 15-27 degC, 0-100% relative humidity",
        "cases": cases,
    }
    print(json.dumps(fixture, indent=2))

    log = sys.stderr
    print("WMO 4.B.1 e_w (hPa):", file=log)
    for t_c in ["0", "20", "-40", "-60"]:
        print(f"  {t_c:>4} degC  {float(wmo_saturation_hpa(t_c)):.9g}", file=log)

    # Viscosity of saturated air at 30 degC, 1013.25 hPa: Wilke mixture of dry air (Sutherland)
    # and water vapour (IAPWS), against dry air.
    T = mpf("303.15")
    p = mpf("101325")
    x_v = wmo_saturation_hpa("30") * 100 / p
    mu_dry = sutherland(T)
    mu_mix = wilke([1 - x_v, x_v], [mu_dry, water_vapour_viscosity(T)], [mpf("28.9644"), mpf("18.01528")])
    print(f"Saturated air at 30 degC: x_v = {float(x_v):.4f}, viscosity change "
          f"{float(mu_mix / mu_dry - 1) * 100:+.2f}%", file=log)

    # Heat capacity of water vapour: 4 R* against a fixed reference (NIST-JANAF gives about
    # 33.6 J/(mol K) near 300 K); the effect on the speed of sound of saturated air at 30 degC.
    def sound(cp_v):
        m = (1 - x_v) * mpf("28.9644") + x_v * mpf("18.01528")
        cp = (1 - x_v) * mpf("3.5") + x_v * cp_v
        return sqrt(cp / (cp - 1) * mpf("8314.32") * T / m)

    print(f"Speed of sound, C_p,v = 4.04 R* against 4 R*: "
          f"{float(sound(mpf('4.04')) / sound(4) - 1) * 100:+.4f}%", file=log)

    worst = 0.0
    for case in cases:
        T, p, h = mpf(case["temperature_k"]), mpf(case["pressure_pa"]), mpf(case["relative_humidity"])
        e = h * wmo_saturation_hpa(T - mpf("273.15")) * 100
        x = e / p
        ideal = p * ((1 - x) * mpf("28.9644") + x * mpf("18.01528")) / (mpf("8314.32") * T)
        worst = max(worst, abs(float(ideal / mpf(case["cipm_2007_density_kg_m3"]) - 1)))
    print(f"Ideal mixture (1976 constants, WMO e_w) against CIPM-2007: largest |error| "
          f"{worst * 100:.4f}%", file=log)


if __name__ == "__main__":
    main()
