# The hpr design format 0.2: types for a document, and a reader that checks one.
# SPDX-License-Identifier: MIT OR Apache-2.0 (https://github.com/nrdptel/hpr-sim)
# Generated from schema/format/hpr-design-0.2.schema.json by `cargo xtask format`.
# Don't edit by hand: the next run overwrites it, and CI fails while it is stale.

"""Types for the hpr design format 0.2, and a reader that checks a document against its
schema. Generated from the schema by `cargo xtask format`; don't edit by hand."""

import json
import math
import re
from typing import Any, Literal, NotRequired, TypedDict, Union, cast

FORMAT = "hpr-design"
"""The format's name: every document says `"format": "hpr-design"`."""

VERSION = "0.2"
"""The version these types describe; `read_design` reads documents of this version only."""


class DesignFile(TypedDict):
    """A rocket design in the hpr design format."""

    extensions: "Extensions"
    """What the source file holds that hpr does not model, by namespace, kept for writing it
    back.
    """

    format: "Format"
    """Always `hpr-design`."""

    motors: "Motors"
    """Every motor configuration, flown or not, with why one is not."""

    provenance: "Provenance"
    """Which program wrote the document, and from what."""

    recovery: "Recovery"
    """When each parachute and streamer opens and each stage separates."""

    rocket: "Rocket"
    """The rocket: its stages and their parts, with every configuration that flies."""

    simulations: list["StoredSimulation"]
    """The simulations the source file stored, with their conditions and results."""

    source_files: list["SourceFile"]
    """The source file's other files, in the order it held them: a `.ork` archive's entries
    besides the design, such as embedded thrust curves and decal images. Version 0.1 called
    them `attachments`.
    """

    version: "Version"
    """The format's version, `major.minor`."""


Atmosphere = Union["AtmosphereIsa", "AtmosphereExtended", "AtmosphereOther"]
"""The atmosphere a stored simulation was flown in."""


class AtmosphereIsa(TypedDict):
    """`isa`: the International Standard Atmosphere."""

    model: Literal["isa"]


class AtmosphereExtended(TypedDict):
    """`extendedisa`: the standard atmosphere from a temperature and pressure at the launch site."""

    model: Literal["extended"]

    pressure_pa: NotRequired[Union[float, None]]
    """`<basepressure>`, Pa."""

    temperature_k: NotRequired[Union[float, None]]
    """`<basetemperature>`, K."""


class AtmosphereOther(TypedDict):
    """A model not listed here, or none, kept by name only."""

    model: Literal["other"]

    name: str
    """The `model` attribute; empty when there is none."""


AutoDimension = Literal[
    "base_radius",
    "outer_radius",
    "fore_radius",
    "aft_radius",
    "shoulder_radius",
    "fore_shoulder_radius",
    "aft_shoulder_radius",
    "inner_radius",
    "packed_radius",
]
"""A dimension resolved from the tree instead of stored in the part."""


class BodyTube(TypedDict):
    """An airframe tube."""

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (bulk)."""

    outer_radius_m: float
    """Outer radius, m."""

    thickness_m: float
    """Wall thickness, m."""


class CenteringRing(TypedDict):
    """A flat annular ring, or a bulkhead when the inner radius is zero."""

    inner_radius_m: float
    """Inner radius, m (zero for a bulkhead)."""

    length_m: float
    """Thickness along the axis, m."""

    material: "Material"
    """Material (bulk)."""

    outer_radius_m: float
    """Outer radius, m."""


class Component(TypedDict):
    """A node of the design tree: a part, where it sits, and what hangs off it."""

    auto: NotRequired[list["AutoDimension"]]
    """Dimensions taken from neighbours or the parent instead of the part's stored values."""

    children: NotRequired[list["Component"]]
    """Attached parts, or a pod set's body components."""

    finish: NotRequired[Union["Finish", None]]
    """The outer surface's finish, for skin friction; `None` means `Finish::default`. Parts
    inside the body ignore it.
    """

    id: str
    """Unique id."""

    motor_mount: NotRequired[Union["MotorMount", None]]
    """Makes a body tube or an inner tube a motor mount."""

    name: NotRequired[str]
    """Name.

    Absent means `""`.
    """

    overrides: NotRequired["Overrides"]
    """Mass, centre-of-mass and inertia overrides."""

    overrides_include_children: NotRequired[bool]
    """Whether the overrides replace this component together with everything attached to it
    (`true`), or this component alone (`false`).
    """

    part: "Part"
    """The part, with its geometry and material."""

    position: NotRequired[Union["Position", None]]
    """Where an attached part sits along its parent. Body components (a stage's own list) have
    none: they stack.
    """


class Configuration(TypedDict):
    """A set of motors to fly with: at most one per mount. A mount that is a cluster
    (`InnerTube::cluster_m`) takes its motor in every tube.
    """

    id: str
    """Unique id among the configurations."""

    motors: list["MountedMotor"]
    """The motors."""

    name: NotRequired[str]
    """Name.

    Absent means `""`.
    """


Content = Union["ContentText", "ContentBase64"]
"""A file's contents: text, or base64 for bytes that are not UTF-8."""


class ContentText(TypedDict):
    """UTF-8 text, as the file holds it."""

    text: str


class ContentBase64(TypedDict):
    """Any other bytes, in standard base64 with padding, which must decode."""

    base64: str


Curve = Union["CurveEmbedded", "CurveSupplied", "CurveCatalog", "CurveUnresolved"]
"""Where a `<motor>` element's thrust curve came from."""


class CurveEmbedded(TypedDict):
    """The archive's own `thrustcurves/<digest>.rse` entry."""

    entry: str
    """The archive entry, such as `thrustcurves/<digest>.rse`."""

    motor: "SolidMotor"
    """The motor built from it."""

    source: Literal["embedded"]


# A curve the caller supplied for the motor's digest (`SuppliedCurves`).
CurveSupplied = TypedDict(
    "CurveSupplied",
    {
        # The digest the design records, which the curve was supplied for.
        "digest": str,
        # Where the supplied curves came from (`SuppliedCurves::source`).
        "from": str,
        # The motor supplied.
        "motor": "SolidMotor",
        "source": Literal["supplied"],
    },
)


class CurveCatalog(TypedDict):
    """The bundled ThrustCurve.org catalog (`Catalog::bundled`)."""

    motor: "SolidMotor"
    """The motor built from it."""

    motor_id: str
    """ThrustCurve.org's motor id."""

    simfile_id: str
    """ThrustCurve.org's id of the curve file flown."""

    source: Literal["catalog"]


class CurveUnresolved(TypedDict):
    """No curve, and why."""

    reason: str
    """The same, in words."""

    source: Literal["unresolved"]

    why: "NoCurve"
    """Why no curve was found."""


Delay = Union["DelaySeconds", "DelayPlugged", "DelayZeroOrPlugged"]
"""One available delay setting. Serialized with a `kind` tag and the seconds as `value`:
`{"kind":"seconds","value":6.0}`, `{"kind":"plugged"}`.
"""


class DelaySeconds(TypedDict):
    """The ejection charge fires this many seconds after burnout: positive, or zero where a file
    says so plainly, as a `.ork` does for a charge at burnout (`hpr_io::ork`).
    """

    kind: Literal["seconds"]

    value: float


class DelayPlugged(TypedDict):
    """No ejection charge: the forward closure is plugged."""

    kind: Literal["plugged"]


class DelayZeroOrPlugged(TypedDict):
    """A `0`: the RASP spec means an ejection charge at burnout, but most files mean plugged
    (`docs/format/eng.md`). The user or the catalog has to settle which.
    """

    kind: Literal["zero_or_plugged"]


Density = Union["DensityBulk", "DensitySurface", "DensityLine"]
"""A density, with its units in the variant."""


class DensityBulk(TypedDict):
    """Mass per volume."""

    kg_m3: float
    """kg/m³."""

    kind: Literal["bulk"]


class DensitySurface(TypedDict):
    """Mass per area, for sheets and fabrics."""

    kg_m2: float
    """kg/m²."""

    kind: Literal["surface"]


class DensityLine(TypedDict):
    """Mass per length, for cords and lines."""

    kg_m: float
    """kg/m."""

    kind: Literal["line"]


DeployEvent = Union[
    Literal["launch"],
    Literal["ejection"],
    Literal["apogee"],
    Literal["altitude"],
    Literal["lower_stage_separation"],
    Literal["never"],
    "DeployEventOther",
]
"""What deploys a recovery device, as `<deployevent>` names it."""


class DeployEventOther(TypedDict):
    """A word not listed here, kept as written."""

    other: str


DeviceKind = Literal["parachute", "streamer"]
"""Which kind of recovery device."""


Dimension = Union["DimensionStated", "DimensionAutomatic"]
"""A number a `.ork` writes, which OpenRocket may be working out for itself."""


class DimensionStated(TypedDict):
    """A number the designer typed."""

    kind: Literal["stated"]

    value: float
    """The number, in whatever unit the tag is written in."""


class DimensionAutomatic(TypedDict):
    """A number OpenRocket works out from the neighbouring components."""

    cached: NotRequired[Union[float, None]]
    """What it last worked out, when the file says (`auto 0.0125`). A bare `auto` gives
    `None`. Either way this is a cached answer, not an input: a reader that resolves the
    dimension itself should prefer its own.
    """

    kind: Literal["automatic"]


class Element(TypedDict):
    """An XML element: its name, its attributes in the order they were written, and its children."""

    attributes: list[tuple[str, str]]
    """The attributes, in document order, as name and value."""

    children: list["Node"]
    """The children, in document order."""

    name: str
    """The element's name, such as `nosecone`."""


class EventSetting_for_DeployEvent(TypedDict):
    """An event, a height for the events that need one, and a delay after it: when a device deploys
    or a stage separates. Each is `None` where the file does not say.
    """

    altitude_m: NotRequired[Union[float, None]]
    """The height, m: above the ground for a deployment (measured; see the module docs)."""

    delay_s: NotRequired[Union[float, None]]
    """Seconds after the event, s."""

    event: NotRequired[Union["DeployEvent", None]]
    """The event."""


class EventSetting_for_SeparationEvent(TypedDict):
    """An event, a height for the events that need one, and a delay after it: when a device deploys
    or a stage separates. Each is `None` where the file does not say.
    """

    altitude_m: NotRequired[Union[float, None]]
    """The height, m: above the ground for a deployment (measured; see the module docs)."""

    delay_s: NotRequired[Union[float, None]]
    """Seconds after the event, s."""

    event: NotRequired[Union["SeparationEvent", None]]
    """The event."""


# The extensions a `.ork` design carries, by namespace.
Extensions = TypedDict(
    "Extensions",
    {
        # What the design holds that hpr does not model.
        #
        # Absent means `{"attributes":[],"parts":[],"sections":[],"tags":[]}`.
        "x-openrocket": NotRequired["OpenRocketExtension"],
    },
)


FinCrossSection = Literal["square", "rounded", "airfoil"]
"""The shape of a fin's section along its chord."""


class FinFillet(TypedDict):
    """Fillets along each fin's root: a concave joint on both faces, running the root chord."""

    material: "Material"
    """Material (bulk)."""

    radius_m: float
    """Radius of the fillet's concave face, m."""


FinPlanform = Union["FinPlanformTrapezoidal", "FinPlanformElliptical", "FinPlanformFreeform"]
"""The outline of one fin."""


class FinPlanformTrapezoidal(TypedDict):
    """A trapezoid with its tip chord parallel to the root."""

    kind: Literal["trapezoidal"]

    root_chord_m: float
    """Root chord, m."""

    span_m: float
    """Span from the body surface to the tip, m."""

    sweep_m: float
    """Axial distance from the root leading edge aft to the tip leading edge, m."""

    tip_chord_m: float
    """Tip chord, m (zero for a pointed fin)."""


class FinPlanformElliptical(TypedDict):
    """Half an ellipse on the root chord."""

    kind: Literal["elliptical"]

    root_chord_m: float
    """Root chord, m."""

    span_m: float
    """Span, m."""


class FinPlanformFreeform(TypedDict):
    """A polygon given as `[x, h]` points from the root leading edge, which must be `[0, 0]`,
    around to the root trailing edge `[c_r, 0]` with `c_r > 0`, closed along the root; `x` runs
    aft and `h` outward, in metres.
    """

    kind: Literal["freeform"]

    points_m: list[list[float]]
    """The outline, m."""


