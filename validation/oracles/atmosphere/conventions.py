"""Numbers quoted in docs/physics/atmosphere.md about atmosphere conventions.

1. Two readings of "ISA + dT" at a geopotential height of 3000 m with dT = +20 K and sea-level
   pressure 101325 Pa: hpr-atmos offsets the temperature at equal geopotential height; the
   aviation (ESDU 77022) convention offsets it at equal pressure altitude. Both are hydrostatic.
2. How far pressure interpolated linearly in height (as RocketPy does) is from the hydrostatic
   pressure between standard pressure levels of the dry 1976 standard atmosphere.
3. How an offset anchored at a launch site drifts from the standard aloft: +20 K at a 1400 m
   field, at the standard's pressure there.

Plain Python, from the 1976 standard's constants and layers (Table 4); shares no code with
hpr-atmos. Run from the repository root:

    refs/venv/bin/python validation/oracles/atmosphere/conventions.py
"""

import math

R_D = 8314.32 / 28.9644
K = 9.80665 / R_D
LAYERS = [(0, -0.0065), (11000, 0), (20000, 0.001), (32000, 0.0028), (47000, 0), (51000, -0.0028), (71000, -0.002)]


def standard(h, dt=0.0, p0=101325.0):
    """Temperature and pressure at geopotential height h (below 84.852 km')."""
    t, p = 288.15 + dt, p0
    for i, (base, lapse) in enumerate(LAYERS):
        top = LAYERS[i + 1][0] if i + 1 < len(LAYERS) else 84852.0
        h1 = min(h, top)
        if lapse == 0:
            t1, p1 = t, p * math.exp(-K * (h1 - base) / t)
        else:
            t1 = t + lapse * (h1 - base)
            p1 = p * (t / t1) ** (K / lapse)
        if h <= top:
            return t1, p1
        t, p = t1, p1
    raise ValueError(h)


def bisect(f, lo, hi):
    for _ in range(200):
        mid = (lo + hi) / 2
        if f(mid):
            lo = mid
        else:
            hi = mid
    return lo


def main():
    dt, h = 20.0, 3000.0
    t1, p1 = standard(h, dt)
    rho1 = p1 / (R_D * t1)
    # Pressure altitude hp whose geopotential height is h: dH = (T_std + dT)/T_std dHp (layer 0).
    hp = bisect(lambda x: x + dt / -0.0065 * math.log((288.15 - 0.0065 * x) / 288.15) < h, 0.0, h)
    t2, p2 = standard(hp)
    t2 += dt
    rho2 = p2 / (R_D * t2)
    print(f"ISA+20 K at 3000 m': equal-height offset T = {t1:.2f} K, p = {p1:.0f} Pa, rho = {rho1:.5f}")
    print(f"                      pressure-altitude offset T = {t2:.2f} K, p = {p2:.0f} Pa, rho = {rho2:.5f}")
    print(f"                      density difference {(rho1 / rho2 - 1) * 100:.2f}%")

    def height_of(pressure):
        return bisect(lambda x: standard(x)[1] > pressure, -1000.0, 40000.0)

    for a, b in [(100000, 92500), (70000, 50000), (5000, 3000)]:
        ha, hb = height_of(a), height_of(b)
        worst = max(
            abs((a + (b - a) * k / 400) / standard(ha + (hb - ha) * k / 400)[1] - 1) for k in range(1, 400)
        )
        print(f"{a / 100:.0f} to {b / 100:.0f} hPa: linear-in-height pressure is off by up to {worst * 100:.2f}%")

    r0 = 6356766.0
    field = r0 * 1400.0 / (r0 + 1400.0)
    _, p_field = standard(field)
    _, p_unit = standard(field, 20.0, 1.0)
    p0 = p_field / p_unit
    for z in [3000.0, 20000.0, 30000.0]:
        h1 = r0 * z / (r0 + z)
        ts, ps = standard(h1)
        ta, pa = standard(h1, 20.0, p0)
        print(f"+20 K anchored at a 1400 m field: density at {z / 1000:.0f} km is "
              f"{((pa / ta) / (ps / ts) - 1) * 100:+.1f}% against the standard")


if __name__ == "__main__":
    main()
