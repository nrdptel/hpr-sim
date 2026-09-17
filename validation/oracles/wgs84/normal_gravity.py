"""Reference values of WGS 84 normal gravity, from the published formulas at 40 significant digits.

Source: NGA.STND.0036_1.0.0_WGS84 (2014-07-08), "Department of Defense World Geodetic System 1984",
chapter 4 and appendix B (`wgs84-nga-stnd-0036` in validation/refs.lock.toml). No publication
tabulates normal gravity over latitude and height, so this script evaluates the published formulas
in arbitrary precision. It shares no code with hpr-core. The derived constants are checked against
the published digits of Tables 3.5 and 3.6 before any value is written.

Run from the repository root (the oracle environment has mpmath):

    refs/venv/bin/python validation/oracles/wgs84/normal_gravity.py \
        > validation/fixtures/earth/wgs84-normal-gravity.json
"""

import json

from mpmath import mp, mpf, atan, atan2, cos, radians, sin, sqrt

mp.dps = 40

# Defining parameters, Table 3.1.
A = mpf("6378137.0")
INV_F = mpf("298.257223563")
GM = mpf("3.986004418e14")
OMEGA = mpf("7.292115e-5")

F = 1 / INV_F
B = A * (1 - F)
E2 = F * (2 - F)
E_LIN = sqrt(A**2 - B**2)  # (4-7)
E_PRIME = E_LIN / B

# Appendix B.
Q0 = ((1 + 3 / E_PRIME**2) * atan(E_PRIME) - 3 / E_PRIME) / 2  # (B-18)
Q0_PRIME = 3 * ((1 + 1 / E_PRIME**2) * (1 - atan(E_PRIME) / E_PRIME)) - 1  # (B-19)
M = OMEGA**2 * A**2 * B / GM  # (B-20)
GAMMA_E = GM / (A * B) * (1 - M - M * E_PRIME * Q0_PRIME / (6 * Q0))  # (B-24)
GAMMA_P = GM / A**2 * (1 + M * E_PRIME * Q0_PRIME / (3 * Q0))  # (B-25)
K = (B * GAMMA_P - A * GAMMA_E) / (A * GAMMA_E)  # (B-26)

# Published digits (Tables 3.5 and 3.6): each derived value must round to them.
PUBLISHED = {
    "b": (B, "6356752.3142"),
    "e2": (E2, "6.694379990141e-3"),
    "E": (E_LIN, "5.2185400842339e5"),
    "gamma_e": (GAMMA_E, "9.7803253359"),
    "gamma_p": (GAMMA_P, "9.8321849379"),
    "k": (K, "1.931852652458e-3"),
    "m": (M, "3.449786506841e-3"),
}
for name, (value, printed) in PUBLISHED.items():
    digits = len(printed.split("e")[0].replace(".", "").replace("-", "").lstrip("0"))
    assert mp.nstr(value, digits, strip_zeros=False) == mp.nstr(mpf(printed), digits, strip_zeros=False), (
        name,
        mp.nstr(value, 20),
        printed,
    )


def surface(lat):
    """Somigliana, eq. (4-1)."""
    s2 = sin(lat) ** 2
    return GAMMA_E * (1 + K * s2) / sqrt(1 - E2 * s2)


def taylor(lat, h):
    """Truncated Taylor series in height, eq. (4-3)."""
    return surface(lat) * (1 - 2 / A * (1 + F + M - 2 * F * sin(lat) ** 2) * h + 3 / A**2 * h**2)


def cartesian(lat, lon, h):
    """Geodetic to Cartesian, eqs. (4-14) and (4-15)."""
    n = A / sqrt(1 - E2 * sin(lat) ** 2)
    return (
        (n + h) * cos(lat) * cos(lon),
        (n + h) * cos(lat) * sin(lon),
        ((B**2 / A**2) * n + h) * sin(lat),
    )