class FinSet(TypedDict):
    """A set of identical fins spaced evenly around the body."""

    base_angle_rad: NotRequired[float]
    """Roll angle of the first fin from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    cant_rad: NotRequired[float]
    """Cant angle, rad.

    Absent means `0.0`.
    """

    count: int
    """Number of fins, at least 1."""

    cross_section: NotRequired["FinCrossSection"]
    """Section shape.

    Absent means `"square"`.
    """

    fillet: NotRequired[Union["FinFillet", None]]
    """Optional fillets along each root."""

    material: "Material"
    """Material (bulk)."""

    planform: "FinPlanform"
    """Outline of each fin."""

    tab: NotRequired[Union["FinTab", None]]
    """Optional tab below each root.

    Absent means `null`.
    """

    thickness_m: float
    """Maximum thickness, m."""


class FinTab(TypedDict):
    """A rectangular tab below a fin's root, reaching into the body."""

    height_m: float
    """Depth below the root, m."""

    length_m: float
    """Length along the root, m."""

    offset_m: float
    """Distance from the root leading edge aft to the tab's leading edge, m."""


Finish = Union[
    "FinishMirror",
    "FinishAverageGlass",
    "FinishPolished",
    "FinishSheetMetal",
    "FinishOptimumPaint",
    "FinishPlanedWood",
    "FinishMassProductionPaint",
    "FinishBareSteel",
    "FinishSmoothCement",
    "FinishAsphaltCoating",
    "FinishDipGalvanized",
    "FinishPoorPaint",
    "FinishCastIron",
    "FinishRawWood",
    "FinishConcrete",
    "FinishCustom",
]
"""A surface finish, by its roughness height `R_s`."""


class FinishMirror(TypedDict):
    """A mirror-like surface, 0 µm (Barrowman Table 4-1)."""

    kind: Literal["mirror"]


class FinishAverageGlass(TypedDict):
    """Average glass, 0.1 µm."""

    kind: Literal["average_glass"]


class FinishPolished(TypedDict):
    """A finished and polished surface, 0.5 µm."""

    kind: Literal["polished"]


class FinishSheetMetal(TypedDict):
    """An aircraft-type sheet-metal surface, 2 µm (Barrowman Table 4-1 only)."""

    kind: Literal["sheet_metal"]


class FinishOptimumPaint(TypedDict):
    """An optimum paint-sprayed surface, 5 µm."""

    kind: Literal["optimum_paint"]


class FinishPlanedWood(TypedDict):
    """Planed wooden boards, 15 µm."""

    kind: Literal["planed_wood"]


class FinishMassProductionPaint(TypedDict):
    """Paint in aircraft mass production, 20 µm. The default."""

    kind: Literal["mass_production_paint"]


class FinishBareSteel(TypedDict):
    """Bare steel plating, 50 µm (Barrowman Table 4-1 only)."""

    kind: Literal["bare_steel"]


class FinishSmoothCement(TypedDict):
    """A smooth cement surface, 50 µm."""

    kind: Literal["smooth_cement"]


class FinishAsphaltCoating(TypedDict):
    """A surface with an asphalt-type coating, 100 µm (Barrowman Table 4-1 only)."""

    kind: Literal["asphalt_coating"]


class FinishDipGalvanized(TypedDict):
    """A dip-galvanized metal surface, 150 µm."""

    kind: Literal["dip_galvanized"]


class FinishPoorPaint(TypedDict):
    """Incorrectly sprayed aircraft paint, 200 µm."""

    kind: Literal["poor_paint"]


class FinishCastIron(TypedDict):
    """The natural surface of cast iron, 250 µm (Barrowman Table 4-1 only)."""

    kind: Literal["cast_iron"]


class FinishRawWood(TypedDict):
    """Raw wooden boards, 500 µm."""

    kind: Literal["raw_wood"]


class FinishConcrete(TypedDict):
    """An average concrete surface, 1000 µm."""

    kind: Literal["concrete"]


class FinishCustom(TypedDict):
    """A given roughness height."""

    kind: Literal["custom"]

    roughness_m: float
    """Roughness height, m."""


Format = Literal["hpr-design"]
"""The format's name: a document holds only `hpr-design`."""


Ignition = Union[
    Literal["launch"],
    "IgnitionTime",
    "IgnitionBurnout",
    "IgnitionSeparation",
    Literal["never"],
]
"""When a motor lights, on the flight's clock: `t = 0` is launch, when the motors that light at
launch ignite. A delay is counted from its event (the decision record on staging,
[ADR-074][adr-074]).

[adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
"""


class IgnitionTime(TypedDict):
    """At a time after launch: an air start on a timer."""

    time: "IgnitionTimeTime"


class IgnitionTimeTime(TypedDict):

    time_s: float
    """The time after launch, s."""


class IgnitionBurnout(TypedDict):
    """A delay after another mount's motor burns out: a sustainer lit by the booster's burnout,
    or by its ejection charge with the charge's delay.
    """

    burnout: "IgnitionBurnoutBurnout"


class IgnitionBurnoutBurnout(TypedDict):

    delay_s: float
    """The delay after that burnout, s."""

    mount: str
    """The id of the mount whose motor's burnout lights this one."""


class IgnitionSeparation(TypedDict):
    """A delay after the stage aft of this motor's stage separates from it (the flight gives the
    separation). A motor whose stage is never freed never lights; one in the last stage, with
    nothing aft of it to separate, is refused.
    """

    separation: "IgnitionSeparationSeparation"


class IgnitionSeparationSeparation(TypedDict):

    delay_s: float
    """The delay after the separation, s."""


IgnitionEvent = Union[
    Literal["automatic"],
    Literal["launch"],
    Literal["ejection_charge"],
    Literal["burnout"],
    Literal["never"],
    "IgnitionEventOther",
]
"""When a motor ignites, as `<ignitionevent>` names it.

OpenRocket's file-format page shows only `automatic`. The five values here are the ones
OpenRocket 24.12 writes, each measured by setting it and saving; the meanings are its own
labels, and for `automatic` its FAQ ("How do I create a staged rocket?"). Anything else is kept
as written.
"""


class IgnitionEventOther(TypedDict):
    """A value not listed here, kept as written."""

    other: str


class InertiaOverride(TypedDict):
    """An inertia tensor about the centre of mass in body axes, kg·m². The off-diagonal entries are
    the tensor's, `I_xy = −∫ x y dm` (`crate::mass`); they default to zero.
    """

    xx_kg_m2: float
    """`I_xx`, kg·m²."""

    xy_kg_m2: NotRequired[float]
    """`I_xy`, kg·m².

    Absent means `0.0`.
    """

    xz_kg_m2: NotRequired[float]
    """`I_xz`, kg·m².

    Absent means `0.0`.
    """

    yy_kg_m2: float
    """`I_yy`, kg·m²."""

    yz_kg_m2: NotRequired[float]
    """`I_yz`, kg·m².

    Absent means `0.0`.
    """

    zz_kg_m2: float
    """`I_zz`, about the rocket's axis, kg·m²."""


class InnerTube(TypedDict):
    """A tube inside the airframe: a coupler, a motor mount tube, an engine block or thrust ring. It
    may sit off the axis, and it may be a cluster: several like tubes side by side, as in a
    cluster's motor mount.

    **A cluster.** `Self::cluster_m` lists where each tube's axis sits, `[x, y]` in body axes
    from the axis the radial offset and angle give. The tubes are the one tube written here,
    repeated at each place: their mass is the sum of the copies, each with its own parallel-axis
    term. What the tube holds (an engine block, a motor) is repeated in every tube in the same way
    (`docs/physics/design.md`, the decision record on clusters, [ADR-075][adr-075]).

    [adr-075]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-075-a-cluster-is-one-tube-repeated-and-a-motor-in-it-one-motor-per-tube-2026-09-25
    """

    angle_rad: NotRequired[float]
    """Roll angle of that offset from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    cluster_m: NotRequired[list[list[float]]]
    """A cluster's tubes: each tube's axis, `[x, y]` in body axes, m, measured from the axis the
    radial offset and angle give. Empty (the default) for one tube on that axis.
    """

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (bulk)."""

    outer_radius_m: float
    """Outer radius, m."""

    radial_offset_m: NotRequired[float]
    """Distance of the tube's axis from the body axis, m.

    Absent means `0.0`.
    """

    thickness_m: float
    """Wall thickness, m."""


class Kept(TypedDict):
    """An element kept whole, and where it was."""

    at: str
    """Its path in the document; see `element_at`."""

    element: "Element"
    """The element, with everything inside it."""


class KeptAttribute(TypedDict):
    """An attribute kept, and the element it was on."""

    at: str
    """The path of the element it was on; see `element_at`."""

    name: str
    """Its name."""

    value: str
    """Its value, as written."""


class LaunchConditions(TypedDict):
    """The launch conditions a stored simulation was flown in. Each is `None` where the file does not
    say.
    """

    atmosphere: NotRequired[Union["Atmosphere", None]]
    """`<atmosphere>`."""

    configuration: NotRequired[Union[str, None]]
    """`<configid>`: the motor configuration flown."""

    geodetic_method: NotRequired[Union[str, None]]
    """`<geodeticmethod>`: `flat`, `spherical` or `wgs84`."""

    into_wind: NotRequired[Union[bool, None]]
    """`<launchintowind>`: whether the rod is pointed into the wind, which OpenRocket then writes
    as the rod's direction.
    """

    latitude_deg: NotRequired[Union[float, None]]
    """`<launchlatitude>`, degrees north."""

    launch_altitude_m: NotRequired[Union[float, None]]
    """`<launchaltitude>`: the launch site's height above sea level, m."""

    longitude_deg: NotRequired[Union[float, None]]
    """`<launchlongitude>`, degrees east."""

    max_time_s: NotRequired[Union[float, None]]
    """`<maxtime>`, s."""

    rod_angle_rad: NotRequired[Union[float, None]]
    """`<launchrodangle>`: the rod's tilt from vertical, rad (the file writes degrees)."""

    rod_direction_rad: NotRequired[Union[float, None]]
    """`<launchroddirection>`: the compass bearing the rod tilts toward, clockwise from north, rad
    (the file writes degrees).
    """

    rod_length_m: NotRequired[Union[float, None]]
    """`<launchrodlength>`, m."""

    time_step_s: NotRequired[Union[float, None]]
    """`<timestep>`, s."""

    wind_from_rad: NotRequired[Union[float, None]]
    """`<winddirection>`, or the average wind's `<direction>`: the compass bearing the wind blows
    from, rad (the file writes radians).
    """

    wind_levels: list["WindLevel"]
    """`<wind model="multilevel">`'s levels, lowest first as written."""

    wind_levels_above: NotRequired[Union[str, None]]
    """The multilevel wind's `altituderef`: whether its altitudes are above the ground (`agl`) or
    the sea (`msl`).
    """

    wind_model: NotRequired[Union[str, None]]
    """`<windmodeltype>`: which wind the run flew, `Average` or the multilevel one. OpenRocket
    writes both winds whichever it flew.
    """

    wind_speed_m_s: NotRequired[Union[float, None]]
    """`<windaverage>`, or the average wind's `<speed>`: the mean wind speed, m/s."""

    wind_turbulence: NotRequired[Union[float, None]]
    """`<windturbulence>`: the turbulence intensity, the standard deviation of the wind speed over
    its mean.
    """


class LaunchLug(TypedDict):
    """A launch lug: a tube on the outside of the airframe, parallel to it."""

    angle_rad: NotRequired[float]
    """Roll angle from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    count: NotRequired[int]
    """Number of lugs in a row.

    Absent means `1`.
    """

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (bulk)."""

    outer_radius_m: float
    """Outer radius, m."""

    spacing_m: NotRequired[float]
    """Axial distance between the forward ends of consecutive lugs, m.

    Absent means `0.0`.
    """

    thickness_m: float
    """Wall thickness, m."""


class LeftOut(TypedDict):
    """Why a configuration is not among the rocket's."""

    message: str
    """The same, in words, naming the motor."""

    why: "NotFlown"
    """The reason."""


class MassComponent(TypedDict):
    """A mass of known value: an altimeter bay, ballast, a payload."""

    mass_kg: float
    """Mass, kg."""

    packing: "Packing"
    """Its extent."""


class MassElement(TypedDict):
    """A mass on the motor axis, with its moments of inertia about its own centre of mass."""

    axial_inertia_kg_m2: float
    """Moment of inertia about the motor axis, through the element's centre of mass, kg·m²."""

    cg_m: float
    """Centre of mass along the motor axis, m from the nozzle exit toward the forward end."""

    mass_kg: float
    """Mass, kg."""

    transverse_inertia_kg_m2: float
    """Moment of inertia about a transverse axis through the element's centre of mass, kg·m²."""


class Material(TypedDict):
    """A named material."""

    density: "Density"
    """Density."""

    name: str
    """Name, for display and for matching imported designs."""


class MotorConfiguration(TypedDict):
    """A motor configuration: what the rocket declares, and every motor the mounts put in it."""

    declared: bool
    """Whether `<rocket>` declares it; one only a mount names is read all the same."""

    default: bool
    """Whether the file marks it `default="true"`."""

    id: str
    """`configid`."""

    inactive_stages: list[Union[int, None]]
    """The `<stage number>`s the configuration marks `active="false"`; `None` for one whose number
    is missing or not a count, which is switched off all the same.
    """

    left_out: NotRequired[Union["LeftOut", None]]
    """Why it is not among the rocket's configurations, or `None` when it is."""

    motors: list["OrkMotor"]
    """Its motors, in the order their mounts appear in the file."""

    name: str
    """`<name>`, which may be empty."""

    staging: NotRequired[Union["Staging", None]]
    """The powered separation it flies, for one among the rocket's configurations that has one
    (`staging`). The rocket's configuration does not carry it: give it to the flight
    (`hpr::ork::separation` maps it onto one), with a recovery device on each part.
    """

    unread: list["UnreadMotor"]
    """Its motors in parts hpr does not read."""


class MotorMount(TypedDict):
    """Makes a body tube or inner tube a motor mount."""

    overhang_m: NotRequired[float]
    """How far the nozzle exit sits aft of the mount's aft end, m (negative when recessed).

    Absent means `0.0`.
    """


class Motors(TypedDict):
    """Every motor configuration a `.ork` design holds."""

    configurations: list["MotorConfiguration"]
    """The configurations: those `<rocket>` declares in its order, then any only a mount names."""


class MountedMotor(TypedDict):
    """A motor in a mount."""

    delay: NotRequired[Union["Delay", None]]
    """The ejection delay chosen, if any."""

    designation: NotRequired[str]
    """Designation, for display.

    Absent means `""`.
    """

    diameter_m: float
    """Case outer diameter, m."""

    failed_tubes: NotRequired[list[int]]
    """The tubes whose motor fails to light, by index into the mount's tubes (a cluster's in the
    order of `InnerTube::cluster_m`, `0` for a single tube; a
    cluster inside another cluster counts the outer copies first, each with all its tubes; a
    mount in a pod counts the pods, in the order of `PodSet::pods`): a
    motor out. Each is carried loaded and gives no thrust. An ignition on the mount's burnout
    takes its first motor that lights, but a recovery device or separation triggered by one
    motor's index waits on that motor alone: point it at a tube that lights, or it never fires.
    Empty (the default) when every motor lights.
    """

    ignition: NotRequired["Ignition"]
    """When it lights."""

    length_m: float
    """Case length, m."""

    motor: "SolidMotor"
    """The motor."""

    mount: str
    """The id of the mount component."""


NoCurve = Literal["hybrid", "no_designation", "not_found", "ambiguous", "unusable"]
"""Why a motor has no thrust curve."""


Node = Union["NodeElement", "NodeText"]
"""A child of an `Element`: another element, or text."""


class NodeElement(TypedDict):
    """A child element."""

    attributes: list[tuple[str, str]]
    """The attributes, in document order, as name and value."""

    children: list["Node"]
    """The children, in document order."""

    kind: Literal["element"]

    name: str
    """The element's name, such as `nosecone`."""


class NodeText(TypedDict):
    """Text. Character and entity references are already resolved."""

    kind: Literal["text"]

    text: str
    """The text as it stands."""


class NoseCone(TypedDict):
    """A nose cone: a profile with its tip forward, and an optional shoulder aft of its base."""

    base_radius_m: float
    """Base radius, m."""

    length_m: float
    """Length from tip to base, m."""

    material: "Material"
    """Material (bulk)."""

    shape: "NoseShape"
    """Profile shape."""

    shoulder: NotRequired[Union["Shoulder", None]]
    """Optional shoulder.

    Absent means `null`.
    """

    wall: "Wall"
    """Filled, or a wall of a thickness."""


NoseShape = Union[
    "NoseShapeConical",
    "NoseShapeOgive",
    "NoseShapeElliptical",
    "NoseShapePowerSeries",
    "NoseShapeParabolicSeries",
    "NoseShapeHaack",
]
"""The shape of a nose cone, or of a transition's profile."""


class NoseShapeConical(TypedDict):
    """A straight cone."""

    kind: Literal["conical"]


class NoseShapeOgive(TypedDict):
    """A circular-arc ogive whose arc radius is `radius_ratio` times the tangent-ogive radius."""

    kind: Literal["ogive"]

    radius_ratio: float
    """The arc radius over the tangent-ogive radius: 1 for a tangent ogive, above 1 for a
    secant ogive, below 1 (down to `R/L`) for a bulged secant ogive.
    """


class NoseShapeElliptical(TypedDict):
    """Half an ellipse: a blunt, rounded tip."""

    kind: Literal["elliptical"]


class NoseShapePowerSeries(TypedDict):
    """`g = ξⁿ`: `n = 1` is a cone and `n = ½` a paraboloid."""

    exponent: float
    """The exponent `n`, in `[0.05, 1]` (`MIN_POWER_EXPONENT`)."""

    kind: Literal["power_series"]


class NoseShapeParabolicSeries(TypedDict):
    """Parabolic series: `K′ = 0` is a cone and `K′ = 1` a full parabola, tangent at the base."""

    kind: Literal["parabolic_series"]

    parameter: float
    """The parameter `K′`, in `[0, 1]`."""


class NoseShapeHaack(TypedDict):
    """Haack series: `C = 0` is the von Kármán (LD-Haack) ogive and `C = 1/3` the LV-Haack."""

    kind: Literal["haack"]

    parameter: float
    """The parameter `C`, in `[0, 2/3]`."""


NotFlown = Literal[
    "unread_motor",
    "no_motor",
    "inactive_stage",
    "no_curve",
    "no_size",
    "ignition_not_flown",
    "airframe_not_as_written",
    "separation_not_flown",
]
"""The reasons a configuration cannot be flown as written, in the order they are checked; each is
checked across every motor before the next, so the one given is the first on this list that
applies.
"""


class Nozzle(TypedDict):
    """A nozzle, for the ambient-pressure correction of thrust."""

    exit_radius_m: float
    """Exit radius, m."""

    reference_pressure_pa: Union[float, None]
    """The ambient pressure the thrust curve was measured at, Pa: the static test site's. The
    thrust is then corrected to the ambient pressure in flight.

    `None` flies the curve as it is at every ambient pressure, with no correction. That is
    RocketPy's default (`Motor(reference_pressure=None)`, whose `pressure_thrust` is then zero,
    `motor.py:1188-1189`), so a RocketPy input transcribed into hpr says `None`.

    Which to give: motor files don't record where the curve was measured. For a motor tested
    near sea level, `STANDARD_SEA_LEVEL_PRESSURE_PA` adds the thrust a higher site gains
    (16 kPa × `A_e` at 1,400 m); `None` leaves it out. A design must say which: the field is
    required, as `null` for `None`, so leaving it out is an error rather than a silent choice.
    Motors read from `.eng` or `.rse` files, or from the catalog, carry no nozzle and so no
    correction.
    """

    throat_radius_m: NotRequired[Union[float, None]]
    """Throat radius, m, when known (informational: the thrust curve already carries its effect)."""


class OpenRocketExtension(TypedDict):
    """The `x-openrocket` extension: the parts and sections of a `.ork` that hpr does not read, each
    kept whole where it was.
    """

    attributes: NotRequired[list["KeptAttribute"]]
    """The attributes hpr does not read on an element it does read, such as a material's
    `group`, and those whose value a reader dropped, such as a material's declared `type` where
    the part needs another.

    Absent means `[]`.
    """

    parts: NotRequired[list["Kept"]]
    """The parts hpr does not read, in file order.

    Absent means `[]`.
    """

    sections: NotRequired[list["Kept"]]
    """The sections of the document hpr does not read, in file order.

    Absent means `[]`.
    """

    tags: NotRequired[list["Kept"]]
    """The tags hpr does not read in an element it does read — a part, a stage, the rocket, a
    stored simulation, or a tag inside any of those that a reader asked for — such as a part's
    `<appearance>`; and those a reader asked for and dropped or simplified, such as a rail
    button's `<screwheight>`, which the design does not hold.

    Absent means `[]`.
    """


class OrkIgnition(TypedDict):
    """When a motor ignites: an event and a delay after it."""

    delay_s: float
    """Seconds after the event, s."""

    event: "IgnitionEvent"
    """The event."""


class OrkMotor(TypedDict):
    """A motor in a mount, in one configuration: what the `<motor>` element says, when it ignites in
    that configuration, and the curve it flies on.
    """

    curve: "Curve"
    """The thrust curve."""

    delay: NotRequired[Union["Delay", None]]
    """`<delay>`: `none` is a plugged motor, with no ejection charge; a number is the seconds from
    burnout to the charge, and `0` fires it at burnout (OpenRocket's technical documentation,
    pages 8 and 10).
    """

    designation: str
    """`<designation>`, such as `H148R`."""

    diameter_m: NotRequired[Union[float, None]]
    """`<diameter>`: the case diameter, m."""

    digest: NotRequired[Union[str, None]]
    """`<digest>`: OpenRocket's key for the thrust curve, which names an embedded curve."""

    ignition: "OrkIgnition"
    """When it ignites in this configuration: the configuration's `<ignitionconfiguration>` where
    the mount has one, the mount's own default where it does not.
    """

    kind: NotRequired[Union[str, None]]
    """`<type>` as written: `single`, `reload` or `hybrid`."""

    length_m: NotRequired[Union[float, None]]
    """`<length>`: the case length, m."""

    manufacturer: str
    """`<manufacturer>`."""

    mount: str
    """The id of the mount component in the rocket."""

    stage: int
    """The index of the mount's stage in `Rocket::stages`."""


class Overrides(TypedDict):
    """Values that replace the mass properties computed from geometry. Each applies in turn:

    1. **Mass** `m′`: the body is rescaled, `I′ = I m′/m`, keeping its centre and shape. A body with
       no mass becomes a point mass at its centre, but for a packed part in a layout (a mass
       component, parachute, streamer or shock cord), which takes `m′` as a solid cylinder of its
       packing, as OpenRocket 24.12 does ([ADR-063][adr-063]).
    2. **Centre of mass**: the centre moves along the axis to `cg_aft_m` aft of the component's
       forward end (a stage's, for a stage), with or without its children, and off the axis to
       `cg_xy_m` when given (otherwise it keeps its offset); the tensor about the centre is
       unchanged.
    3. **Inertia**: the tensor about the (new) centre is replaced.

    [adr-063]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-063-packed-parts-read-and-weighed-as-openrocket-packs-them-2026-09-21
    """

    cg_aft_m: NotRequired[Union[float, None]]
    """Centre of mass, m aft of the forward end of the component (or the stage)."""

    cg_xy_m: NotRequired[Union[list[float], None]]
    """Centre of mass off the axis, `[x, y]` in body axes, m. For a part inside a cluster's tube or
    a pod, it is measured in that one copy as written (a pod on the body's axis), and the copies
    carry it to each place.
    """

    inertia: NotRequired[Union["InertiaOverride", None]]
    """Inertia tensor about the centre of mass. For a part inside a cluster's tube or a pod, it is
    in that one copy's axes as written, and turns with each pod.
    """

    mass_kg: NotRequired[Union[float, None]]
    """Mass, kg."""


class Packing(TypedDict):
    """Where and how compactly a mass or a recovery part is stowed: a solid cylinder."""

    angle_rad: NotRequired[float]
    """Roll angle of that offset from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    length_m: float
    """Length, m."""

    radial_offset_m: NotRequired[float]
    """Distance of its axis from the body axis, m.

    Absent means `0.0`.
    """

    radius_m: float
    """Radius, m."""


class Parachute(TypedDict):
    """A parachute, packed."""

    canopy_material: "Material"
    """Canopy fabric (surface)."""

    diameter_m: float
    """Nominal (flat) canopy diameter, m."""

    line_count: int
    """Number of shroud lines."""

    line_length_m: float
    """Length of each shroud line, m."""

    line_material: "Material"
    """Shroud line (line)."""

    packing: "Packing"
    """How it is packed."""


Part = Union[
    "PartNoseCone",
    "PartBodyTube",
    "PartTransition",
    "PartInnerTube",
    "PartCenteringRing",
    "PartFinSet",
    "PartTubeFinSet",
    "PartLaunchLug",
    "PartRailButton",
    "PartPodSet",
    "PartMassComponent",
    "PartParachute",
    "PartStreamer",
    "PartShockCord",
]
"""A part in the tree. Serialized as an object with one key, the part's kind."""