def ellipsoidal(x, y, z):
    """Normal gravity components and the ECEF vector, eqs. (4-5) to (4-13) and (4-18)."""
    r2 = x**2 + y**2 + z**2
    u = sqrt((r2 - E_LIN**2) / 2 * (1 + sqrt(1 + 4 * E_LIN**2 * z**2 / (r2 - E_LIN**2) ** 2)))  # (4-8)
    p = sqrt(x**2 + y**2)
    beta = atan2(z * sqrt(u**2 + E_LIN**2), u * p)  # (4-9), all quadrants
    w = sqrt((u**2 + E_LIN**2 * sin(beta) ** 2) / (u**2 + E_LIN**2))  # (4-10)
    q = ((1 + 3 * u**2 / E_LIN**2) * atan(E_LIN / u) - 3 * u / E_LIN) / 2  # (4-11)
    q0 = ((1 + 3 * B**2 / E_LIN**2) * atan(E_LIN / B) - 3 * B / E_LIN) / 2  # (4-12)
    q_prime = 3 * (1 + u**2 / E_LIN**2) * (1 - u / E_LIN * atan(E_LIN / u)) - 1  # (4-13)
    s = u**2 + E_LIN**2
    gamma_u = -(1 / w) * (
        GM / s + OMEGA**2 * A**2 * E_LIN / s * (q_prime / q0) * (sin(beta) ** 2 / 2 - mpf(1) / 6)
    ) + (1 / w) * OMEGA**2 * u * cos(beta) ** 2  # (4-5)
    gamma_beta = (1 / w) * OMEGA**2 * A**2 / sqrt(s) * (q / q0) * sin(beta) * cos(beta) - (
        1 / w
    ) * OMEGA**2 * sqrt(s) * sin(beta) * cos(beta)  # (4-6)
    lon = atan2(y, x)
    c = u / (w * sqrt(s))
    # (4-18): R1 maps (gamma_u, gamma_beta, gamma_lambda = 0) to (gamma_x, gamma_y, gamma_z).
    gx = c * cos(beta) * cos(lon) * gamma_u - sin(beta) * cos(lon) / w * gamma_beta
    gy = c * cos(beta) * sin(lon) * gamma_u - sin(beta) * sin(lon) / w * gamma_beta
    gz = sin(beta) / w * gamma_u + c * cos(beta) * gamma_beta
    return gamma_u, gamma_beta, (gx, gy, gz)


def exact_components(lat, lon, h):
    """Exact gamma_h (4-16) and gamma_phi (4-23) through the spherical components (4-17)-(4-20)."""
    x, y, z = cartesian(lat, lon, h)
    _, _, (gx, gy, gz) = ellipsoidal(x, y, z)
    psi = atan2(z, sqrt(x**2 + y**2))  # geocentric latitude
    # (4-19): R2 maps (gamma_x, gamma_y, gamma_z) to (gamma_r, gamma_psi, gamma_lambda).
    gamma_r = cos(psi) * cos(lon) * gx + cos(psi) * sin(lon) * gy + sin(psi) * gz
    gamma_psi = -sin(psi) * cos(lon) * gx - sin(psi) * sin(lon) * gy + cos(psi) * gz
    alpha = lat - psi  # (4-20)
    gamma_h = -gamma_r * cos(alpha) - gamma_psi * sin(alpha)  # (4-16)
    gamma_phi = -gamma_r * sin(alpha) + gamma_psi * cos(alpha)  # (4-23)
    return gamma_h, gamma_phi, (gx, gy, gz)


def potential(x, y, z):
    """Normal potential U (gravitational plus centrifugal) in ellipsoidal-harmonic coordinates, the
    potential that (4-5) and (4-6) differentiate: W. A. Heiskanen and H. Moritz, Physical Geodesy,
    1967, eq. 2-126 (NGA.STND.0036 reference [18]). The checks below confirm it independently: U is
    constant on the ellipsoid with the published U0, its gravitational part is harmonic above the
    ellipsoid, and its numerical gradient reproduces the ECEF vector."""
    r2 = x**2 + y**2 + z**2
    u = sqrt((r2 - E_LIN**2) / 2 * (1 + sqrt(1 + 4 * E_LIN**2 * z**2 / (r2 - E_LIN**2) ** 2)))
    beta = atan2(z * sqrt(u**2 + E_LIN**2), u * sqrt(x**2 + y**2))
    q = ((1 + 3 * u**2 / E_LIN**2) * atan(E_LIN / u) - 3 * u / E_LIN) / 2
    return (
        GM / E_LIN * atan(E_LIN / u)
        + OMEGA**2 * A**2 / 2 * (q / Q0) * (sin(beta) ** 2 - mpf(1) / 3)
        + OMEGA**2 / 2 * (u**2 + E_LIN**2) * cos(beta) ** 2
    )