class PartNoseCone(TypedDict):
    """A nose cone (body component)."""

    nose_cone: "NoseCone"


class PartBodyTube(TypedDict):
    """A body tube (body component)."""

    body_tube: "BodyTube"


class PartTransition(TypedDict):
    """A transition (body component)."""

    transition: "Transition"


class PartInnerTube(TypedDict):
    """An inner tube (internal): a coupler, motor mount tube or engine block."""

    inner_tube: "InnerTube"


class PartCenteringRing(TypedDict):
    """A centering ring or bulkhead (internal)."""

    centering_ring: "CenteringRing"


class PartFinSet(TypedDict):
    """A fin set (external, on a body tube). Its axial extent is the root chord."""

    fin_set: "FinSet"


class PartTubeFinSet(TypedDict):
    """Tube fins (external, on a body tube)."""

    tube_fin_set: "TubeFinSet"


class PartLaunchLug(TypedDict):
    """Launch lugs (external, on a body tube). The extent covers the whole row."""

    launch_lug: "LaunchLug"


class PartRailButton(TypedDict):
    """Rail buttons (external, on a body tube). The extent covers the whole row."""

    rail_button: "RailButton"


class PartPodSet(TypedDict):
    """Pods (external, on a body tube). Its children are the pod's body components, which stack
    along the pod's axis; its extent is theirs (`Component::length_m`).
    """

    pod_set: "PodSet"


class PartMassComponent(TypedDict):
    """A mass component (internal)."""

    mass_component: "MassComponent"


class PartParachute(TypedDict):
    """A parachute (internal)."""

    parachute: "Parachute"


class PartStreamer(TypedDict):
    """A streamer (internal)."""

    streamer: "Streamer"


class PartShockCord(TypedDict):
    """A shock cord (internal)."""

    shock_cord: "ShockCord"


class PodSet(TypedDict):
    """Pods beside the airframe: side pods, or outboard motor pods. A pod set attaches to a body tube
    like a fin set, and its children are the pod's own body components (nose cone, body tubes,
    transitions), which stack aft from the pod set's position along the pod's axis instead of the
    body's.

    **Copies.** The pod written in the tree is one pod on the body's axis, repeated `count` times
    around it as a rotational pattern: pod `k` is that pod turned by `φ_k = angle + 2π k / count`
    about the body's axis and moved to `r (cos φ_k, sin φ_k)` (`Self::pods`), in
    [body axes](https://github.com/nrdptel/hpr-sim/blob/main/docs/physics/frames.md). Everything
    the pod holds (fins on its tubes, parts inside them, a motor in a mount) turns and moves with
    it, so what points away from the airframe on one pod does on every pod. Each copy adds its own
    parallel-axis term, `I_O = I_cg + m (|d|² E − d dᵀ)` with `d` the copy's centre from the point
    `O` (J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*, appendix B). The pod set
    itself weighs nothing: its mass is its pods'. See the design page's *Pods* section
    (`docs/physics/design.md`) and the decision record on pods, [ADR-089][adr-089].

    **A pod of no length.** A pod may be a single body tube of no length, which weighs nothing: what
    hangs from it (fins, a launch lug) sits on a tube of that radius, most often none, so on the
    pod's own axis, and is repeated around the body's as any pod is. OpenRocket draws winglets this
    way, calling the tube a "phantom body". A pod set may also hold nothing at all, and then weighs
    nothing.

    **Flown** since
    [M1.13c1](https://github.com/nrdptel/hpr-sim/blob/main/docs/decisions-and-roadmap.md#m1-13c1):
    each pod's parts take their own normal force and drag, once per pod
    ([aerodynamics: Pods](https://nrdptel.github.io/hpr-sim/physics/aero.html#pods)).

    [adr-089]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-089-a-pod-is-a-stack-of-body-components-repeated-around-the-axis-2026-09-27
    """

    angle_rad: NotRequired[float]
    """Roll angle of the first pod from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    count: int
    """Number of pods, at least one, spaced evenly around the body's axis."""

    radial_offset_m: float
    """Distance of each pod's axis from the body's axis, m."""


Position = Union[
    "PositionTop",
    "PositionMiddle",
    "PositionBottom",
    "PositionAfter",
    "PositionAbsolute",
]
"""Where an attached part sits along its parent. Offsets are positive aft."""


# The part's forward end is `aft_offset_m` aft of the parent's forward end.
PositionTop = TypedDict(
    "PositionTop",
    {
        # Offset, m.
        #
        # Absent means `0.0`.
        "aft_offset_m": NotRequired[float],
        "from": Literal["top"],
    },
)


# The part's middle is `aft_offset_m` aft of the parent's middle.
PositionMiddle = TypedDict(
    "PositionMiddle",
    {
        # Offset, m.
        #
        # Absent means `0.0`.
        "aft_offset_m": NotRequired[float],
        "from": Literal["middle"],
    },
)


# The part's aft end is `aft_offset_m` aft of the parent's aft end.
PositionBottom = TypedDict(
    "PositionBottom",
    {
        # Offset, m.
        #
        # Absent means `0.0`.
        "aft_offset_m": NotRequired[float],
        "from": Literal["bottom"],
    },
)


# The part's forward end is `aft_offset_m` aft of the previous sibling's aft end, or of the
# parent's forward end for the first child.
PositionAfter = TypedDict(
    "PositionAfter",
    {
        # Offset, m.
        #
        # Absent means `0.0`.
        "aft_offset_m": NotRequired[float],
        "from": Literal["after"],
    },
)


# The part's forward end is at station `station_m`, measured aft of the nose tip.
PositionAbsolute = TypedDict(
    "PositionAbsolute",
    {
        "from": Literal["absolute"],
        # Station, m.
        "station_m": float,
    },
)


Propellant = Union["PropellantColumn", "PropellantGrains"]
"""How the propellant is laid out and how its shape evolves. Serialized with a `model` tag
(`"column"`, `"grains"`).

Not `Copy`, so that a later model can hold tabulated data.
"""


class PropellantColumn(TypedDict):
    """A fixed-shape column: the default when only a motor's envelope is known."""

    center_m: float
    """Centre of the column along the motor axis, m from the nozzle exit."""

    inner_radius_m: float
    """Inner (bore) radius, m; zero for a solid column."""

    length_m: float
    """Length, m."""

    mass_kg: float
    """Initial propellant mass, kg."""

    model: Literal["column"]

    outer_radius_m: float
    """Outer radius, m."""


class PropellantGrains(TypedDict):
    """BATES grains that regress on their bores and ends."""

    center_m: float
    """Centre of the grain stack along the motor axis, m from the nozzle exit."""

    count: int
    """Number of grains `N`, at least 1."""

    density_kg_m3: float
    """Propellant density `ρ`, kg/m³."""

    inhibited_ends: bool
    """Whether the grain ends are inhibited, so only the bores burn (RocketPy's
    `only_radial_burn`).
    """

    initial_height_m: float
    """Initial grain height (length) `h₀`, m."""

    initial_inner_radius_m: float
    """Initial bore radius `r₀`, m, positive."""

    model: Literal["grains"]

    outer_radius_m: float
    """Grain outer radius `R`, m."""

    separation_m: float
    """Gap between adjacent grains `s`, m."""


class Provenance(TypedDict):
    """Which program wrote a document, and from what."""

    source: NotRequired[Union["Source", None]]
    """The file the design was read from, if it was read from one."""

    tool: str
    """The program, such as `hpr-sim`."""

    tool_version: str
    """Its version."""


class RailButton(TypedDict):
    """A rail button: a base disc on the airframe, a narrower waist, and a flange that rides in the
    rail, stacked outward along a radial line.
    """

    angle_rad: NotRequired[float]
    """Roll angle from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    base_height_m: float
    """Height of the base, m."""

    count: NotRequired[int]
    """Number of buttons in a row.

    Absent means `1`.
    """

    flange_height_m: float
    """Height of the flange, m."""

    height_m: float
    """Total height above the airframe, m."""

    inner_diameter_m: float
    """Diameter of the waist, m."""

    material: "Material"
    """Material (bulk)."""

    outer_diameter_m: float
    """Diameter of the base and the flange, m."""

    spacing_m: NotRequired[float]
    """Axial distance between the forward edges of consecutive buttons, m.

    Absent means `0.0`.
    """


class Recovery(TypedDict):
    """Every recovery setting in a `.ork` design."""

    devices: list["RecoveryDevice"]
    """The parachutes and streamers read, in file order."""

    separations: list["StageSeparation"]
    """Every stage that states when it separates, in file order."""

    unread: list["UnreadDevice"]
    """The parachutes and streamers in parts hpr does not read."""

    unread_separations: list["UnreadDevice"]
    """The parallel stages that state a separation, which hpr does not read yet."""


class RecoveryDevice(TypedDict):
    """A parachute's or streamer's recovery settings."""

    cd: NotRequired[Union["Dimension", None]]
    """`<cd>`: a stated drag coefficient, or `auto` for OpenRocket's own (0.8 for a parachute,
    from the strip's size for a streamer). `None` where the file says nothing.
    """

    configurations: dict[str, "EventSetting_for_DeployEvent"]
    """When it deploys in each configuration that changes that, by `configid`, with anything the
    configuration leaves out taken from `RecoveryDevice::deployment`.
    """

    deployment: "EventSetting_for_DeployEvent"
    """When it deploys, as the device states it."""

    id: str
    """The id of the device's component in the rocket."""

    kind: "DeviceKind"
    """Parachute or streamer."""

    stage: int
    """The index of its stage in `Rocket::stages`."""


ReferenceDiameter = Union[
    "ReferenceDiameterMaximum",
    "ReferenceDiameterNoseBase",
    "ReferenceDiameterCustom",
]
"""How the reference diameter (for aerodynamic coefficients) is chosen."""


class ReferenceDiameterMaximum(TypedDict):
    """The widest body component (nose cone, body tube or transition) in any stage. Internal
    parts, shoulders, fins, tube fins, lugs and rail buttons don't count.
    """

    kind: Literal["maximum"]


class ReferenceDiameterNoseBase(TypedDict):
    """The base of the first nose cone."""

    kind: Literal["nose_base"]


class ReferenceDiameterCustom(TypedDict):
    """A given diameter."""

    diameter_m: float
    """Diameter, m."""

    kind: Literal["custom"]


class Rocket(TypedDict):
    """A rocket design: its stages, how its reference diameter is chosen, and its motor
    configurations.
    """

    configurations: NotRequired[list["Configuration"]]
    """Motor configurations.

    Absent means `[]`.
    """

    name: NotRequired[str]
    """Name.

    Absent means `""`.
    """

    reference_diameter: NotRequired["ReferenceDiameter"]
    """How the reference diameter is chosen.

    Absent means `{"kind":"maximum"}`.
    """

    stages: list["Stage"]
    """Stages from the nose aft. The first holds the nose cone."""


SeparationEvent = Union[
    Literal["launch"],
    Literal["ignition"],
    Literal["burnout"],
    Literal["ejection"],
    Literal["upper_ignition"],
    Literal["altitude_ascending"],
    Literal["apogee"],
    Literal["altitude_descending"],
    Literal["never"],
    "SeparationEventOther",
]
"""What separates a stage from the one above it, as `<separationevent>` names it. "This stage" is
the stage that carries the setting, the lower one, which drops away.
"""


class SeparationEventOther(TypedDict):
    """A word not listed here, kept as written."""

    other: str


class ShockCord(TypedDict):
    """A shock cord, packed."""

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (line)."""

    packing: "Packing"
    """How it is packed."""


class Shoulder(TypedDict):
    """A cylindrical extension of a nose cone or transition that fits inside the adjoining tube."""

    capped: NotRequired[bool]
    """Whether a disc closes the shoulder's far end.

    Absent means `false`.
    """

    length_m: float
    """Length, m."""

    outer_radius_m: float
    """Outer radius, m."""

    thickness_m: float
    """Wall thickness, m."""


class SolidMotor(TypedDict):
    """A solid rocket motor.

    Nothing here can tell a hybrid's thrust curve from a solid's, so the checks are at the edges:
    `crate::catalog::CatalogMotor::motor` refuses hybrids and the `.rse` reader warns about them
    (`.eng` files don't say).
    """

    curve: "ThrustCurve"

    dry: "MassElement"

    nozzle: NotRequired[Union["Nozzle", None]]

    propellant: "Propellant"


class Source(TypedDict):
    """The file a design was read from: its format and its SHA-256, which name it without its path,
    and whether its rocket was read exactly as written.
    """

    airframe_not_as_written: NotRequired[Union[str, None]]
    """Why the file's airframe or a motor mount was not read exactly as written, if it wasn't: a
    part left out, a value dropped or simplified, or something assumed
    (`hpr_io::ork::airframe_not_as_written`). No configuration of such a rocket flies, and
    `hpr sim` flies no other motor in it. Absent when the rocket was read as written. A document
    migrated from 0.1 that doesn't show which holds exactly `migrate::UNKNOWN`, a fixed text a
    program can compare against.
    """

    format: "SourceFormat"
    """The file's format."""

    sha256: str
    """The SHA-256 of the file's bytes, as 64 lowercase hexadecimal digits."""


class SourceFile(TypedDict):
    """One of the source file's other files: its name, and its contents as text where they are UTF-8,
    or else as base64 ([RFC 4648][rfc-4648], section 4).

    Beyond the schema, the reader holds a document's source files to four rules: base64 decodes; no
    two share a name; each thrust curve a motor names as embedded is among them; and a name is not
    empty, not `rocket.ork` (the design's own entry) and not a directory's, ending in `/` or `\\`.

    [rfc-4648]: https://www.rfc-editor.org/rfc/rfc4648#section-4
    """

    content: "Content"
    """Its contents."""

    name: str
    """Its name in the source file, such as `thrustcurves/<digest>.rse`."""


SourceFormat = Literal["ork", "hpr_design"]
"""The formats a design can be read from."""


class Stage(TypedDict):
    """A stage: body components stacked from its forward end aft."""

    components: list["Component"]
    """Body components (nose cones, body tubes, transitions), forward to aft."""

    id: str
    """Unique id."""

    name: NotRequired[str]
    """Name.

    Absent means `""`.
    """

    overrides: NotRequired["Overrides"]
    """Overrides for the whole stage without its motors, applied after every override inside it. A
    centre-of-mass override is measured aft of the stage's forward end.
    """


class StageSeparation(TypedDict):
    """A stage's separation settings."""

    configurations: dict[str, "EventSetting_for_SeparationEvent"]
    """When it separates in each configuration that changes that, by `configid`, with anything
    the configuration leaves out taken from `StageSeparation::separation`.
    """

    id: str
    """The stage's id in the rocket."""

    separation: "EventSetting_for_SeparationEvent"
    """When it separates, as the stage states it."""

    stage: int
    """The stage's index in `Rocket::stages`."""


class Staging(TypedDict):
    """The one powered separation a configuration flies: where the stack comes apart and when."""

    after_stage: int
    """The last stage that stays with the nose; the stages after it drop away."""

    time_s: float
    """When that is, s after launch: known before the flight."""

    trigger: "StagingTrigger"
    """When."""


StagingTrigger = Union["StagingTriggerTime", "StagingTriggerBurnout"]
"""When a powered separation fires, in the terms of hpr's flight triggers
(`hpr_sim::recovery::Trigger`, which this crate does not depend on).
"""


class StagingTriggerTime(TypedDict):
    """At a time after launch, s."""

    time: "StagingTriggerTimeTime"


class StagingTriggerTimeTime(TypedDict):

    time_s: float
    """The time after launch, s."""


class StagingTriggerBurnout(TypedDict):
    """A delay after the burnout of the motor in a mount (the first tube's, for a cluster, whose
    tubes light together).
    """

    burnout: "StagingTriggerBurnoutBurnout"


class StagingTriggerBurnoutBurnout(TypedDict):

    delay_s: float
    """The delay after its burnout, s."""

    mount: str
    """The mount's component id."""


class StoredBranch(TypedDict):
    """One stage's stored time series."""

    events: list["StoredEvent"]
    """The `<event>`s, in file order."""

    name: str
    """`name`: the stage's name."""

    rows: list[list[Union[float, None]]]
    """The `<datapoint>` rows, each a value per column, as written: SI, angles in radians, latitude
    and longitude in degrees. `None` where the file says `NaN`, a quantity OpenRocket did not
    compute at that step.
    """

    types: list[str]
    """`types`: each column's name as OpenRocket shows it, such as `Time` or `Altitude`."""


class StoredEvent(TypedDict):
    """An event a stored simulation logged."""

    kind: str
    """`type`, such as `apogee` or `recoverydevicedeployment`."""

    source: NotRequired[Union[str, None]]
    """`source`: the id of the component it came from, if any."""

    time_s: float
    """`time`, s."""


class StoredResults(TypedDict):
    """What a stored simulation gave: its summary and its time series."""

    branches: list["StoredBranch"]
    """The time series, one per stage, in file order."""

    deployment_speed_m_s: NotRequired[Union[float, None]]
    """`deploymentvelocity`: the speed at the first deployment, m/s."""

    flight_time_s: NotRequired[Union[float, None]]
    """`flighttime`, s."""

    ground_hit_speed_m_s: NotRequired[Union[float, None]]
    """`groundhitvelocity`, m/s."""

    max_acceleration_m_s2: NotRequired[Union[float, None]]
    """`maxacceleration`, m/s²."""

    max_altitude_m: NotRequired[Union[float, None]]
    """`maxaltitude`, m."""

    max_mach: NotRequired[Union[float, None]]
    """`maxmach`."""

    max_speed_m_s: NotRequired[Union[float, None]]
    """`maxvelocity`, m/s."""

    optimum_delay_s: NotRequired[Union[float, None]]
    """`optimumdelay`: the ejection delay that would have fired at apogee, s."""

    rod_exit_speed_m_s: NotRequired[Union[float, None]]
    """`launchrodvelocity`: the speed leaving the rod, m/s."""

    time_to_apogee_s: NotRequired[Union[float, None]]
    """`timetoapogee`, s."""

    warnings: list[str]
    """The `<warning>`s OpenRocket stored with the results, each as its text."""


class StoredSimulation(TypedDict):
    """A simulation stored in a `.ork`."""

    calculator: NotRequired[Union[str, None]]
    """`<calculator>`, such as `BarrowmanCalculator`."""

    conditions: NotRequired[Union["LaunchConditions", None]]
    """`<conditions>`: what the run was flown in."""

    name: str
    """`<name>`."""

    parser_warnings: NotRequired[bool]
    """Whether reading this simulation required dropping or reinterpreting stored data.

    Absent means `false`.
    """

    results: NotRequired[Union["StoredResults", None]]
    """`<flightdata>`: what it gave, when OpenRocket saved it."""

    simulator: NotRequired[Union[str, None]]
    """`<simulator>`, such as `RK4Simulator`."""

    status: NotRequired[Union[str, None]]
    """The `status` attribute as written: `uptodate`, `outdated`, `loaded`, `external`,
    `notsimulated`, `cantrun` or `aborted` in the files OpenRocket 24.12 writes.
    """


class Streamer(TypedDict):
    """A streamer, packed."""

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (surface)."""

    packing: "Packing"
    """How it is packed."""

    width_m: float
    """Width, m."""


class ThrustCurve(TypedDict):
    """A thrust curve: thrust in newtons against time in seconds from ignition, joined by straight
    lines.

    Built through `ThrustCurve::new`, which checks the samples; it serializes as its samples and
    re-checks them when deserialized.
    """

    thrusts_n: list[float]

    times_s: list[float]


class Transition(TypedDict):
    """A transition between two radii, with optional shoulders at either end."""

    aft_radius_m: float
    """Radius at the aft end, m."""

    aft_shoulder: NotRequired[Union["Shoulder", None]]
    """Optional shoulder aft of the aft end.

    Absent means `null`.
    """

    clipped: NotRequired[bool]
    """Whether the profile is clipped from a longer nose cone (`crate::shapes`).

    Absent means `false`.
    """

    fore_radius_m: float
    """Radius at the forward end, m."""

    fore_shoulder: NotRequired[Union["Shoulder", None]]
    """Optional shoulder forward of the fore end.

    Absent means `null`.
    """

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (bulk)."""

    shape: "NoseShape"
    """Profile shape."""

    wall: "Wall"
    """Filled, or a wall of a thickness."""


class TubeFinSet(TypedDict):
    """Tube fins: open tubes parallel to the body, touching it, spaced evenly around it."""

    base_angle_rad: NotRequired[float]
    """Roll angle of the first tube's axis from `x_B` toward `y_B`, rad.

    Absent means `0.0`.
    """

    count: int
    """Number of tubes, at least 1."""

    length_m: float
    """Length, m."""

    material: "Material"
    """Material (bulk)."""

    outer_radius_m: float
    """Outer radius, m."""

    thickness_m: float
    """Wall thickness, m."""


class UnreadDevice(TypedDict):
    """A parachute or streamer inside a part hpr does not read, such as a pod."""

    at: str
    """Where it is in the file."""

    inside: str
    """The tag of the outermost part that was not read: a `podset` or `parallelstage`, or else
    the device's own tag.
    """

    tag: str
    """`parachute` or `streamer`."""


class UnreadMotor(TypedDict):
    """A `<motor>` inside a part hpr does not read, such as a pod's mount."""

    at: str
    """Where its mount is in the file."""

    designation: str
    """`<designation>`."""

    inside: str
    """The tag of the part that was not read: the outermost `podset` or `parallelstage` around
    the mount, or else the mount's own tag.
    """

    reason: str
    """Why its mount was not read, in words."""


Version = str
"""A version of the format, `major.minor`."""


Wall = Union["WallFilled", "WallShell"]
"""Whether a solid of revolution is filled or a wall."""


class WallFilled(TypedDict):
    """Solid all the way to the axis."""

    kind: Literal["filled"]


class WallShell(TypedDict):
    """A wall of constant thickness measured normal to the outer surface. A thickness of zero is a
    surface with no wall: the part keeps its shape and weighs nothing, which is what OpenRocket
    makes of a part written with no wall ([ADR-061][adr-061]).

    [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
    """

    kind: Literal["shell"]

    thickness_m: float
    """Wall thickness, m; zero or more."""


class WindLevel(TypedDict):
    """One level of a multilevel wind."""

    altitude_m: NotRequired[Union[float, None]]
    """Its altitude, m, above the ground or the sea as `LaunchConditions::wind_levels_above`
    says.
    """

    from_rad: NotRequired[Union[float, None]]
    """The bearing it blows from, rad."""

    speed_m_s: NotRequired[Union[float, None]]
    """Its mean speed, m/s."""

    standard_deviation_m_s: NotRequired[Union[float, None]]
    """The standard deviation of its speed, m/s."""


# The schema the reader checks a document against: the committed schema without its prose.
_SCHEMA: Any = json.loads(
    r"""{"$defs":{"Atmosphere":{"oneOf":[{"additionalProperties":false,"properties":{"model":{"const":"isa","type":"string"}},"required":["model"],"type":"object"},{"additionalProperties":false,"properties":{"model":{"const":"extended","type":"string"},"pressure_pa":{"type":["number","null"]},"temperature_k":{"type":["number","null"]}},"required":["model"],"type":"object"},{"additionalProperties":false,"properties":{"model":{"const":"other","type":"string"},"name":{"type":"string"}},"required":["model","name"],"type":"object"}]},"AutoDimension":{"oneOf":[{"const":"base_radius","type":"string"},{"const":"outer_radius","type":"string"},{"const":"fore_radius","type":"string"},{"const":"aft_radius","type":"string"},{"const":"shoulder_radius","type":"string"},{"const":"fore_shoulder_radius","type":"string"},{"const":"aft_shoulder_radius","type":"string"},{"const":"inner_radius","type":"string"},{"const":"packed_radius","type":"string"}]},"BodyTube":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"CenteringRing":{"additionalProperties":false,"properties":{"inner_radius_m":{"type":"number"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"}},"required":["length_m","outer_radius_m","inner_radius_m","material"],"type":"object"},"Component":{"additionalProperties":false,"properties":{"auto":{"items":{"$ref":"#/$defs/AutoDimension"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Component"},"type":"array"},"finish":{"anyOf":[{"$ref":"#/$defs/Finish"},{"type":"null"}]},"id":{"type":"string"},"motor_mount":{"anyOf":[{"$ref":"#/$defs/MotorMount"},{"type":"null"}]},"name":{"type":"string"},"overrides":{"$ref":"#/$defs/Overrides"},"overrides_include_children":{"type":"boolean"},"part":{"$ref":"#/$defs/Part"},"position":{"anyOf":[{"$ref":"#/$defs/Position"},{"type":"null"}]}},"required":["id","part"],"type":"object"},"Configuration":{"additionalProperties":false,"properties":{"id":{"type":"string"},"motors":{"items":{"$ref":"#/$defs/MountedMotor"},"type":"array"},"name":{"type":"string"}},"required":["id","motors"],"type":"object"},"Content":{"oneOf":[{"additionalProperties":false,"properties":{"text":{"type":"string"}},"required":["text"],"type":"object"},{"additionalProperties":false,"properties":{"base64":{"type":"string"}},"required":["base64"],"type":"object"}]},"Curve":{"oneOf":[{"additionalProperties":false,"properties":{"entry":{"type":"string"},"motor":{"$ref":"#/$defs/SolidMotor"},"source":{"const":"embedded","type":"string"}},"required":["source","entry","motor"],"type":"object"},{"additionalProperties":false,"properties":{"digest":{"type":"string"},"from":{"type":"string"},"motor":{"$ref":"#/$defs/SolidMotor"},"source":{"const":"supplied","type":"string"}},"required":["source","digest","from","motor"],"type":"object"},{"additionalProperties":false,"properties":{"motor":{"$ref":"#/$defs/SolidMotor"},"motor_id":{"type":"string"},"simfile_id":{"type":"string"},"source":{"const":"catalog","type":"string"}},"required":["source","motor_id","simfile_id","motor"],"type":"object"},{"additionalProperties":false,"properties":{"reason":{"type":"string"},"source":{"const":"unresolved","type":"string"},"why":{"$ref":"#/$defs/NoCurve"}},"required":["source","why","reason"],"type":"object"}]},"Delay":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"seconds","type":"string"},"value":{"type":"number"}},"required":["kind","value"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"plugged","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"zero_or_plugged","type":"string"}},"required":["kind"],"type":"object"}]},"Density":{"oneOf":[{"additionalProperties":false,"properties":{"kg_m3":{"type":"number"},"kind":{"const":"bulk","type":"string"}},"required":["kind","kg_m3"],"type":"object"},{"additionalProperties":false,"properties":{"kg_m2":{"type":"number"},"kind":{"const":"surface","type":"string"}},"required":["kind","kg_m2"],"type":"object"},{"additionalProperties":false,"properties":{"kg_m":{"type":"number"},"kind":{"const":"line","type":"string"}},"required":["kind","kg_m"],"type":"object"}]},"DeployEvent":{"oneOf":[{"const":"launch","type":"string"},{"const":"ejection","type":"string"},{"const":"apogee","type":"string"},{"const":"altitude","type":"string"},{"const":"lower_stage_separation","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"DeviceKind":{"oneOf":[{"const":"parachute","type":"string"},{"const":"streamer","type":"string"}]},"Dimension":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"stated","type":"string"},"value":{"type":"number"}},"required":["kind","value"],"type":"object"},{"additionalProperties":false,"properties":{"cached":{"type":["number","null"]},"kind":{"const":"automatic","type":"string"}},"required":["kind"],"type":"object"}]},"Element":{"additionalProperties":false,"properties":{"attributes":{"items":{"maxItems":2,"minItems":2,"prefixItems":[{"type":"string"},{"type":"string"}],"type":"array"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Node"},"type":"array"},"name":{"type":"string"}},"required":["name","attributes","children"],"type":"object"},"EventSetting_for_DeployEvent":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"delay_s":{"type":["number","null"]},"event":{"anyOf":[{"$ref":"#/$defs/DeployEvent"},{"type":"null"}]}},"type":"object"},"EventSetting_for_SeparationEvent":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"delay_s":{"type":["number","null"]},"event":{"anyOf":[{"$ref":"#/$defs/SeparationEvent"},{"type":"null"}]}},"type":"object"},"Extensions":{"additionalProperties":false,"properties":{"x-openrocket":{"$ref":"#/$defs/OpenRocketExtension"}},"type":"object"},"FinCrossSection":{"oneOf":[{"const":"square","type":"string"},{"const":"rounded","type":"string"},{"const":"airfoil","type":"string"}]},"FinFillet":{"additionalProperties":false,"properties":{"material":{"$ref":"#/$defs/Material"},"radius_m":{"type":"number"}},"required":["radius_m","material"],"type":"object"},"FinPlanform":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"trapezoidal","type":"string"},"root_chord_m":{"type":"number"},"span_m":{"type":"number"},"sweep_m":{"type":"number"},"tip_chord_m":{"type":"number"}},"required":["kind","root_chord_m","tip_chord_m","span_m","sweep_m"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"elliptical","type":"string"},"root_chord_m":{"type":"number"},"span_m":{"type":"number"}},"required":["kind","root_chord_m","span_m"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"freeform","type":"string"},"points_m":{"items":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":"array"},"type":"array"}},"required":["kind","points_m"],"type":"object"}]},"FinSet":{"additionalProperties":false,"properties":{"base_angle_rad":{"type":"number"},"cant_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"cross_section":{"$ref":"#/$defs/FinCrossSection"},"fillet":{"anyOf":[{"$ref":"#/$defs/FinFillet"},{"type":"null"}]},"material":{"$ref":"#/$defs/Material"},"planform":{"$ref":"#/$defs/FinPlanform"},"tab":{"anyOf":[{"$ref":"#/$defs/FinTab"},{"type":"null"}]},"thickness_m":{"type":"number"}},"required":["count","planform","thickness_m","material"],"type":"object"},"FinTab":{"additionalProperties":false,"properties":{"height_m":{"type":"number"},"length_m":{"type":"number"},"offset_m":{"type":"number"}},"required":["height_m","length_m","offset_m"],"type":"object"},"Finish":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"mirror","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"average_glass","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"polished","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"sheet_metal","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"optimum_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"planed_wood","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"mass_production_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"bare_steel","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"smooth_cement","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"asphalt_coating","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"dip_galvanized","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"poor_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"cast_iron","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"raw_wood","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"concrete","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"custom","type":"string"},"roughness_m":{"type":"number"}},"required":["kind","roughness_m"],"type":"object"}]},"Format":{"oneOf":[{"const":"hpr-design","type":"string"}]},"Ignition":{"oneOf":[{"const":"launch","type":"string"},{"additionalProperties":false,"properties":{"time":{"additionalProperties":false,"properties":{"time_s":{"type":"number"}},"required":["time_s"],"type":"object"}},"required":["time"],"type":"object"},{"additionalProperties":false,"properties":{"burnout":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"mount":{"type":"string"}},"required":["mount","delay_s"],"type":"object"}},"required":["burnout"],"type":"object"},{"additionalProperties":false,"properties":{"separation":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"}},"required":["delay_s"],"type":"object"}},"required":["separation"],"type":"object"},{"const":"never","type":"string"}]},"IgnitionEvent":{"oneOf":[{"const":"automatic","type":"string"},{"const":"launch","type":"string"},{"const":"ejection_charge","type":"string"},{"const":"burnout","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"InertiaOverride":{"additionalProperties":false,"properties":{"xx_kg_m2":{"type":"number"},"xy_kg_m2":{"type":"number"},"xz_kg_m2":{"type":"number"},"yy_kg_m2":{"type":"number"},"yz_kg_m2":{"type":"number"},"zz_kg_m2":{"type":"number"}},"required":["xx_kg_m2","yy_kg_m2","zz_kg_m2"],"type":"object"},"InnerTube":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"cluster_m":{"items":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":"array"},"type":"array"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"radial_offset_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"Kept":{"additionalProperties":false,"properties":{"at":{"type":"string"},"element":{"$ref":"#/$defs/Element"}},"required":["at","element"],"type":"object"},"KeptAttribute":{"additionalProperties":false,"properties":{"at":{"type":"string"},"name":{"type":"string"},"value":{"type":"string"}},"required":["at","name","value"],"type":"object"},"LaunchConditions":{"additionalProperties":false,"properties":{"atmosphere":{"anyOf":[{"$ref":"#/$defs/Atmosphere"},{"type":"null"}]},"configuration":{"type":["string","null"]},"geodetic_method":{"type":["string","null"]},"into_wind":{"type":["boolean","null"]},"latitude_deg":{"type":["number","null"]},"launch_altitude_m":{"type":["number","null"]},"longitude_deg":{"type":["number","null"]},"max_time_s":{"type":["number","null"]},"rod_angle_rad":{"type":["number","null"]},"rod_direction_rad":{"type":["number","null"]},"rod_length_m":{"type":["number","null"]},"time_step_s":{"type":["number","null"]},"wind_from_rad":{"type":["number","null"]},"wind_levels":{"items":{"$ref":"#/$defs/WindLevel"},"type":"array"},"wind_levels_above":{"type":["string","null"]},"wind_model":{"type":["string","null"]},"wind_speed_m_s":{"type":["number","null"]},"wind_turbulence":{"type":["number","null"]}},"required":["wind_levels"],"type":"object"},"LaunchLug":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"spacing_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"LeftOut":{"additionalProperties":false,"properties":{"message":{"type":"string"},"why":{"$ref":"#/$defs/NotFlown"}},"required":["why","message"],"type":"object"},"MassComponent":{"additionalProperties":false,"properties":{"mass_kg":{"type":"number"},"packing":{"$ref":"#/$defs/Packing"}},"required":["mass_kg","packing"],"type":"object"},"MassElement":{"additionalProperties":false,"properties":{"axial_inertia_kg_m2":{"type":"number"},"cg_m":{"type":"number"},"mass_kg":{"type":"number"},"transverse_inertia_kg_m2":{"type":"number"}},"required":["mass_kg","cg_m","axial_inertia_kg_m2","transverse_inertia_kg_m2"],"type":"object"},"Material":{"additionalProperties":false,"properties":{"density":{"$ref":"#/$defs/Density"},"name":{"type":"string"}},"required":["name","density"],"type":"object"},"MotorConfiguration":{"additionalProperties":false,"properties":{"declared":{"type":"boolean"},"default":{"type":"boolean"},"id":{"type":"string"},"inactive_stages":{"items":{"format":"uint32","minimum":0,"type":["integer","null"]},"type":"array"},"left_out":{"anyOf":[{"$ref":"#/$defs/LeftOut"},{"type":"null"}]},"motors":{"items":{"$ref":"#/$defs/OrkMotor"},"type":"array"},"name":{"type":"string"},"staging":{"anyOf":[{"$ref":"#/$defs/Staging"},{"type":"null"}]},"unread":{"items":{"$ref":"#/$defs/UnreadMotor"},"type":"array"}},"required":["id","name","default","declared","inactive_stages","motors","unread"],"type":"object"},"MotorMount":{"additionalProperties":false,"properties":{"overhang_m":{"type":"number"}},"type":"object"},"Motors":{"additionalProperties":false,"properties":{"configurations":{"items":{"$ref":"#/$defs/MotorConfiguration"},"type":"array"}},"required":["configurations"],"type":"object"},"MountedMotor":{"additionalProperties":false,"properties":{"delay":{"anyOf":[{"$ref":"#/$defs/Delay"},{"type":"null"}]},"designation":{"type":"string"},"diameter_m":{"type":"number"},"failed_tubes":{"items":{"format":"uint","minimum":0,"type":"integer"},"type":"array"},"ignition":{"$ref":"#/$defs/Ignition"},"length_m":{"type":"number"},"motor":{"$ref":"#/$defs/SolidMotor"},"mount":{"type":"string"}},"required":["mount","diameter_m","length_m","motor"],"type":"object"},"NoCurve":{"oneOf":[{"const":"hybrid","type":"string"},{"const":"no_designation","type":"string"},{"const":"not_found","type":"string"},{"const":"ambiguous","type":"string"},{"const":"unusable","type":"string"}]},"Node":{"oneOf":[{"additionalProperties":false,"properties":{"attributes":{"items":{"maxItems":2,"minItems":2,"prefixItems":[{"type":"string"},{"type":"string"}],"type":"array"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Node"},"type":"array"},"kind":{"const":"element","type":"string"},"name":{"type":"string"}},"required":["kind","name","attributes","children"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"text","type":"string"},"text":{"type":"string"}},"required":["kind","text"],"type":"object"}]},"NoseCone":{"additionalProperties":false,"properties":{"base_radius_m":{"type":"number"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"shape":{"$ref":"#/$defs/NoseShape"},"shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"wall":{"$ref":"#/$defs/Wall"}},"required":["shape","length_m","base_radius_m","wall","material"],"type":"object"},"NoseShape":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"conical","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"ogive","type":"string"},"radius_ratio":{"type":"number"}},"required":["kind","radius_ratio"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"elliptical","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"exponent":{"type":"number"},"kind":{"const":"power_series","type":"string"}},"required":["kind","exponent"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"parabolic_series","type":"string"},"parameter":{"type":"number"}},"required":["kind","parameter"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"haack","type":"string"},"parameter":{"type":"number"}},"required":["kind","parameter"],"type":"object"}]},"NotFlown":{"oneOf":[{"const":"unread_motor","type":"string"},{"const":"no_motor","type":"string"},{"const":"inactive_stage","type":"string"},{"const":"no_curve","type":"string"},{"const":"no_size","type":"string"},{"const":"ignition_not_flown","type":"string"},{"const":"airframe_not_as_written","type":"string"},{"const":"separation_not_flown","type":"string"}]},"Nozzle":{"additionalProperties":false,"properties":{"exit_radius_m":{"type":"number"},"reference_pressure_pa":{"type":["number","null"]},"throat_radius_m":{"type":["number","null"]}},"required":["exit_radius_m","reference_pressure_pa"],"type":"object"},"OpenRocketExtension":{"additionalProperties":false,"properties":{"attributes":{"items":{"$ref":"#/$defs/KeptAttribute"},"type":"array"},"parts":{"items":{"$ref":"#/$defs/Kept"},"type":"array"},"sections":{"items":{"$ref":"#/$defs/Kept"},"type":"array"},"tags":{"items":{"$ref":"#/$defs/Kept"},"type":"array"}},"type":"object"},"OrkIgnition":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"event":{"$ref":"#/$defs/IgnitionEvent"}},"required":["event","delay_s"],"type":"object"},"OrkMotor":{"additionalProperties":false,"properties":{"curve":{"$ref":"#/$defs/Curve"},"delay":{"anyOf":[{"$ref":"#/$defs/Delay"},{"type":"null"}]},"designation":{"type":"string"},"diameter_m":{"type":["number","null"]},"digest":{"type":["string","null"]},"ignition":{"$ref":"#/$defs/OrkIgnition"},"kind":{"type":["string","null"]},"length_m":{"type":["number","null"]},"manufacturer":{"type":"string"},"mount":{"type":"string"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["mount","stage","manufacturer","designation","ignition","curve"],"type":"object"},"Overrides":{"additionalProperties":false,"properties":{"cg_aft_m":{"type":["number","null"]},"cg_xy_m":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":["array","null"]},"inertia":{"anyOf":[{"$ref":"#/$defs/InertiaOverride"},{"type":"null"}]},"mass_kg":{"type":["number","null"]}},"type":"object"},"Packing":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"length_m":{"type":"number"},"radial_offset_m":{"type":"number"},"radius_m":{"type":"number"}},"required":["length_m","radius_m"],"type":"object"},"Parachute":{"additionalProperties":false,"properties":{"canopy_material":{"$ref":"#/$defs/Material"},"diameter_m":{"type":"number"},"line_count":{"format":"uint32","minimum":0,"type":"integer"},"line_length_m":{"type":"number"},"line_material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"}},"required":["diameter_m","canopy_material","line_count","line_length_m","line_material","packing"],"type":"object"},"Part":{"oneOf":[{"additionalProperties":false,"properties":{"nose_cone":{"$ref":"#/$defs/NoseCone"}},"required":["nose_cone"],"type":"object"},{"additionalProperties":false,"properties":{"body_tube":{"$ref":"#/$defs/BodyTube"}},"required":["body_tube"],"type":"object"},{"additionalProperties":false,"properties":{"transition":{"$ref":"#/$defs/Transition"}},"required":["transition"],"type":"object"},{"additionalProperties":false,"properties":{"inner_tube":{"$ref":"#/$defs/InnerTube"}},"required":["inner_tube"],"type":"object"},{"additionalProperties":false,"properties":{"centering_ring":{"$ref":"#/$defs/CenteringRing"}},"required":["centering_ring"],"type":"object"},{"additionalProperties":false,"properties":{"fin_set":{"$ref":"#/$defs/FinSet"}},"required":["fin_set"],"type":"object"},{"additionalProperties":false,"properties":{"tube_fin_set":{"$ref":"#/$defs/TubeFinSet"}},"required":["tube_fin_set"],"type":"object"},{"additionalProperties":false,"properties":{"launch_lug":{"$ref":"#/$defs/LaunchLug"}},"required":["launch_lug"],"type":"object"},{"additionalProperties":false,"properties":{"rail_button":{"$ref":"#/$defs/RailButton"}},"required":["rail_button"],"type":"object"},{"additionalProperties":false,"properties":{"pod_set":{"$ref":"#/$defs/PodSet"}},"required":["pod_set"],"type":"object"},{"additionalProperties":false,"properties":{"mass_component":{"$ref":"#/$defs/MassComponent"}},"required":["mass_component"],"type":"object"},{"additionalProperties":false,"properties":{"parachute":{"$ref":"#/$defs/Parachute"}},"required":["parachute"],"type":"object"},{"additionalProperties":false,"properties":{"streamer":{"$ref":"#/$defs/Streamer"}},"required":["streamer"],"type":"object"},{"additionalProperties":false,"properties":{"shock_cord":{"$ref":"#/$defs/ShockCord"}},"required":["shock_cord"],"type":"object"}]},"PodSet":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"radial_offset_m":{"type":"number"}},"required":["count","radial_offset_m"],"type":"object"},"Position":{"oneOf":[{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"top","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"middle","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"bottom","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"after","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"from":{"const":"absolute","type":"string"},"station_m":{"type":"number"}},"required":["from","station_m"],"type":"object"}]},"Propellant":{"oneOf":[{"additionalProperties":false,"properties":{"center_m":{"type":"number"},"inner_radius_m":{"type":"number"},"length_m":{"type":"number"},"mass_kg":{"type":"number"},"model":{"const":"column","type":"string"},"outer_radius_m":{"type":"number"}},"required":["model","mass_kg","center_m","outer_radius_m","inner_radius_m","length_m"],"type":"object"},{"additionalProperties":false,"properties":{"center_m":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"density_kg_m3":{"type":"number"},"inhibited_ends":{"type":"boolean"},"initial_height_m":{"type":"number"},"initial_inner_radius_m":{"type":"number"},"model":{"const":"grains","type":"string"},"outer_radius_m":{"type":"number"},"separation_m":{"type":"number"}},"required":["model","count","density_kg_m3","outer_radius_m","initial_inner_radius_m","initial_height_m","separation_m","center_m","inhibited_ends"],"type":"object"}]},"Provenance":{"additionalProperties":false,"properties":{"source":{"anyOf":[{"$ref":"#/$defs/Source"},{"type":"null"}]},"tool":{"type":"string"},"tool_version":{"type":"string"}},"required":["tool","tool_version"],"type":"object"},"RailButton":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"base_height_m":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"flange_height_m":{"type":"number"},"height_m":{"type":"number"},"inner_diameter_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_diameter_m":{"type":"number"},"spacing_m":{"type":"number"}},"required":["outer_diameter_m","inner_diameter_m","height_m","base_height_m","flange_height_m","material"],"type":"object"},"Recovery":{"additionalProperties":false,"properties":{"devices":{"items":{"$ref":"#/$defs/RecoveryDevice"},"type":"array"},"separations":{"items":{"$ref":"#/$defs/StageSeparation"},"type":"array"},"unread":{"items":{"$ref":"#/$defs/UnreadDevice"},"type":"array"},"unread_separations":{"items":{"$ref":"#/$defs/UnreadDevice"},"type":"array"}},"required":["devices","separations","unread","unread_separations"],"type":"object"},"RecoveryDevice":{"additionalProperties":false,"properties":{"cd":{"anyOf":[{"$ref":"#/$defs/Dimension"},{"type":"null"}]},"configurations":{"additionalProperties":{"$ref":"#/$defs/EventSetting_for_DeployEvent"},"type":"object"},"deployment":{"$ref":"#/$defs/EventSetting_for_DeployEvent"},"id":{"type":"string"},"kind":{"$ref":"#/$defs/DeviceKind"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["id","stage","kind","deployment","configurations"],"type":"object"},"ReferenceDiameter":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"maximum","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"nose_base","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"diameter_m":{"type":"number"},"kind":{"const":"custom","type":"string"}},"required":["kind","diameter_m"],"type":"object"}]},"Rocket":{"additionalProperties":false,"properties":{"configurations":{"items":{"$ref":"#/$defs/Configuration"},"type":"array"},"name":{"type":"string"},"reference_diameter":{"$ref":"#/$defs/ReferenceDiameter"},"stages":{"items":{"$ref":"#/$defs/Stage"},"type":"array"}},"required":["stages"],"type":"object"},"SeparationEvent":{"oneOf":[{"const":"launch","type":"string"},{"const":"ignition","type":"string"},{"const":"burnout","type":"string"},{"const":"ejection","type":"string"},{"const":"upper_ignition","type":"string"},{"const":"altitude_ascending","type":"string"},{"const":"apogee","type":"string"},{"const":"altitude_descending","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"ShockCord":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"}},"required":["length_m","material","packing"],"type":"object"},"Shoulder":{"additionalProperties":false,"properties":{"capped":{"type":"boolean"},"length_m":{"type":"number"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m"],"type":"object"},"SolidMotor":{"additionalProperties":false,"properties":{"curve":{"$ref":"#/$defs/ThrustCurve"},"dry":{"$ref":"#/$defs/MassElement"},"nozzle":{"anyOf":[{"$ref":"#/$defs/Nozzle"},{"type":"null"}]},"propellant":{"$ref":"#/$defs/Propellant"}},"required":["curve","propellant","dry"],"type":"object"},"Source":{"additionalProperties":false,"properties":{"airframe_not_as_written":{"type":["string","null"]},"format":{"$ref":"#/$defs/SourceFormat"},"sha256":{"pattern":"^[0-9a-f]{64}$","type":"string"}},"required":["format","sha256"],"type":"object"},"SourceFile":{"additionalProperties":false,"properties":{"content":{"$ref":"#/$defs/Content"},"name":{"type":"string"}},"required":["name","content"],"type":"object"},"SourceFormat":{"oneOf":[{"const":"ork","type":"string"},{"const":"hpr_design","type":"string"}]},"Stage":{"additionalProperties":false,"properties":{"components":{"items":{"$ref":"#/$defs/Component"},"type":"array"},"id":{"type":"string"},"name":{"type":"string"},"overrides":{"$ref":"#/$defs/Overrides"}},"required":["id","components"],"type":"object"},"StageSeparation":{"additionalProperties":false,"properties":{"configurations":{"additionalProperties":{"$ref":"#/$defs/EventSetting_for_SeparationEvent"},"type":"object"},"id":{"type":"string"},"separation":{"$ref":"#/$defs/EventSetting_for_SeparationEvent"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["id","stage","separation","configurations"],"type":"object"},"Staging":{"additionalProperties":false,"properties":{"after_stage":{"format":"uint","minimum":0,"type":"integer"},"time_s":{"type":"number"},"trigger":{"$ref":"#/$defs/StagingTrigger"}},"required":["after_stage","trigger","time_s"],"type":"object"},"StagingTrigger":{"oneOf":[{"additionalProperties":false,"properties":{"time":{"additionalProperties":false,"properties":{"time_s":{"type":"number"}},"required":["time_s"],"type":"object"}},"required":["time"],"type":"object"},{"additionalProperties":false,"properties":{"burnout":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"mount":{"type":"string"}},"required":["mount","delay_s"],"type":"object"}},"required":["burnout"],"type":"object"}]},"StoredBranch":{"additionalProperties":false,"properties":{"events":{"items":{"$ref":"#/$defs/StoredEvent"},"type":"array"},"name":{"type":"string"},"rows":{"items":{"items":{"type":["number","null"]},"type":"array"},"type":"array"},"types":{"items":{"type":"string"},"type":"array"}},"required":["name","types","rows","events"],"type":"object"},"StoredEvent":{"additionalProperties":false,"properties":{"kind":{"type":"string"},"source":{"type":["string","null"]},"time_s":{"type":"number"}},"required":["time_s","kind"],"type":"object"},"StoredResults":{"additionalProperties":false,"properties":{"branches":{"items":{"$ref":"#/$defs/StoredBranch"},"type":"array"},"deployment_speed_m_s":{"type":["number","null"]},"flight_time_s":{"type":["number","null"]},"ground_hit_speed_m_s":{"type":["number","null"]},"max_acceleration_m_s2":{"type":["number","null"]},"max_altitude_m":{"type":["number","null"]},"max_mach":{"type":["number","null"]},"max_speed_m_s":{"type":["number","null"]},"optimum_delay_s":{"type":["number","null"]},"rod_exit_speed_m_s":{"type":["number","null"]},"time_to_apogee_s":{"type":["number","null"]},"warnings":{"items":{"type":"string"},"type":"array"}},"required":["branches","warnings"],"type":"object"},"StoredSimulation":{"additionalProperties":false,"properties":{"calculator":{"type":["string","null"]},"conditions":{"anyOf":[{"$ref":"#/$defs/LaunchConditions"},{"type":"null"}]},"name":{"type":"string"},"parser_warnings":{"type":"boolean"},"results":{"anyOf":[{"$ref":"#/$defs/StoredResults"},{"type":"null"}]},"simulator":{"type":["string","null"]},"status":{"type":["string","null"]}},"required":["name"],"type":"object"},"Streamer":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"},"width_m":{"type":"number"}},"required":["length_m","width_m","material","packing"],"type":"object"},"ThrustCurve":{"additionalProperties":false,"properties":{"thrusts_n":{"items":{"type":"number"},"type":"array"},"times_s":{"items":{"type":"number"},"type":"array"}},"required":["times_s","thrusts_n"],"type":"object"},"Transition":{"additionalProperties":false,"properties":{"aft_radius_m":{"type":"number"},"aft_shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"clipped":{"type":"boolean"},"fore_radius_m":{"type":"number"},"fore_shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"shape":{"$ref":"#/$defs/NoseShape"},"wall":{"$ref":"#/$defs/Wall"}},"required":["shape","length_m","fore_radius_m","aft_radius_m","wall","material"],"type":"object"},"TubeFinSet":{"additionalProperties":false,"properties":{"base_angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["count","length_m","outer_radius_m","thickness_m","material"],"type":"object"},"UnreadDevice":{"additionalProperties":false,"properties":{"at":{"type":"string"},"inside":{"type":"string"},"tag":{"type":"string"}},"required":["at","tag","inside"],"type":"object"},"UnreadMotor":{"additionalProperties":false,"properties":{"at":{"type":"string"},"designation":{"type":"string"},"inside":{"type":"string"},"reason":{"type":"string"}},"required":["at","designation","inside","reason"],"type":"object"},"Version":{"pattern":"^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$","type":"string"},"Wall":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"filled","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"shell","type":"string"},"thickness_m":{"type":"number"}},"required":["kind","thickness_m"],"type":"object"}]},"WindLevel":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"from_rad":{"type":["number","null"]},"speed_m_s":{"type":["number","null"]},"standard_deviation_m_s":{"type":["number","null"]}},"type":"object"}},"additionalProperties":false,"properties":{"extensions":{"$ref":"#/$defs/Extensions"},"format":{"$ref":"#/$defs/Format"},"motors":{"$ref":"#/$defs/Motors"},"provenance":{"$ref":"#/$defs/Provenance"},"recovery":{"$ref":"#/$defs/Recovery"},"rocket":{"$ref":"#/$defs/Rocket"},"simulations":{"items":{"$ref":"#/$defs/StoredSimulation"},"type":"array"},"source_files":{"items":{"$ref":"#/$defs/SourceFile"},"type":"array"},"version":{"$ref":"#/$defs/Version"}},"required":["format","version","provenance","rocket","motors","recovery","simulations","extensions","source_files"],"type":"object"}"""
)


class DesignFormatError(ValueError):
    """A document the reader refused: not JSON, not an hpr design, another version, or not valid."""


def read_design(text: str) -> DesignFile:
    """Reads a design document (`.hpr` text) and checks it against the format's schema, so what
    comes back has the types above.

    It checks what the schema says: every required key present, no unknown key, each value of
    its type, each tagged union one of its forms. hpr's own reader checks a few things more that
    no schema can say, such as that two source files don't share a name, so hpr can still refuse
    a document this takes. Like hpr, it refuses two equal keys in one object, `NaN`, `Infinity`
    and any number too large for a 64-bit float, a lone UTF-16 surrogate (`"\\ud800"`), nesting
    128 levels deep, and `2.0` where a whole number belongs.

    Raises `DesignFormatError` when the document is not one of this version.
    """
    try:
        # A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
        value = json.loads(
            text.removeprefix("﻿"),
            object_pairs_hook=_no_repeated_keys,
            parse_constant=_no_constants,
        )
    except RecursionError:
        raise DesignFormatError(f"not JSON: nested more than {_MOST_LEVELS} levels deep") from None
    except ValueError as error:
        raise DesignFormatError(f"not JSON: {error}") from None
    unread = _scan(value)
    if unread is not None:
        raise DesignFormatError(f"not JSON: {unread}")
    if not isinstance(value, dict) or value.get("format") != FORMAT:
        raise DesignFormatError(f'not an hpr design: its "format" is not "{FORMAT}"')
    if value.get("version") != VERSION:
        raise DesignFormatError(_version_message(value.get("version")))
    problem = _check(value, _SCHEMA, "$")
    if problem is not None:
        raise DesignFormatError(f"{problem[0]}: {problem[1]}")
    return cast(DesignFile, value)


def _version_message(version: Any) -> str:
    """Why a document of `version`, which isn't this one, is refused, and what to do."""
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version, re.ASCII) if isinstance(
        version, str
    ) else None
    if match is None:
        return f'its "version" is {_shown(version)}, not a version such as "{VERSION}"'
    theirs = (int(match[1]), int(match[2]))
    ours = tuple(int(part) for part in VERSION.split("."))
    if theirs > ours:
        return (
            f"written in version {version}, newer than these types, which read {VERSION}:"
            " take the types from a newer hpr"
        )
    return (
        f"written in version {version}; these types read {VERSION} only"
        f" (`hpr convert` rewrites an older document at {VERSION})"
    )