# Cross-checks of the component formulas and the rotation R1, independent of both:
# 1. the ellipsoid is a level surface with the published U0 (Table 3.6);
for lat_deg in ("0", "45", "90"):
    u0 = potential(*cartesian(radians(mpf(lat_deg)), mpf(0), mpf(0)))
    assert mp.nstr(u0, 12) == mp.nstr(mpf("6.26368517146e7"), 12), (lat_deg, u0)


# 2. the gravitational part V = U - omega^2 (x^2 + y^2)/2 is harmonic outside the ellipsoid, so
#    q(u)/q0 off the ellipsoid is checked too (second central differences, step 1 m);
def assert_harmonic(x, y, z):
    d = mpf(1)

    def v(px, py, pz):
        return potential(px, py, pz) - OMEGA**2 * (px**2 + py**2) / 2

    centre = v(x, y, z)
    laplacian = (
        v(x + d, y, z) + v(x - d, y, z) + v(x, y + d, z) + v(x, y - d, z) + v(x, y, z + d) + v(x, y, z - d)
        - 6 * centre
    ) / d**2
    # Each second derivative is about GM/r^3 ~ 1.5e-6 s^-2; the sum must vanish.
    assert abs(laplacian) < mpf("1e-18"), laplacian


# 3. the gravity vector is the gradient of U (central differences, step 1e-4 m).
def assert_gradient(x, y, z, vector):
    d = mpf("1e-4")
    grad = (
        (potential(x + d, y, z) - potential(x - d, y, z)) / (2 * d),
        (potential(x, y + d, z) - potential(x, y - d, z)) / (2 * d),
        (potential(x, y, z + d) - potential(x, y, z - d)) / (2 * d),
    )
    for g, v in zip(grad, vector):
        assert abs(g - v) < mpf("1e-12"), (g, v)


# (latitude deg, longitude deg, ellipsoidal height m): the equator and poles, launch sites
# (Spaceport America, a southern-hemisphere field, a Florida coast site), and heights to 200 km.
POINTS = [
    ("0", "0", "0"),
    ("45", "0", "0"),
    ("90", "0", "0"),
    ("-90", "0", "0"),
    ("32.99", "-106.97", "1400"),
    ("-33.9", "18.6", "300"),
    ("60", "10", "10000"),
    ("28.5", "-80.6", "30000"),
    ("45", "135", "100000"),
    ("-71.3", "-170", "5000"),
    ("15", "250", "200000"),
]


def f(value):
    return float(mp.nstr(value, 20))


cases = []
for lat_s, lon_s, h_s in POINTS:
    lat, lon, h = radians(mpf(lat_s)), radians(mpf(lon_s)), mpf(h_s)
    x, y, z = cartesian(lat, lon, h)
    gamma_u, gamma_beta, (gx, gy, gz) = ellipsoidal(x, y, z)
    gamma_h, gamma_phi, _ = exact_components(lat, lon, h)
    assert_gradient(x, y, z, (gx, gy, gz))
    if h > 0:
        assert_harmonic(x, y, z)
    cases.append(
        {
            "latitude_deg": float(lat_s),
            "longitude_deg": float(lon_s),
            "height_m": float(h_s),
            "surface_mps2": f(surface(lat)),
            "taylor_mps2": f(taylor(lat, h)),
            "magnitude_mps2": f(sqrt(gamma_u**2 + gamma_beta**2)),  # (4-4)
            "down_mps2": f(gamma_h),
            "north_mps2": f(gamma_phi),
            "ecef_mps2": [f(gx), f(gy), f(gz)],
        }
    )

print(
    json.dumps(
        {
            "source": "NGA.STND.0036_1.0.0_WGS84 (2014), eqs. 4-1, 4-3 to 4-20, 4-23, B-18 to B-26",
            "generator": "validation/oracles/wgs84/normal_gravity.py (mpmath, 40 digits)",
            "constants": {
                "gamma_e_mps2": f(GAMMA_E),
                "gamma_p_mps2": f(GAMMA_P),
                "k": f(K),
                "m": f(M),
                "b_m": f(B),
                "e2": f(E2),
                "linear_eccentricity_m": f(E_LIN),
            },
            "cases": cases,
        },
        indent=2,
    )
)