# The deepest nesting hpr reads: serde_json refuses a 128th level of arrays and objects.
_MOST_LEVELS = 127

# The largest finite 64-bit float.
_LARGEST_FLOAT = 1.7976931348623157e308


def _no_repeated_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, item in pairs:
        if key in result:
            raise ValueError(f"the key {json.dumps(key)} appears twice in one object")
        result[key] = item
    return result


def _no_constants(name: str) -> Any:
    raise ValueError(f"{name} is not a JSON number")


def _is_text(text: str) -> bool:
    """Whether `text` is Unicode, with no lone UTF-16 surrogate, which JSON can escape."""
    try:
        text.encode("utf-8")
    except UnicodeEncodeError:
        return False
    return True


def _scan(value: Any) -> str | None:
    """Why hpr couldn't read `value` as JSON although Python's reader did, or `None`."""
    stack: list[tuple[Any, int]] = [(value, 0)]
    while stack:
        item, level = stack.pop()
        if isinstance(item, float) and not math.isfinite(item):
            return "a number too large for a 64-bit float"
        if isinstance(item, int) and not isinstance(item, bool) and abs(item) > _LARGEST_FLOAT:
            return "a number too large for a 64-bit float"
        if isinstance(item, str) and not _is_text(item):
            return f"a lone UTF-16 surrogate in {_shown(item)}"
        if isinstance(item, (dict, list)):
            if level + 1 > _MOST_LEVELS:
                return f"nested more than {_MOST_LEVELS} levels deep"
            for key in item if isinstance(item, dict) else ():
                if not _is_text(key):
                    return f"a lone UTF-16 surrogate in the key {_shown(key)}"
            children = item.values() if isinstance(item, dict) else item
            stack.extend((child, level + 1) for child in children)
    return None


def _shown(value: Any) -> str:
    """`value` as JSON, cut to its first 40 characters."""
    text = json.dumps(value, separators=(",", ":"))
    return text if len(text) <= 40 else text[:40] + "…"


def _is_type(value: Any, kind: str) -> bool:
    if kind == "null":
        return value is None
    if kind == "boolean":
        return isinstance(value, bool)
    if kind == "string":
        return isinstance(value, str)
    if kind == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if kind == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if kind == "array":
        return isinstance(value, list)
    if kind == "object":
        return isinstance(value, dict)
    return False


_IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")

_MOST = {"uint32": 4294967295, "uint": 18446744073709551615}

_PATTERNS: dict[str, re.Pattern[str]] = {}


def _pattern(pattern: str) -> re.Pattern[str]:
    """`pattern` as JSON Schema reads it: ASCII classes, and `$` at the very end of the text only,
    where Python's `$` would also match before a final newline."""
    if pattern not in _PATTERNS:
        translated = pattern[:-1] + r"\Z" if pattern.endswith("$") else pattern
        _PATTERNS[pattern] = re.compile(translated, re.ASCII)
    return _PATTERNS[pattern]


# A problem: where in the document a check failed, and why; for a value that isn't a union's
# constant, also the value and the constant, so a union can list every constant it allows.
_Problem = tuple[str, str, Union[str, None], Union[str, None]]


def _member(path: str, key: str) -> str:
    """`path` followed by the key `key`: `.name` where it can be, else `["na-me"]`."""
    return f"{path}.{key}" if _IDENTIFIER.fullmatch(key) else f"{path}[{json.dumps(key)}]"


def _union(value: Any, forms: list[Any], path: str, exactly_one: bool) -> _Problem | None:
    """The problem with `value` against the union `forms`, or `None` if a form holds (exactly
    one, for `oneOf`): where the forms fail deepest, every constant they wanted there, or else the
    first deepest problem, or else that the value is none of them."""
    matched = 0
    problems: list[_Problem] = []
    for form in forms:
        problem = _check(value, form, path)
        if problem is None:
            matched += 1
        else:
            problems.append(problem)
    if exactly_one and matched > 1:
        return (path, f"matches {matched} of its forms, not one", None, None)
    if matched > 0:
        return None
    depth = max((len(p[0]) for p in problems), default=-1)
    deepest = [p for p in problems if len(p[0]) == depth]
    wanted = [p[3] for p in deepest]
    if len(deepest) > 1 and all(p[0] == deepest[0][0] for p in deepest) and None not in wanted:
        choices = list(dict.fromkeys(w for w in wanted if w is not None))
        listed = f"{', '.join(choices[:-1])} or {choices[-1]}"
        return (deepest[0][0], f"is {deepest[0][2]}, not {listed}", None, None)
    if depth > len(path):
        return deepest[0]
    return (path, f"{_shown(value)} is none of the {len(forms)} forms allowed here", None, None)


def _check(value: Any, node: Any, path: str) -> _Problem | None:
    """The first problem with `value` against `node`, or `None` when it holds."""
    if "$ref" in node:
        target = _SCHEMA["$defs"].get(node["$ref"].removeprefix("#/$defs/"))
        if target is None:
            return (path, f"the schema has no {node['$ref']}", None, None)
        problem = _check(value, target, path)
        if problem is not None:
            return problem
    if "type" in node:
        kinds = [node["type"]] if isinstance(node["type"], str) else node["type"]
        if not any(_is_type(value, kind) for kind in kinds):
            return (path, f"is {_shown(value)}, not {' or '.join(kinds)}", None, None)
    if "const" in node and value != node["const"]:
        found, expected = _shown(value), json.dumps(node["const"])
        return (path, f"is {found}, not {expected}", found, expected)
    for keyword, exactly_one in (("oneOf", True), ("anyOf", False)):
        if keyword in node:
            problem = _union(value, node[keyword], path, exactly_one)
            if problem is not None:
                return problem
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in node and value < node["minimum"]:
            return (path, f"is {value}, less than {node['minimum']}", None, None)
        most = _MOST.get(node.get("format", ""))
        if most is not None and value > most:
            return (path, f"is {value}, more than {node['format']} holds ({most})", None, None)
    if isinstance(value, str) and "pattern" in node and not _pattern(node["pattern"]).search(value):
        return (path, f"{_shown(value)} doesn't match {node['pattern']}", None, None)
    if isinstance(value, list):
        if "minItems" in node and len(value) < node["minItems"]:
            return (path, f"has {len(value)} items, fewer than {node['minItems']}", None, None)
        if "maxItems" in node and len(value) > node["maxItems"]:
            return (path, f"has {len(value)} items, more than {node['maxItems']}", None, None)
        prefix = node.get("prefixItems", [])
        for i, item in enumerate(value):
            item_node = prefix[i] if i < len(prefix) else node.get("items")
            problem = None if item_node is None else _check(item, item_node, f"{path}[{i}]")
            if problem is not None:
                return problem
    if isinstance(value, dict):
        for key in node.get("required", []):
            if key not in value:
                return (path, f"has no {json.dumps(key)}, which it needs", None, None)
        properties = node.get("properties", {})
        extra = node.get("additionalProperties", True)
        for key, item in value.items():
            if key in properties:
                problem = _check(item, properties[key], _member(path, key))
            elif extra is False:
                return (path, f"has the unknown key {json.dumps(key)}", None, None)
            elif isinstance(extra, dict):
                problem = _check(item, extra, _member(path, key))
            else:
                problem = None
            if problem is not None:
                return problem
    return None
