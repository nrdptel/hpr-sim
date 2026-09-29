// The hpr design format 0.2: types for a document, and a reader that checks one.
// SPDX-License-Identifier: MIT OR Apache-2.0
// From https://github.com/nrdptel/hpr-sim
// Generated from schema/format/hpr-design-0.2.schema.json by `cargo xtask format`.
// Don't edit by hand: the next run overwrites it, and CI fails while it is stale.

/** The format's name: every document says `"format": "hpr-design"`. */
export const FORMAT = "hpr-design";

/** The version these types describe; `readDesign` reads documents of this version only. */
export const VERSION = "0.2";

/** A rocket design in the hpr design format. */
export interface DesignFile {
  /**
   * What the source file holds that hpr does not model, by namespace, kept for writing it
   * back.
   */
  extensions: Extensions;
  /** Always `hpr-design`. */
  format: Format;
  /** Every motor configuration, flown or not, with why one is not. */
  motors: Motors;
  /** Which program wrote the document, and from what. */
  provenance: Provenance;
  /** When each parachute and streamer opens and each stage separates. */
  recovery: Recovery;
  /** The rocket: its stages and their parts, with every configuration that flies. */
  rocket: Rocket;
  /** The simulations the source file stored, with their conditions and results. */
  simulations: StoredSimulation[];
  /**
   * The source file's other files, in the order it held them: a `.ork` archive's entries
   * besides the design, such as embedded thrust curves and decal images. Version 0.1 called
   * them `attachments`.
   */
  source_files: SourceFile[];
  /** The format's version, `major.minor`. */
  version: Version;
}

/** The atmosphere a stored simulation was flown in. */
export type Atmosphere =
  /** `isa`: the International Standard Atmosphere. */
  | {
    model: "isa";
  }
  /** `extendedisa`: the standard atmosphere from a temperature and pressure at the launch site. */
  | {
    model: "extended";
    /** `<basepressure>`, Pa. */
    pressure_pa?: number | null;
    /** `<basetemperature>`, K. */
    temperature_k?: number | null;
  }
  /** A model not listed here, or none, kept by name only. */
  | {
    model: "other";
    /** The `model` attribute; empty when there is none. */
    name: string;
  };

/** A dimension resolved from the tree instead of stored in the part. */
export type AutoDimension =
  /** A nose cone's base radius: the next body component's forward radius. */
  | "base_radius"
  /**
   * A body tube's outer radius: the previous body component's aft radius, or, when that can't
   * be resolved, the next one's forward radius. A centering ring's or inner tube's outer
   * radius: its parent's inner radius, which is how a coupler or an engine block fills the tube
   * it sits in; inside a hollow nose cone or transition, the parent's outer radius at the
   * narrower end of the part less the parent's wall, and never below zero, as OpenRocket 24.12
   * reads it ([ADR-096][adr-096]). An inner tube whose resolved radius is less than its wall
   * is laid out solid (its wall is cut to its radius), as OpenRocket weighs it; a stated radius
   * with too thick a wall is still refused. A tube fin set's outer radius: the radius at which
   * its tubes close the ring around the body tube they sit on,
   * `TubeFinSet::closing_radius_m`, its wall cut to that
   * radius when thicker, as OpenRocket 24.12 reads it ([ADR-098][adr-098]).
   *
   * [adr-098]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-098-a-tube-fin-sets-automatic-radius-read-as-openrocket-reads-it-2026-09-28
   * [adr-096]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-096-fin-fillets-and-an-automatic-radius-inside-a-nose-cone-read-as-openrocket-reads-them-2026-09-28
   */
  | "outer_radius"
  /** A transition's forward radius: the previous body component's aft radius. */
  | "fore_radius"
  /** A transition's aft radius: the next body component's forward radius. */
  | "aft_radius"
  /** A nose cone's shoulder outer radius: the inner radius of the body tube behind it. */
  | "shoulder_radius"
  /**
   * A transition's forward shoulder outer radius: the inner radius of the body tube ahead of it.
   */
  | "fore_shoulder_radius"
  /** A transition's aft shoulder outer radius: the inner radius of the body tube behind it. */
  | "aft_shoulder_radius"
  /**
   * A centering ring's inner radius: the outer radius of the widest on-axis inner tube among
   * its siblings that overlaps it along the axis, or zero when none does.
   */
  | "inner_radius"
  /**
   * A mass component's or recovery part's packed radius: its parent's inner radius less the
   * part's radial offset.
   */
  | "packed_radius";

/** An airframe tube. */
export interface BodyTube {
  /** Length, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Outer radius, m. */
  outer_radius_m: number;
  /** Wall thickness, m. */
  thickness_m: number;
}

/** A flat annular ring, or a bulkhead when the inner radius is zero. */
export interface CenteringRing {
  /** Inner radius, m (zero for a bulkhead). */
  inner_radius_m: number;
  /** Thickness along the axis, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Outer radius, m. */
  outer_radius_m: number;
}

/** A node of the design tree: a part, where it sits, and what hangs off it. */
export interface Component {
  /** Dimensions taken from neighbours or the parent instead of the part's stored values. */
  auto?: AutoDimension[];
  /** Attached parts, or a pod set's body components. */
  children?: Component[];
  /**
   * The outer surface's finish, for skin friction; `None` means `Finish::default`. Parts
   * inside the body ignore it.
   */
  finish?: Finish | null;
  /** Unique id. */
  id: string;
  /** Makes a body tube or an inner tube a motor mount. */
  motor_mount?: MotorMount | null;
  /**
   * Name.
   *
   * Absent means `""`.
   */
  name?: string;
  /** Mass, centre-of-mass and inertia overrides. */
  overrides?: Overrides;
  /**
   * Whether the overrides replace this component together with everything attached to it
   * (`true`), or this component alone (`false`).
   */
  overrides_include_children?: boolean;
  /** The part, with its geometry and material. */
  part: Part;
  /**
   * Where an attached part sits along its parent. Body components (a stage's own list) have
   * none: they stack.
   */
  position?: Position | null;
}

/**
 * A set of motors to fly with: at most one per mount. A mount that is a cluster
 * (`InnerTube::cluster_m`) takes its motor in every tube.
 */
export interface Configuration {
  /** Unique id among the configurations. */
  id: string;
  /** The motors. */
  motors: MountedMotor[];
  /**
   * Name.
   *
   * Absent means `""`.
   */
  name?: string;
}

/** A file's contents: text, or base64 for bytes that are not UTF-8. */
export type Content =
  /** UTF-8 text, as the file holds it. */
  | {
    text: string;
  }
  /** Any other bytes, in standard base64 with padding, which must decode. */
  | {
    base64: string;
  };

/** Where a `<motor>` element's thrust curve came from. */
export type Curve =
  /** The archive's own `thrustcurves/<digest>.rse` entry. */
  | {
    /** The archive entry, such as `thrustcurves/<digest>.rse`. */
    entry: string;
    /** The motor built from it. */
    motor: SolidMotor;
    source: "embedded";
  }
  /** A curve the caller supplied for the motor's digest (`SuppliedCurves`). */
  | {
    /** The digest the design records, which the curve was supplied for. */
    digest: string;
    /** Where the supplied curves came from (`SuppliedCurves::source`). */
    from: string;
    /** The motor supplied. */
    motor: SolidMotor;
    source: "supplied";
  }
  /** The bundled ThrustCurve.org catalog (`Catalog::bundled`). */
  | {
    /** The motor built from it. */
    motor: SolidMotor;
    /** ThrustCurve.org's motor id. */
    motor_id: string;
    /** ThrustCurve.org's id of the curve file flown. */
    simfile_id: string;
    source: "catalog";
  }
  /** No curve, and why. */
  | {
    /** The same, in words. */
    reason: string;
    source: "unresolved";
    /** Why no curve was found. */
    why: NoCurve;
  };

/**
 * One available delay setting. Serialized with a `kind` tag and the seconds as `value`:
 * `{"kind":"seconds","value":6.0}`, `{"kind":"plugged"}`.
 */
export type Delay =
  /**
   * The ejection charge fires this many seconds after burnout: positive, or zero where a file
   * says so plainly, as a `.ork` does for a charge at burnout (`hpr_io::ork`).
   */
  | {
    kind: "seconds";
    value: number;
  }
  /** No ejection charge: the forward closure is plugged. */
  | {
    kind: "plugged";
  }
  /**
   * A `0`: the RASP spec means an ejection charge at burnout, but most files mean plugged
   * (`docs/format/eng.md`). The user or the catalog has to settle which.
   */
  | {
    kind: "zero_or_plugged";
  };

/** A density, with its units in the variant. */
export type Density =
  /** Mass per volume. */
  | {
    /** kg/m³. */
    kg_m3: number;
    kind: "bulk";
  }
  /** Mass per area, for sheets and fabrics. */
  | {
    /** kg/m². */
    kg_m2: number;
    kind: "surface";
  }
  /** Mass per length, for cords and lines. */
  | {
    /** kg/m. */
    kg_m: number;
    kind: "line";
  };

/** What deploys a recovery device, as `<deployevent>` names it. */
export type DeployEvent =
  /** `launch`: at launch, plus the delay. */
  | "launch"
  /** `ejection`: the first ejection charge of the device's own stage. */
  | "ejection"
  /** `apogee`. */
  | "apogee"
  /** `altitude`: the height `<deployaltitude>` gives, above the ground, on the way down. */
  | "altitude"
  /** `lowerstageseparation`: the separation of the stage below. */
  | "lower_stage_separation"
  /** `never`. */
  | "never"
  /** A word not listed here, kept as written. */
  | {
    other: string;
  };

/** Which kind of recovery device. */
export type DeviceKind = "parachute" | "streamer";

/** A number a `.ork` writes, which OpenRocket may be working out for itself. */
export type Dimension =
  /** A number the designer typed. */
  | {
    kind: "stated";
    /** The number, in whatever unit the tag is written in. */
    value: number;
  }
  /** A number OpenRocket works out from the neighbouring components. */
  | {
    /**
     * What it last worked out, when the file says (`auto 0.0125`). A bare `auto` gives
     * `None`. Either way this is a cached answer, not an input: a reader that resolves the
     * dimension itself should prefer its own.
     */
    cached?: number | null;
    kind: "automatic";
  };

/** An XML element: its name, its attributes in the order they were written, and its children. */
export interface Element {
  /** The attributes, in document order, as name and value. */
  attributes: Array<[string, string]>;
  /** The children, in document order. */
  children: Node[];
  /** The element's name, such as `nosecone`. */
  name: string;
}

/**
 * An event, a height for the events that need one, and a delay after it: when a device deploys
 * or a stage separates. Each is `None` where the file does not say.
 */
export interface EventSetting_for_DeployEvent {
  /** The height, m: above the ground for a deployment (measured; see the module docs). */
  altitude_m?: number | null;
  /** Seconds after the event, s. */
  delay_s?: number | null;
  /** The event. */
  event?: DeployEvent | null;
}

/**
 * An event, a height for the events that need one, and a delay after it: when a device deploys
 * or a stage separates. Each is `None` where the file does not say.
 */
export interface EventSetting_for_SeparationEvent {
  /** The height, m: above the ground for a deployment (measured; see the module docs). */
  altitude_m?: number | null;
  /** Seconds after the event, s. */
  delay_s?: number | null;
  /** The event. */
  event?: SeparationEvent | null;
}

/** The extensions a `.ork` design carries, by namespace. */
export interface Extensions {
  /**
   * What the design holds that hpr does not model.
   *
   * Absent means `{"attributes":[],"parts":[],"sections":[],"tags":[]}`.
   */
  "x-openrocket"?: OpenRocketExtension;
}

/** The shape of a fin's section along its chord. */
export type FinCrossSection = "square" | "rounded" | "airfoil";

/** Fillets along each fin's root: a concave joint on both faces, running the root chord. */
export interface FinFillet {
  /** Material (bulk). */
  material: Material;
  /** Radius of the fillet's concave face, m. */
  radius_m: number;
}

/** The outline of one fin. */
export type FinPlanform =
  /** A trapezoid with its tip chord parallel to the root. */
  | {
    kind: "trapezoidal";
    /** Root chord, m. */
    root_chord_m: number;
    /** Span from the body surface to the tip, m. */
    span_m: number;
    /** Axial distance from the root leading edge aft to the tip leading edge, m. */
    sweep_m: number;
    /** Tip chord, m (zero for a pointed fin). */
    tip_chord_m: number;
  }
  /** Half an ellipse on the root chord. */
  | {
    kind: "elliptical";
    /** Root chord, m. */
    root_chord_m: number;
    /** Span, m. */
    span_m: number;
  }
  /**
   * A polygon given as `[x, h]` points from the root leading edge, which must be `[0, 0]`,
   * around to the root trailing edge `[c_r, 0]` with `c_r > 0`, closed along the root; `x` runs
   * aft and `h` outward, in metres.
   */
  | {
    kind: "freeform";
    /** The outline, m. */
    points_m: number[][];
  };

/** A set of identical fins spaced evenly around the body. */
export interface FinSet {
  /**
   * Roll angle of the first fin from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  base_angle_rad?: number;
  /**
   * Cant angle, rad.
   *
   * Absent means `0.0`.
   */
  cant_rad?: number;
  /** Number of fins, at least 1. */
  count: number;
  /**
   * Section shape.
   *
   * Absent means `"square"`.
   */
  cross_section?: FinCrossSection;
  /** Optional fillets along each root. */
  fillet?: FinFillet | null;
  /** Material (bulk). */
  material: Material;
  /** Outline of each fin. */
  planform: FinPlanform;
  /**
   * Optional tab below each root.
   *
   * Absent means `null`.
   */
  tab?: FinTab | null;
  /** Maximum thickness, m. */
  thickness_m: number;
}

/** A rectangular tab below a fin's root, reaching into the body. */
export interface FinTab {
  /** Depth below the root, m. */
  height_m: number;
  /** Length along the root, m. */
  length_m: number;
  /** Distance from the root leading edge aft to the tab's leading edge, m. */
  offset_m: number;
}

/** A surface finish, by its roughness height `R_s`. */
export type Finish =
  /** A mirror-like surface, 0 µm (Barrowman Table 4-1). */
  | {
    kind: "mirror";
  }
  /** Average glass, 0.1 µm. */
  | {
    kind: "average_glass";
  }
  /** A finished and polished surface, 0.5 µm. */
  | {
    kind: "polished";
  }
  /** An aircraft-type sheet-metal surface, 2 µm (Barrowman Table 4-1 only). */
  | {
    kind: "sheet_metal";
  }
  /** An optimum paint-sprayed surface, 5 µm. */
  | {
    kind: "optimum_paint";
  }
  /** Planed wooden boards, 15 µm. */
  | {
    kind: "planed_wood";
  }
  /** Paint in aircraft mass production, 20 µm. The default. */
  | {
    kind: "mass_production_paint";
  }
  /** Bare steel plating, 50 µm (Barrowman Table 4-1 only). */
  | {
    kind: "bare_steel";
  }
  /** A smooth cement surface, 50 µm. */
  | {
    kind: "smooth_cement";
  }
  /** A surface with an asphalt-type coating, 100 µm (Barrowman Table 4-1 only). */
  | {
    kind: "asphalt_coating";
  }
  /** A dip-galvanized metal surface, 150 µm. */
  | {
    kind: "dip_galvanized";
  }
  /** Incorrectly sprayed aircraft paint, 200 µm. */
  | {
    kind: "poor_paint";
  }
  /** The natural surface of cast iron, 250 µm (Barrowman Table 4-1 only). */
  | {
    kind: "cast_iron";
  }
  /** Raw wooden boards, 500 µm. */
  | {
    kind: "raw_wood";
  }
  /** An average concrete surface, 1000 µm. */
  | {
    kind: "concrete";
  }
  /** A given roughness height. */
  | {
    kind: "custom";
    /** Roughness height, m. */
    roughness_m: number;
  };

/** The format's name: a document holds only `hpr-design`. */
export type Format = "hpr-design";

/**
 * When a motor lights, on the flight's clock: `t = 0` is launch, when the motors that light at
 * launch ignite. A delay is counted from its event (the decision record on staging,
 * [ADR-074][adr-074]).
 *
 * [adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
 */
export type Ignition =
  /** At launch, `t = 0`. */
  | "launch"
  /** At a time after launch: an air start on a timer. */
  | {
    time: {
      /** The time after launch, s. */
      time_s: number;
    };
  }
  /**
   * A delay after another mount's motor burns out: a sustainer lit by the booster's burnout,
   * or by its ejection charge with the charge's delay.
   */
  | {
    burnout: {
      /** The delay after that burnout, s. */
      delay_s: number;
      /** The id of the mount whose motor's burnout lights this one. */
      mount: string;
    };
  }
  /**
   * A delay after the stage aft of this motor's stage separates from it (the flight gives the
   * separation). A motor whose stage is never freed never lights; one in the last stage, with
   * nothing aft of it to separate, is refused.
   */
  | {
    separation: {
      /** The delay after the separation, s. */
      delay_s: number;
    };
  }
  /**
   * Never: the motor stays loaded and gives no thrust all flight, as if every tube of its mount
   * were listed in `MountedMotor::failed_tubes`. A `.ork` design can set a motor so, or light
   * it at an event that never comes, and OpenRocket then flies it unlit.
   */
  | "never";

/**
 * When a motor ignites, as `<ignitionevent>` names it.
 *
 * OpenRocket's file-format page shows only `automatic`. The five values here are the ones
 * OpenRocket 24.12 writes, each measured by setting it and saving; the meanings are its own
 * labels, and for `automatic` its FAQ ("How do I create a staged rocket?"). Anything else is kept
 * as written.
 */
export type IgnitionEvent =
  /**
   * `automatic`, "Automatic (launch or ejection charge)": the lowest stage at launch, and each
   * stage above it at the ejection charge of the stage below.
   */
  | "automatic"
  /** `launch`: at launch. */
  | "launch"
  /** `ejectioncharge`: at the first ejection charge of the stage below. */
  | "ejection_charge"
  /** `burnout`: at the first burnout of the stage below. */
  | "burnout"
  /** `never`. */
  | "never"
  /** A value not listed here, kept as written. */
  | {
    other: string;
  };

/**
 * An inertia tensor about the centre of mass in body axes, kg·m². The off-diagonal entries are
 * the tensor's, `I_xy = −∫ x y dm` (`crate::mass`); they default to zero.
 */
export interface InertiaOverride {
  /** `I_xx`, kg·m². */
  xx_kg_m2: number;
  /**
   * `I_xy`, kg·m².
   *
   * Absent means `0.0`.
   */
  xy_kg_m2?: number;
  /**
   * `I_xz`, kg·m².
   *
   * Absent means `0.0`.
   */
  xz_kg_m2?: number;
  /** `I_yy`, kg·m². */
  yy_kg_m2: number;
  /**
   * `I_yz`, kg·m².
   *
   * Absent means `0.0`.
   */
  yz_kg_m2?: number;
  /** `I_zz`, about the rocket's axis, kg·m². */
  zz_kg_m2: number;
}

/**
 * A tube inside the airframe: a coupler, a motor mount tube, an engine block or thrust ring. It
 * may sit off the axis, and it may be a cluster: several like tubes side by side, as in a
 * cluster's motor mount.
 *
 * **A cluster.** `Self::cluster_m` lists where each tube's axis sits, `[x, y]` in body axes
 * from the axis the radial offset and angle give. The tubes are the one tube written here,
 * repeated at each place: their mass is the sum of the copies, each with its own parallel-axis
 * term. What the tube holds (an engine block, a motor) is repeated in every tube in the same way
 * (`docs/physics/design.md`, the decision record on clusters, [ADR-075][adr-075]).
 *
 * [adr-075]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-075-a-cluster-is-one-tube-repeated-and-a-motor-in-it-one-motor-per-tube-2026-09-25
 */
export interface InnerTube {
  /**
   * Roll angle of that offset from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  angle_rad?: number;
  /**
   * A cluster's tubes: each tube's axis, `[x, y]` in body axes, m, measured from the axis the
   * radial offset and angle give. Empty (the default) for one tube on that axis.
   */
  cluster_m?: number[][];
  /** Length, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Outer radius, m. */
  outer_radius_m: number;
  /**
   * Distance of the tube's axis from the body axis, m.
   *
   * Absent means `0.0`.
   */
  radial_offset_m?: number;
  /** Wall thickness, m. */
  thickness_m: number;
}

/** An element kept whole, and where it was. */
export interface Kept {
  /** Its path in the document; see `element_at`. */
  at: string;
  /** The element, with everything inside it. */
  element: Element;
}

/** An attribute kept, and the element it was on. */
export interface KeptAttribute {
  /** The path of the element it was on; see `element_at`. */
  at: string;
  /** Its name. */
  name: string;
  /** Its value, as written. */
  value: string;
}

/**
 * The launch conditions a stored simulation was flown in. Each is `None` where the file does not
 * say.
 */
export interface LaunchConditions {
  /** `<atmosphere>`. */
  atmosphere?: Atmosphere | null;
  /** `<configid>`: the motor configuration flown. */
  configuration?: string | null;
  /** `<geodeticmethod>`: `flat`, `spherical` or `wgs84`. */
  geodetic_method?: string | null;
  /**
   * `<launchintowind>`: whether the rod is pointed into the wind, which OpenRocket then writes
   * as the rod's direction.
   */
  into_wind?: boolean | null;
  /** `<launchlatitude>`, degrees north. */
  latitude_deg?: number | null;
  /** `<launchaltitude>`: the launch site's height above sea level, m. */
  launch_altitude_m?: number | null;
  /** `<launchlongitude>`, degrees east. */
  longitude_deg?: number | null;
  /** `<maxtime>`, s. */
  max_time_s?: number | null;
  /** `<launchrodangle>`: the rod's tilt from vertical, rad (the file writes degrees). */
  rod_angle_rad?: number | null;
  /**
   * `<launchroddirection>`: the compass bearing the rod tilts toward, clockwise from north, rad
   * (the file writes degrees).
   */
  rod_direction_rad?: number | null;
  /** `<launchrodlength>`, m. */
  rod_length_m?: number | null;
  /** `<timestep>`, s. */
  time_step_s?: number | null;
  /**
   * `<winddirection>`, or the average wind's `<direction>`: the compass bearing the wind blows
   * from, rad (the file writes radians).
   */
  wind_from_rad?: number | null;
  /** `<wind model="multilevel">`'s levels, lowest first as written. */
  wind_levels: WindLevel[];
  /**
   * The multilevel wind's `altituderef`: whether its altitudes are above the ground (`agl`) or
   * the sea (`msl`).
   */
  wind_levels_above?: string | null;
  /**
   * `<windmodeltype>`: which wind the run flew, `Average` or the multilevel one. OpenRocket
   * writes both winds whichever it flew.
   */
  wind_model?: string | null;
  /** `<windaverage>`, or the average wind's `<speed>`: the mean wind speed, m/s. */
  wind_speed_m_s?: number | null;
  /**
   * `<windturbulence>`: the turbulence intensity, the standard deviation of the wind speed over
   * its mean.
   */
  wind_turbulence?: number | null;
}

/** A launch lug: a tube on the outside of the airframe, parallel to it. */
export interface LaunchLug {
  /**
   * Roll angle from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  angle_rad?: number;
  /**
   * Number of lugs in a row.
   *
   * Absent means `1`.
   */
  count?: number;
  /** Length, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Outer radius, m. */
  outer_radius_m: number;
  /**
   * Axial distance between the forward ends of consecutive lugs, m.
   *
   * Absent means `0.0`.
   */
  spacing_m?: number;
  /** Wall thickness, m. */
  thickness_m: number;
}

/** Why a configuration is not among the rocket's. */
export interface LeftOut {
  /** The same, in words, naming the motor. */
  message: string;
  /** The reason. */
  why: NotFlown;
}

/** A mass of known value: an altimeter bay, ballast, a payload. */
export interface MassComponent {
  /** Mass, kg. */
  mass_kg: number;
  /** Its extent. */
  packing: Packing;
}

/** A mass on the motor axis, with its moments of inertia about its own centre of mass. */
export interface MassElement {
  /** Moment of inertia about the motor axis, through the element's centre of mass, kg·m². */
  axial_inertia_kg_m2: number;
  /** Centre of mass along the motor axis, m from the nozzle exit toward the forward end. */
  cg_m: number;
  /** Mass, kg. */
  mass_kg: number;
  /** Moment of inertia about a transverse axis through the element's centre of mass, kg·m². */
  transverse_inertia_kg_m2: number;
}

/** A named material. */
export interface Material {
  /** Density. */
  density: Density;
  /** Name, for display and for matching imported designs. */
  name: string;
}

/** A motor configuration: what the rocket declares, and every motor the mounts put in it. */
export interface MotorConfiguration {
  /** Whether `<rocket>` declares it; one only a mount names is read all the same. */
  declared: boolean;
  /** Whether the file marks it `default="true"`. */
  default: boolean;
  /** `configid`. */
  id: string;
  /**
   * The `<stage number>`s the configuration marks `active="false"`; `None` for one whose number
   * is missing or not a count, which is switched off all the same.
   */
  inactive_stages: Array<number | null>;
  /** Why it is not among the rocket's configurations, or `None` when it is. */
  left_out?: LeftOut | null;
  /** Its motors, in the order their mounts appear in the file. */
  motors: OrkMotor[];
  /** `<name>`, which may be empty. */
  name: string;
  /**
   * The powered separation it flies, for one among the rocket's configurations that has one
   * (`staging`). The rocket's configuration does not carry it: give it to the flight
   * (`hpr::ork::separation` maps it onto one), with a recovery device on each part.
   */
  staging?: Staging | null;
  /** Its motors in parts hpr does not read. */
  unread: UnreadMotor[];
}

/** Makes a body tube or inner tube a motor mount. */
export interface MotorMount {
  /**
   * How far the nozzle exit sits aft of the mount's aft end, m (negative when recessed).
   *
   * Absent means `0.0`.
   */
  overhang_m?: number;
}

/** Every motor configuration a `.ork` design holds. */
export interface Motors {
  /** The configurations: those `<rocket>` declares in its order, then any only a mount names. */
  configurations: MotorConfiguration[];
}

/** A motor in a mount. */
export interface MountedMotor {
  /** The ejection delay chosen, if any. */
  delay?: Delay | null;
  /**
   * Designation, for display.
   *
   * Absent means `""`.
   */
  designation?: string;
  /** Case outer diameter, m. */
  diameter_m: number;
  /**
   * The tubes whose motor fails to light, by index into the mount's tubes (a cluster's in the
   * order of `InnerTube::cluster_m`, `0` for a single tube; a
   * cluster inside another cluster counts the outer copies first, each with all its tubes; a
   * mount in a pod counts the pods, in the order of `PodSet::pods`): a
   * motor out. Each is carried loaded and gives no thrust. An ignition on the mount's burnout
   * takes its first motor that lights, but a recovery device or separation triggered by one
   * motor's index waits on that motor alone: point it at a tube that lights, or it never fires.
   * Empty (the default) when every motor lights.
   */
  failed_tubes?: number[];
  /** When it lights. */
  ignition?: Ignition;
  /** Case length, m. */
  length_m: number;
  /** The motor. */
  motor: SolidMotor;
  /** The id of the mount component. */
  mount: string;
}

/** Why a motor has no thrust curve. */
export type NoCurve = "hybrid" | "no_designation" | "not_found" | "ambiguous" | "unusable";

/** A child of an `Element`: another element, or text. */
export type Node =
  /** A child element. */
  | {
    /** The attributes, in document order, as name and value. */
    attributes: Array<[string, string]>;
    /** The children, in document order. */
    children: Node[];
    kind: "element";
    /** The element's name, such as `nosecone`. */
    name: string;
  }
  /** Text. Character and entity references are already resolved. */
  | {
    kind: "text";
    /** The text as it stands. */
    text: string;
  };

/** A nose cone: a profile with its tip forward, and an optional shoulder aft of its base. */
export interface NoseCone {
  /** Base radius, m. */
  base_radius_m: number;
  /** Length from tip to base, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Profile shape. */
  shape: NoseShape;
  /**
   * Optional shoulder.
   *
   * Absent means `null`.
   */
  shoulder?: Shoulder | null;
  /** Filled, or a wall of a thickness. */
  wall: Wall;
}

/** The shape of a nose cone, or of a transition's profile. */
export type NoseShape =
  /** A straight cone. */
  | {
    kind: "conical";
  }
  /** A circular-arc ogive whose arc radius is `radius_ratio` times the tangent-ogive radius. */
  | {
    kind: "ogive";
    /**
     * The arc radius over the tangent-ogive radius: 1 for a tangent ogive, above 1 for a
     * secant ogive, below 1 (down to `R/L`) for a bulged secant ogive.
     */
    radius_ratio: number;
  }
  /** Half an ellipse: a blunt, rounded tip. */
  | {
    kind: "elliptical";
  }
  /** `g = ξⁿ`: `n = 1` is a cone and `n = ½` a paraboloid. */
  | {
    /** The exponent `n`, in `[0.05, 1]` (`MIN_POWER_EXPONENT`). */
    exponent: number;
    kind: "power_series";
  }
  /** Parabolic series: `K′ = 0` is a cone and `K′ = 1` a full parabola, tangent at the base. */
  | {
    kind: "parabolic_series";
    /** The parameter `K′`, in `[0, 1]`. */
    parameter: number;
  }
  /** Haack series: `C = 0` is the von Kármán (LD-Haack) ogive and `C = 1/3` the LV-Haack. */
  | {
    kind: "haack";
    /** The parameter `C`, in `[0, 2/3]`. */
    parameter: number;
  };

/**
 * The reasons a configuration cannot be flown as written, in the order they are checked; each is
 * checked across every motor before the next, so the one given is the first on this list that
 * applies.
 */
export type NotFlown =
  /** A motor is in a part hpr does not read, such as a pod. */
  | "unread_motor"
  /** It holds no motor. */
  | "no_motor"
  /** It marks a stage inactive, and hpr flies every stage. */
  | "inactive_stage"
  /** A motor has no thrust curve. */
  | "no_curve"
  /** A motor has no case diameter or length. */
  | "no_size"
  /**
   * A motor lights when hpr can't light it: at a word hpr does not know, after a negative
   * delay, at the ejection charge of a motor below that states no delay, or at an event of a
   * stage below that holds no motor, or motors in more than one mount; or no motor of the
   * configuration lights at all (`staging`). A motor that never lights beside one that does is flown unlit.
   */
  | "ignition_not_flown"
  /**
   * The airframe or a motor mount was not read exactly as written: reading it raised a
   * warning. A part was left out (a pod set hpr cannot lay out, a parallel stage, a part hpr
   * could not give a shape), a value was dropped or simplified (a rail button's screw
   * head, a material that could not be read), or something was assumed (a shape hpr does not know read
   * as a cone). Flying it would fly a
   * rocket the design may not be.
   */
  | "airframe_not_as_written"
  /**
   * Its stages come apart in a way hpr doesn't fly: more than one separation; one that can
   * come before apogee with no motor ahead of it still burning or yet to light when it fires,
   * or with a motor behind it not yet spent; one at launch, or at the ignition of a motor that
   * never lights; a negative delay; or an event hpr has no trigger for (`staging`).
   */
  | "separation_not_flown";

/** A nozzle, for the ambient-pressure correction of thrust. */
export interface Nozzle {
  /** Exit radius, m. */
  exit_radius_m: number;
  /**
   * The ambient pressure the thrust curve was measured at, Pa: the static test site's. The
   * thrust is then corrected to the ambient pressure in flight.
   *
   * `None` flies the curve as it is at every ambient pressure, with no correction. That is
   * RocketPy's default (`Motor(reference_pressure=None)`, whose `pressure_thrust` is then zero,
   * `motor.py:1188-1189`), so a RocketPy input transcribed into hpr says `None`.
   *
   * Which to give: motor files don't record where the curve was measured. For a motor tested
   * near sea level, `STANDARD_SEA_LEVEL_PRESSURE_PA` adds the thrust a higher site gains
   * (16 kPa × `A_e` at 1,400 m); `None` leaves it out. A design must say which: the field is
   * required, as `null` for `None`, so leaving it out is an error rather than a silent choice.
   * Motors read from `.eng` or `.rse` files, or from the catalog, carry no nozzle and so no
   * correction.
   */
  reference_pressure_pa: number | null;
  /** Throat radius, m, when known (informational: the thrust curve already carries its effect). */
  throat_radius_m?: number | null;
}

/**
 * The `x-openrocket` extension: the parts and sections of a `.ork` that hpr does not read, each
 * kept whole where it was.
 */
export interface OpenRocketExtension {
  /**
   * The attributes hpr does not read on an element it does read, such as a material's
   * `group`, and those whose value a reader dropped, such as a material's declared `type` where
   * the part needs another.
   *
   * Absent means `[]`.
   */
  attributes?: KeptAttribute[];
  /**
   * The parts hpr does not read, in file order.
   *
   * Absent means `[]`.
   */
  parts?: Kept[];
  /**
   * The sections of the document hpr does not read, in file order.
   *
   * Absent means `[]`.
   */
  sections?: Kept[];
  /**
   * The tags hpr does not read in an element it does read — a part, a stage, the rocket, a
   * stored simulation, or a tag inside any of those that a reader asked for — such as a part's
   * `<appearance>`; and those a reader asked for and dropped or simplified, such as a rail
   * button's `<screwheight>`, which the design does not hold.
   *
   * Absent means `[]`.
   */
  tags?: Kept[];
}

/** When a motor ignites: an event and a delay after it. */
export interface OrkIgnition {
  /** Seconds after the event, s. */
  delay_s: number;
  /** The event. */
  event: IgnitionEvent;
}

/**
 * A motor in a mount, in one configuration: what the `<motor>` element says, when it ignites in
 * that configuration, and the curve it flies on.
 */
export interface OrkMotor {
  /** The thrust curve. */
  curve: Curve;
  /**
   * `<delay>`: `none` is a plugged motor, with no ejection charge; a number is the seconds from
   * burnout to the charge, and `0` fires it at burnout (OpenRocket's technical documentation,
   * pages 8 and 10).
   */
  delay?: Delay | null;
  /** `<designation>`, such as `H148R`. */
  designation: string;
  /** `<diameter>`: the case diameter, m. */
  diameter_m?: number | null;
  /** `<digest>`: OpenRocket's key for the thrust curve, which names an embedded curve. */
  digest?: string | null;
  /**
   * When it ignites in this configuration: the configuration's `<ignitionconfiguration>` where
   * the mount has one, the mount's own default where it does not.
   */
  ignition: OrkIgnition;
  /** `<type>` as written: `single`, `reload` or `hybrid`. */
  kind?: string | null;
  /** `<length>`: the case length, m. */
  length_m?: number | null;
  /** `<manufacturer>`. */
  manufacturer: string;
  /** The id of the mount component in the rocket. */
  mount: string;
  /** The index of the mount's stage in `Rocket::stages`. */
  stage: number;
}

/**
 * Values that replace the mass properties computed from geometry. Each applies in turn:
 *
 * 1. **Mass** `m′`: the body is rescaled, `I′ = I m′/m`, keeping its centre and shape. A body with
 *    no mass becomes a point mass at its centre, but for a packed part in a layout (a mass
 *    component, parachute, streamer or shock cord), which takes `m′` as a solid cylinder of its
 *    packing, as OpenRocket 24.12 does ([ADR-063][adr-063]).
 * 2. **Centre of mass**: the centre moves along the axis to `cg_aft_m` aft of the component's
 *    forward end (a stage's, for a stage), with or without its children, and off the axis to
 *    `cg_xy_m` when given (otherwise it keeps its offset); the tensor about the centre is
 *    unchanged.
 * 3. **Inertia**: the tensor about the (new) centre is replaced.
 *
 * [adr-063]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-063-packed-parts-read-and-weighed-as-openrocket-packs-them-2026-09-21
 */
export interface Overrides {
  /** Centre of mass, m aft of the forward end of the component (or the stage). */
  cg_aft_m?: number | null;
  /**
   * Centre of mass off the axis, `[x, y]` in body axes, m. For a part inside a cluster's tube or
   * a pod, it is measured in that one copy as written (a pod on the body's axis), and the copies
   * carry it to each place.
   */
  cg_xy_m?: number[] | null;
  /**
   * Inertia tensor about the centre of mass. For a part inside a cluster's tube or a pod, it is
   * in that one copy's axes as written, and turns with each pod.
   */
  inertia?: InertiaOverride | null;
  /** Mass, kg. */
  mass_kg?: number | null;
}

/** Where and how compactly a mass or a recovery part is stowed: a solid cylinder. */
export interface Packing {
  /**
   * Roll angle of that offset from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  angle_rad?: number;
  /** Length, m. */
  length_m: number;
  /**
   * Distance of its axis from the body axis, m.
   *
   * Absent means `0.0`.
   */
  radial_offset_m?: number;
  /** Radius, m. */
  radius_m: number;
}

/** A parachute, packed. */
export interface Parachute {
  /** Canopy fabric (surface). */
  canopy_material: Material;
  /** Nominal (flat) canopy diameter, m. */
  diameter_m: number;
  /** Number of shroud lines. */
  line_count: number;
  /** Length of each shroud line, m. */
  line_length_m: number;
  /** Shroud line (line). */
  line_material: Material;
  /** How it is packed. */
  packing: Packing;
}

/** A part in the tree. Serialized as an object with one key, the part's kind. */
export type Part =
  /** A nose cone (body component). */
  | {
    nose_cone: NoseCone;
  }
  /** A body tube (body component). */
  | {
    body_tube: BodyTube;
  }
  /** A transition (body component). */
  | {
    transition: Transition;
  }
  /** An inner tube (internal): a coupler, motor mount tube or engine block. */
  | {
    inner_tube: InnerTube;
  }
  /** A centering ring or bulkhead (internal). */
  | {
    centering_ring: CenteringRing;
  }
  /** A fin set (external, on a body tube). Its axial extent is the root chord. */
  | {
    fin_set: FinSet;
  }
  /** Tube fins (external, on a body tube). */
  | {
    tube_fin_set: TubeFinSet;
  }
  /** Launch lugs (external, on a body tube). The extent covers the whole row. */
  | {
    launch_lug: LaunchLug;
  }
  /** Rail buttons (external, on a body tube). The extent covers the whole row. */
  | {
    rail_button: RailButton;
  }
  /**
   * Pods (external, on a body tube). Its children are the pod's body components, which stack
   * along the pod's axis; its extent is theirs (`Component::length_m`).
   */
  | {
    pod_set: PodSet;
  }
  /** A mass component (internal). */
  | {
    mass_component: MassComponent;
  }
  /** A parachute (internal). */
  | {
    parachute: Parachute;
  }
  /** A streamer (internal). */
  | {
    streamer: Streamer;
  }
  /** A shock cord (internal). */
  | {
    shock_cord: ShockCord;
  };

/**
 * Pods beside the airframe: side pods, or outboard motor pods. A pod set attaches to a body tube
 * like a fin set, and its children are the pod's own body components (nose cone, body tubes,
 * transitions), which stack aft from the pod set's position along the pod's axis instead of the
 * body's.
 *
 * **Copies.** The pod written in the tree is one pod on the body's axis, repeated `count` times
 * around it as a rotational pattern: pod `k` is that pod turned by `φ_k = angle + 2π k / count`
 * about the body's axis and moved to `r (cos φ_k, sin φ_k)` (`Self::pods`), in
 * [body axes](https://github.com/nrdptel/hpr-sim/blob/main/docs/physics/frames.md). Everything
 * the pod holds (fins on its tubes, parts inside them, a motor in a mount) turns and moves with
 * it, so what points away from the airframe on one pod does on every pod. Each copy adds its own
 * parallel-axis term, `I_O = I_cg + m (|d|² E − d dᵀ)` with `d` the copy's centre from the point
 * `O` (J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*, appendix B). The pod set
 * itself weighs nothing: its mass is its pods'. See the design page's *Pods* section
 * (`docs/physics/design.md`) and the decision record on pods, [ADR-089][adr-089].
 *
 * **A pod of no length.** A pod may be a single body tube of no length, which weighs nothing: what
 * hangs from it (fins, a launch lug) sits on a tube of that radius, most often none, so on the
 * pod's own axis, and is repeated around the body's as any pod is. OpenRocket draws winglets this
 * way, calling the tube a "phantom body". A pod set may also hold nothing at all, and then weighs
 * nothing.
 *
 * **Flown** since
 * [M1.13c1](https://github.com/nrdptel/hpr-sim/blob/main/docs/decisions-and-roadmap.md#m1-13c1):
 * each pod's parts take their own normal force and drag, once per pod
 * ([aerodynamics: Pods](https://nrdptel.github.io/hpr-sim/physics/aero.html#pods)).
 *
 * [adr-089]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-089-a-pod-is-a-stack-of-body-components-repeated-around-the-axis-2026-09-27
 */
export interface PodSet {
  /**
   * Roll angle of the first pod from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  angle_rad?: number;
  /** Number of pods, at least one, spaced evenly around the body's axis. */
  count: number;
  /** Distance of each pod's axis from the body's axis, m. */
  radial_offset_m: number;
}

/** Where an attached part sits along its parent. Offsets are positive aft. */
export type Position =
  /** The part's forward end is `aft_offset_m` aft of the parent's forward end. */
  | {
    /**
     * Offset, m.
     *
     * Absent means `0.0`.
     */
    aft_offset_m?: number;
    from: "top";
  }
  /** The part's middle is `aft_offset_m` aft of the parent's middle. */
  | {
    /**
     * Offset, m.
     *
     * Absent means `0.0`.
     */
    aft_offset_m?: number;
    from: "middle";
  }
  /** The part's aft end is `aft_offset_m` aft of the parent's aft end. */
  | {
    /**
     * Offset, m.
     *
     * Absent means `0.0`.
     */
    aft_offset_m?: number;
    from: "bottom";
  }
  /**
   * The part's forward end is `aft_offset_m` aft of the previous sibling's aft end, or of the
   * parent's forward end for the first child.
   */
  | {
    /**
     * Offset, m.
     *
     * Absent means `0.0`.
     */
    aft_offset_m?: number;
    from: "after";
  }
  /** The part's forward end is at station `station_m`, measured aft of the nose tip. */
  | {
    from: "absolute";
    /** Station, m. */
    station_m: number;
  };

/**
 * How the propellant is laid out and how its shape evolves. Serialized with a `model` tag
 * (`"column"`, `"grains"`).
 *
 * Not `Copy`, so that a later model can hold tabulated data.
 */
export type Propellant =
  /** A fixed-shape column: the default when only a motor's envelope is known. */
  | {
    /** Centre of the column along the motor axis, m from the nozzle exit. */
    center_m: number;
    /** Inner (bore) radius, m; zero for a solid column. */
    inner_radius_m: number;
    /** Length, m. */
    length_m: number;
    /** Initial propellant mass, kg. */
    mass_kg: number;
    model: "column";
    /** Outer radius, m. */
    outer_radius_m: number;
  }
  /** BATES grains that regress on their bores and ends. */
  | {
    /** Centre of the grain stack along the motor axis, m from the nozzle exit. */
    center_m: number;
    /** Number of grains `N`, at least 1. */
    count: number;
    /** Propellant density `ρ`, kg/m³. */
    density_kg_m3: number;
    /**
     * Whether the grain ends are inhibited, so only the bores burn (RocketPy's
     * `only_radial_burn`).
     */
    inhibited_ends: boolean;
    /** Initial grain height (length) `h₀`, m. */
    initial_height_m: number;
    /** Initial bore radius `r₀`, m, positive. */
    initial_inner_radius_m: number;
    model: "grains";
    /** Grain outer radius `R`, m. */
    outer_radius_m: number;
    /** Gap between adjacent grains `s`, m. */
    separation_m: number;
  };

/** Which program wrote a document, and from what. */
export interface Provenance {
  /** The file the design was read from, if it was read from one. */
  source?: Source | null;
  /** The program, such as `hpr-sim`. */
  tool: string;
  /** Its version. */
  tool_version: string;
}

/**
 * A rail button: a base disc on the airframe, a narrower waist, and a flange that rides in the
 * rail, stacked outward along a radial line.
 */
export interface RailButton {
  /**
   * Roll angle from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  angle_rad?: number;
  /** Height of the base, m. */
  base_height_m: number;
  /**
   * Number of buttons in a row.
   *
   * Absent means `1`.
   */
  count?: number;
  /** Height of the flange, m. */
  flange_height_m: number;
  /** Total height above the airframe, m. */
  height_m: number;
  /** Diameter of the waist, m. */
  inner_diameter_m: number;
  /** Material (bulk). */
  material: Material;
  /** Diameter of the base and the flange, m. */
  outer_diameter_m: number;
  /**
   * Axial distance between the forward edges of consecutive buttons, m.
   *
   * Absent means `0.0`.
   */
  spacing_m?: number;
}

/** Every recovery setting in a `.ork` design. */
export interface Recovery {
  /** The parachutes and streamers read, in file order. */
  devices: RecoveryDevice[];
  /** Every stage that states when it separates, in file order. */
  separations: StageSeparation[];
  /** The parachutes and streamers in parts hpr does not read. */
  unread: UnreadDevice[];
  /** The parallel stages that state a separation, which hpr does not read yet. */
  unread_separations: UnreadDevice[];
}

/** A parachute's or streamer's recovery settings. */
export interface RecoveryDevice {
  /**
   * `<cd>`: a stated drag coefficient, or `auto` for OpenRocket's own (0.8 for a parachute,
   * from the strip's size for a streamer). `None` where the file says nothing.
   */
  cd?: Dimension | null;
  /**
   * When it deploys in each configuration that changes that, by `configid`, with anything the
   * configuration leaves out taken from `RecoveryDevice::deployment`.
   */
  configurations: { [key: string]: EventSetting_for_DeployEvent };
  /** When it deploys, as the device states it. */
  deployment: EventSetting_for_DeployEvent;
  /** The id of the device's component in the rocket. */
  id: string;
  /** Parachute or streamer. */
  kind: DeviceKind;
  /** The index of its stage in `Rocket::stages`. */
  stage: number;
}

/** How the reference diameter (for aerodynamic coefficients) is chosen. */
export type ReferenceDiameter =
  /**
   * The widest body component (nose cone, body tube or transition) in any stage. Internal
   * parts, shoulders, fins, tube fins, lugs and rail buttons don't count.
   */
  | {
    kind: "maximum";
  }
  /** The base of the first nose cone. */
  | {
    kind: "nose_base";
  }
  /** A given diameter. */
  | {
    /** Diameter, m. */
    diameter_m: number;
    kind: "custom";
  };

/**
 * A rocket design: its stages, how its reference diameter is chosen, and its motor
 * configurations.
 */
export interface Rocket {
  /**
   * Motor configurations.
   *
   * Absent means `[]`.
   */
  configurations?: Configuration[];
  /**
   * Name.
   *
   * Absent means `""`.
   */
  name?: string;
  /**
   * How the reference diameter is chosen.
   *
   * Absent means `{"kind":"maximum"}`.
   */
  reference_diameter?: ReferenceDiameter;
  /** Stages from the nose aft. The first holds the nose cone. */
  stages: Stage[];
}

/**
 * What separates a stage from the one above it, as `<separationevent>` names it. "This stage" is
 * the stage that carries the setting, the lower one, which drops away.
 */
export type SeparationEvent =
  /** `launch`: at launch, plus the delay. */
  | "launch"
  /** `ignition`: this stage's motor igniting. */
  | "ignition"
  /** `burnout`: this stage's motor burning out. */
  | "burnout"
  /** `ejection`: this stage's ejection charge. */
  | "ejection"
  /** `upperignition`: the motor of the stage above igniting. */
  | "upper_ignition"
  /** `altitudeascending`: a height on the way up. */
  | "altitude_ascending"
  /** `apogee`. */
  | "apogee"
  /** `altitudedescending`: a height on the way down. */
  | "altitude_descending"
  /** `never`. */
  | "never"
  /** A word not listed here, kept as written. */
  | {
    other: string;
  };

/** A shock cord, packed. */
export interface ShockCord {
  /** Length, m. */
  length_m: number;
  /** Material (line). */
  material: Material;
  /** How it is packed. */
  packing: Packing;
}

/** A cylindrical extension of a nose cone or transition that fits inside the adjoining tube. */
export interface Shoulder {
  /**
   * Whether a disc closes the shoulder's far end.
   *
   * Absent means `false`.
   */
  capped?: boolean;
  /** Length, m. */
  length_m: number;
  /** Outer radius, m. */
  outer_radius_m: number;
  /** Wall thickness, m. */
  thickness_m: number;
}

/**
 * A solid rocket motor.
 *
 * Nothing here can tell a hybrid's thrust curve from a solid's, so the checks are at the edges:
 * `crate::catalog::CatalogMotor::motor` refuses hybrids and the `.rse` reader warns about them
 * (`.eng` files don't say).
 */
export interface SolidMotor {
  curve: ThrustCurve;
  dry: MassElement;
  nozzle?: Nozzle | null;
  propellant: Propellant;
}

/**
 * The file a design was read from: its format and its SHA-256, which name it without its path,
 * and whether its rocket was read exactly as written.
 */
export interface Source {
  /**
   * Why the file's airframe or a motor mount was not read exactly as written, if it wasn't: a
   * part left out, a value dropped or simplified, or something assumed
   * (`hpr_io::ork::airframe_not_as_written`). No configuration of such a rocket flies, and
   * `hpr sim` flies no other motor in it. Absent when the rocket was read as written. A document
   * migrated from 0.1 that doesn't show which holds exactly `migrate::UNKNOWN`, a fixed text a
   * program can compare against.
   */
  airframe_not_as_written?: string | null;
  /** The file's format. */
  format: SourceFormat;
  /** The SHA-256 of the file's bytes, as 64 lowercase hexadecimal digits. */
  sha256: string;
}

/**
 * One of the source file's other files: its name, and its contents as text where they are UTF-8,
 * or else as base64 ([RFC 4648][rfc-4648], section 4).
 *
 * Beyond the schema, the reader holds a document's source files to four rules: base64 decodes; no
 * two share a name; each thrust curve a motor names as embedded is among them; and a name is not
 * empty, not `rocket.ork` (the design's own entry) and not a directory's, ending in `/` or `\`.
 *
 * [rfc-4648]: https://www.rfc-editor.org/rfc/rfc4648#section-4
 */
export interface SourceFile {
  /** Its contents. */
  content: Content;
  /** Its name in the source file, such as `thrustcurves/<digest>.rse`. */
  name: string;
}

/** The formats a design can be read from. */
export type SourceFormat = "ork" | "hpr_design";

/** A stage: body components stacked from its forward end aft. */
export interface Stage {
  /** Body components (nose cones, body tubes, transitions), forward to aft. */
  components: Component[];
  /** Unique id. */
  id: string;
  /**
   * Name.
   *
   * Absent means `""`.
   */
  name?: string;
  /**
   * Overrides for the whole stage without its motors, applied after every override inside it. A
   * centre-of-mass override is measured aft of the stage's forward end.
   */
  overrides?: Overrides;
}

/** A stage's separation settings. */
export interface StageSeparation {
  /**
   * When it separates in each configuration that changes that, by `configid`, with anything
   * the configuration leaves out taken from `StageSeparation::separation`.
   */
  configurations: { [key: string]: EventSetting_for_SeparationEvent };
  /** The stage's id in the rocket. */
  id: string;
  /** When it separates, as the stage states it. */
  separation: EventSetting_for_SeparationEvent;
  /** The stage's index in `Rocket::stages`. */
  stage: number;
}

/** The one powered separation a configuration flies: where the stack comes apart and when. */
export interface Staging {
  /** The last stage that stays with the nose; the stages after it drop away. */
  after_stage: number;
  /** When that is, s after launch: known before the flight. */
  time_s: number;
  /** When. */
  trigger: StagingTrigger;
}

/**
 * When a powered separation fires, in the terms of hpr's flight triggers
 * (`hpr_sim::recovery::Trigger`, which this crate does not depend on).
 */
export type StagingTrigger =
  /** At a time after launch, s. */
  | {
    time: {
      /** The time after launch, s. */
      time_s: number;
    };
  }
  /**
   * A delay after the burnout of the motor in a mount (the first tube's, for a cluster, whose
   * tubes light together).
   */
  | {
    burnout: {
      /** The delay after its burnout, s. */
      delay_s: number;
      /** The mount's component id. */
      mount: string;
    };
  };

/** One stage's stored time series. */
export interface StoredBranch {
  /** The `<event>`s, in file order. */
  events: StoredEvent[];
  /** `name`: the stage's name. */
  name: string;
  /**
   * The `<datapoint>` rows, each a value per column, as written: SI, angles in radians, latitude
   * and longitude in degrees. `None` where the file says `NaN`, a quantity OpenRocket did not
   * compute at that step.
   */
  rows: Array<Array<number | null>>;
  /** `types`: each column's name as OpenRocket shows it, such as `Time` or `Altitude`. */
  types: string[];
}

/** An event a stored simulation logged. */
export interface StoredEvent {
  /** `type`, such as `apogee` or `recoverydevicedeployment`. */
  kind: string;
  /** `source`: the id of the component it came from, if any. */
  source?: string | null;
  /** `time`, s. */
  time_s: number;
}

/** What a stored simulation gave: its summary and its time series. */
export interface StoredResults {
  /** The time series, one per stage, in file order. */
  branches: StoredBranch[];
  /** `deploymentvelocity`: the speed at the first deployment, m/s. */
  deployment_speed_m_s?: number | null;
  /** `flighttime`, s. */
  flight_time_s?: number | null;
  /** `groundhitvelocity`, m/s. */
  ground_hit_speed_m_s?: number | null;
  /** `maxacceleration`, m/s². */
  max_acceleration_m_s2?: number | null;
  /** `maxaltitude`, m. */
  max_altitude_m?: number | null;
  /** `maxmach`. */
  max_mach?: number | null;
  /** `maxvelocity`, m/s. */
  max_speed_m_s?: number | null;
  /** `optimumdelay`: the ejection delay that would have fired at apogee, s. */
  optimum_delay_s?: number | null;
  /** `launchrodvelocity`: the speed leaving the rod, m/s. */
  rod_exit_speed_m_s?: number | null;
  /** `timetoapogee`, s. */
  time_to_apogee_s?: number | null;
  /** The `<warning>`s OpenRocket stored with the results, each as its text. */
  warnings: string[];
}

/** A simulation stored in a `.ork`. */
export interface StoredSimulation {
  /** `<calculator>`, such as `BarrowmanCalculator`. */
  calculator?: string | null;
  /** `<conditions>`: what the run was flown in. */
  conditions?: LaunchConditions | null;
  /** `<name>`. */
  name: string;
  /**
   * Whether reading this simulation required dropping or reinterpreting stored data.
   *
   * Absent means `false`.
   */
  parser_warnings?: boolean;
  /** `<flightdata>`: what it gave, when OpenRocket saved it. */
  results?: StoredResults | null;
  /** `<simulator>`, such as `RK4Simulator`. */
  simulator?: string | null;
  /**
   * The `status` attribute as written: `uptodate`, `outdated`, `loaded`, `external`,
   * `notsimulated`, `cantrun` or `aborted` in the files OpenRocket 24.12 writes.
   */
  status?: string | null;
}

/** A streamer, packed. */
export interface Streamer {
  /** Length, m. */
  length_m: number;
  /** Material (surface). */
  material: Material;
  /** How it is packed. */
  packing: Packing;
  /** Width, m. */
  width_m: number;
}

/**
 * A thrust curve: thrust in newtons against time in seconds from ignition, joined by straight
 * lines.
 *
 * Built through `ThrustCurve::new`, which checks the samples; it serializes as its samples and
 * re-checks them when deserialized.
 */
export interface ThrustCurve {
  thrusts_n: number[];
  times_s: number[];
}

/** A transition between two radii, with optional shoulders at either end. */
export interface Transition {
  /** Radius at the aft end, m. */
  aft_radius_m: number;
  /**
   * Optional shoulder aft of the aft end.
   *
   * Absent means `null`.
   */
  aft_shoulder?: Shoulder | null;
  /**
   * Whether the profile is clipped from a longer nose cone (`crate::shapes`).
   *
   * Absent means `false`.
   */
  clipped?: boolean;
  /** Radius at the forward end, m. */
  fore_radius_m: number;
  /**
   * Optional shoulder forward of the fore end.
   *
   * Absent means `null`.
   */
  fore_shoulder?: Shoulder | null;
  /** Length, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Profile shape. */
  shape: NoseShape;
  /** Filled, or a wall of a thickness. */
  wall: Wall;
}

/** Tube fins: open tubes parallel to the body, touching it, spaced evenly around it. */
export interface TubeFinSet {
  /**
   * Roll angle of the first tube's axis from `x_B` toward `y_B`, rad.
   *
   * Absent means `0.0`.
   */
  base_angle_rad?: number;
  /** Number of tubes, at least 1. */
  count: number;
  /** Length, m. */
  length_m: number;
  /** Material (bulk). */
  material: Material;
  /** Outer radius, m. */
  outer_radius_m: number;
  /** Wall thickness, m. */
  thickness_m: number;
}

/** A parachute or streamer inside a part hpr does not read, such as a pod. */
export interface UnreadDevice {
  /** Where it is in the file. */
  at: string;
  /**
   * The tag of the outermost part that was not read: a `podset` or `parallelstage`, or else
   * the device's own tag.
   */
  inside: string;
  /** `parachute` or `streamer`. */
  tag: string;
}

/** A `<motor>` inside a part hpr does not read, such as a pod's mount. */
export interface UnreadMotor {
  /** Where its mount is in the file. */
  at: string;
  /** `<designation>`. */
  designation: string;
  /**
   * The tag of the part that was not read: the outermost `podset` or `parallelstage` around
   * the mount, or else the mount's own tag.
   */
  inside: string;
  /** Why its mount was not read, in words. */
  reason: string;
}

/** A version of the format, `major.minor`. */
export type Version = string;

/** Whether a solid of revolution is filled or a wall. */
export type Wall =
  /** Solid all the way to the axis. */
  | {
    kind: "filled";
  }
  /**
   * A wall of constant thickness measured normal to the outer surface. A thickness of zero is a
   * surface with no wall: the part keeps its shape and weighs nothing, which is what OpenRocket
   * makes of a part written with no wall ([ADR-061][adr-061]).
   *
   * [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
   */
  | {
    kind: "shell";
    /** Wall thickness, m; zero or more. */
    thickness_m: number;
  };

/** One level of a multilevel wind. */
export interface WindLevel {
  /**
   * Its altitude, m, above the ground or the sea as `LaunchConditions::wind_levels_above`
   * says.
   */
  altitude_m?: number | null;
  /** The bearing it blows from, rad. */
  from_rad?: number | null;
  /** Its mean speed, m/s. */
  speed_m_s?: number | null;
  /** The standard deviation of its speed, m/s. */
  standard_deviation_m_s?: number | null;
}

// The schema the reader checks a document against: the committed schema without its prose.
const SCHEMA: SchemaNode = {"$defs":{"Atmosphere":{"oneOf":[{"additionalProperties":false,"properties":{"model":{"const":"isa","type":"string"}},"required":["model"],"type":"object"},{"additionalProperties":false,"properties":{"model":{"const":"extended","type":"string"},"pressure_pa":{"type":["number","null"]},"temperature_k":{"type":["number","null"]}},"required":["model"],"type":"object"},{"additionalProperties":false,"properties":{"model":{"const":"other","type":"string"},"name":{"type":"string"}},"required":["model","name"],"type":"object"}]},"AutoDimension":{"oneOf":[{"const":"base_radius","type":"string"},{"const":"outer_radius","type":"string"},{"const":"fore_radius","type":"string"},{"const":"aft_radius","type":"string"},{"const":"shoulder_radius","type":"string"},{"const":"fore_shoulder_radius","type":"string"},{"const":"aft_shoulder_radius","type":"string"},{"const":"inner_radius","type":"string"},{"const":"packed_radius","type":"string"}]},"BodyTube":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"CenteringRing":{"additionalProperties":false,"properties":{"inner_radius_m":{"type":"number"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"}},"required":["length_m","outer_radius_m","inner_radius_m","material"],"type":"object"},"Component":{"additionalProperties":false,"properties":{"auto":{"items":{"$ref":"#/$defs/AutoDimension"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Component"},"type":"array"},"finish":{"anyOf":[{"$ref":"#/$defs/Finish"},{"type":"null"}]},"id":{"type":"string"},"motor_mount":{"anyOf":[{"$ref":"#/$defs/MotorMount"},{"type":"null"}]},"name":{"type":"string"},"overrides":{"$ref":"#/$defs/Overrides"},"overrides_include_children":{"type":"boolean"},"part":{"$ref":"#/$defs/Part"},"position":{"anyOf":[{"$ref":"#/$defs/Position"},{"type":"null"}]}},"required":["id","part"],"type":"object"},"Configuration":{"additionalProperties":false,"properties":{"id":{"type":"string"},"motors":{"items":{"$ref":"#/$defs/MountedMotor"},"type":"array"},"name":{"type":"string"}},"required":["id","motors"],"type":"object"},"Content":{"oneOf":[{"additionalProperties":false,"properties":{"text":{"type":"string"}},"required":["text"],"type":"object"},{"additionalProperties":false,"properties":{"base64":{"type":"string"}},"required":["base64"],"type":"object"}]},"Curve":{"oneOf":[{"additionalProperties":false,"properties":{"entry":{"type":"string"},"motor":{"$ref":"#/$defs/SolidMotor"},"source":{"const":"embedded","type":"string"}},"required":["source","entry","motor"],"type":"object"},{"additionalProperties":false,"properties":{"digest":{"type":"string"},"from":{"type":"string"},"motor":{"$ref":"#/$defs/SolidMotor"},"source":{"const":"supplied","type":"string"}},"required":["source","digest","from","motor"],"type":"object"},{"additionalProperties":false,"properties":{"motor":{"$ref":"#/$defs/SolidMotor"},"motor_id":{"type":"string"},"simfile_id":{"type":"string"},"source":{"const":"catalog","type":"string"}},"required":["source","motor_id","simfile_id","motor"],"type":"object"},{"additionalProperties":false,"properties":{"reason":{"type":"string"},"source":{"const":"unresolved","type":"string"},"why":{"$ref":"#/$defs/NoCurve"}},"required":["source","why","reason"],"type":"object"}]},"Delay":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"seconds","type":"string"},"value":{"type":"number"}},"required":["kind","value"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"plugged","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"zero_or_plugged","type":"string"}},"required":["kind"],"type":"object"}]},"Density":{"oneOf":[{"additionalProperties":false,"properties":{"kg_m3":{"type":"number"},"kind":{"const":"bulk","type":"string"}},"required":["kind","kg_m3"],"type":"object"},{"additionalProperties":false,"properties":{"kg_m2":{"type":"number"},"kind":{"const":"surface","type":"string"}},"required":["kind","kg_m2"],"type":"object"},{"additionalProperties":false,"properties":{"kg_m":{"type":"number"},"kind":{"const":"line","type":"string"}},"required":["kind","kg_m"],"type":"object"}]},"DeployEvent":{"oneOf":[{"const":"launch","type":"string"},{"const":"ejection","type":"string"},{"const":"apogee","type":"string"},{"const":"altitude","type":"string"},{"const":"lower_stage_separation","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"DeviceKind":{"oneOf":[{"const":"parachute","type":"string"},{"const":"streamer","type":"string"}]},"Dimension":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"stated","type":"string"},"value":{"type":"number"}},"required":["kind","value"],"type":"object"},{"additionalProperties":false,"properties":{"cached":{"type":["number","null"]},"kind":{"const":"automatic","type":"string"}},"required":["kind"],"type":"object"}]},"Element":{"additionalProperties":false,"properties":{"attributes":{"items":{"maxItems":2,"minItems":2,"prefixItems":[{"type":"string"},{"type":"string"}],"type":"array"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Node"},"type":"array"},"name":{"type":"string"}},"required":["name","attributes","children"],"type":"object"},"EventSetting_for_DeployEvent":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"delay_s":{"type":["number","null"]},"event":{"anyOf":[{"$ref":"#/$defs/DeployEvent"},{"type":"null"}]}},"type":"object"},"EventSetting_for_SeparationEvent":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"delay_s":{"type":["number","null"]},"event":{"anyOf":[{"$ref":"#/$defs/SeparationEvent"},{"type":"null"}]}},"type":"object"},"Extensions":{"additionalProperties":false,"properties":{"x-openrocket":{"$ref":"#/$defs/OpenRocketExtension"}},"type":"object"},"FinCrossSection":{"oneOf":[{"const":"square","type":"string"},{"const":"rounded","type":"string"},{"const":"airfoil","type":"string"}]},"FinFillet":{"additionalProperties":false,"properties":{"material":{"$ref":"#/$defs/Material"},"radius_m":{"type":"number"}},"required":["radius_m","material"],"type":"object"},"FinPlanform":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"trapezoidal","type":"string"},"root_chord_m":{"type":"number"},"span_m":{"type":"number"},"sweep_m":{"type":"number"},"tip_chord_m":{"type":"number"}},"required":["kind","root_chord_m","tip_chord_m","span_m","sweep_m"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"elliptical","type":"string"},"root_chord_m":{"type":"number"},"span_m":{"type":"number"}},"required":["kind","root_chord_m","span_m"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"freeform","type":"string"},"points_m":{"items":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":"array"},"type":"array"}},"required":["kind","points_m"],"type":"object"}]},"FinSet":{"additionalProperties":false,"properties":{"base_angle_rad":{"type":"number"},"cant_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"cross_section":{"$ref":"#/$defs/FinCrossSection"},"fillet":{"anyOf":[{"$ref":"#/$defs/FinFillet"},{"type":"null"}]},"material":{"$ref":"#/$defs/Material"},"planform":{"$ref":"#/$defs/FinPlanform"},"tab":{"anyOf":[{"$ref":"#/$defs/FinTab"},{"type":"null"}]},"thickness_m":{"type":"number"}},"required":["count","planform","thickness_m","material"],"type":"object"},"FinTab":{"additionalProperties":false,"properties":{"height_m":{"type":"number"},"length_m":{"type":"number"},"offset_m":{"type":"number"}},"required":["height_m","length_m","offset_m"],"type":"object"},"Finish":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"mirror","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"average_glass","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"polished","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"sheet_metal","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"optimum_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"planed_wood","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"mass_production_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"bare_steel","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"smooth_cement","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"asphalt_coating","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"dip_galvanized","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"poor_paint","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"cast_iron","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"raw_wood","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"concrete","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"custom","type":"string"},"roughness_m":{"type":"number"}},"required":["kind","roughness_m"],"type":"object"}]},"Format":{"oneOf":[{"const":"hpr-design","type":"string"}]},"Ignition":{"oneOf":[{"const":"launch","type":"string"},{"additionalProperties":false,"properties":{"time":{"additionalProperties":false,"properties":{"time_s":{"type":"number"}},"required":["time_s"],"type":"object"}},"required":["time"],"type":"object"},{"additionalProperties":false,"properties":{"burnout":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"mount":{"type":"string"}},"required":["mount","delay_s"],"type":"object"}},"required":["burnout"],"type":"object"},{"additionalProperties":false,"properties":{"separation":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"}},"required":["delay_s"],"type":"object"}},"required":["separation"],"type":"object"},{"const":"never","type":"string"}]},"IgnitionEvent":{"oneOf":[{"const":"automatic","type":"string"},{"const":"launch","type":"string"},{"const":"ejection_charge","type":"string"},{"const":"burnout","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"InertiaOverride":{"additionalProperties":false,"properties":{"xx_kg_m2":{"type":"number"},"xy_kg_m2":{"type":"number"},"xz_kg_m2":{"type":"number"},"yy_kg_m2":{"type":"number"},"yz_kg_m2":{"type":"number"},"zz_kg_m2":{"type":"number"}},"required":["xx_kg_m2","yy_kg_m2","zz_kg_m2"],"type":"object"},"InnerTube":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"cluster_m":{"items":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":"array"},"type":"array"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"radial_offset_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"Kept":{"additionalProperties":false,"properties":{"at":{"type":"string"},"element":{"$ref":"#/$defs/Element"}},"required":["at","element"],"type":"object"},"KeptAttribute":{"additionalProperties":false,"properties":{"at":{"type":"string"},"name":{"type":"string"},"value":{"type":"string"}},"required":["at","name","value"],"type":"object"},"LaunchConditions":{"additionalProperties":false,"properties":{"atmosphere":{"anyOf":[{"$ref":"#/$defs/Atmosphere"},{"type":"null"}]},"configuration":{"type":["string","null"]},"geodetic_method":{"type":["string","null"]},"into_wind":{"type":["boolean","null"]},"latitude_deg":{"type":["number","null"]},"launch_altitude_m":{"type":["number","null"]},"longitude_deg":{"type":["number","null"]},"max_time_s":{"type":["number","null"]},"rod_angle_rad":{"type":["number","null"]},"rod_direction_rad":{"type":["number","null"]},"rod_length_m":{"type":["number","null"]},"time_step_s":{"type":["number","null"]},"wind_from_rad":{"type":["number","null"]},"wind_levels":{"items":{"$ref":"#/$defs/WindLevel"},"type":"array"},"wind_levels_above":{"type":["string","null"]},"wind_model":{"type":["string","null"]},"wind_speed_m_s":{"type":["number","null"]},"wind_turbulence":{"type":["number","null"]}},"required":["wind_levels"],"type":"object"},"LaunchLug":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"spacing_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m","material"],"type":"object"},"LeftOut":{"additionalProperties":false,"properties":{"message":{"type":"string"},"why":{"$ref":"#/$defs/NotFlown"}},"required":["why","message"],"type":"object"},"MassComponent":{"additionalProperties":false,"properties":{"mass_kg":{"type":"number"},"packing":{"$ref":"#/$defs/Packing"}},"required":["mass_kg","packing"],"type":"object"},"MassElement":{"additionalProperties":false,"properties":{"axial_inertia_kg_m2":{"type":"number"},"cg_m":{"type":"number"},"mass_kg":{"type":"number"},"transverse_inertia_kg_m2":{"type":"number"}},"required":["mass_kg","cg_m","axial_inertia_kg_m2","transverse_inertia_kg_m2"],"type":"object"},"Material":{"additionalProperties":false,"properties":{"density":{"$ref":"#/$defs/Density"},"name":{"type":"string"}},"required":["name","density"],"type":"object"},"MotorConfiguration":{"additionalProperties":false,"properties":{"declared":{"type":"boolean"},"default":{"type":"boolean"},"id":{"type":"string"},"inactive_stages":{"items":{"format":"uint32","minimum":0,"type":["integer","null"]},"type":"array"},"left_out":{"anyOf":[{"$ref":"#/$defs/LeftOut"},{"type":"null"}]},"motors":{"items":{"$ref":"#/$defs/OrkMotor"},"type":"array"},"name":{"type":"string"},"staging":{"anyOf":[{"$ref":"#/$defs/Staging"},{"type":"null"}]},"unread":{"items":{"$ref":"#/$defs/UnreadMotor"},"type":"array"}},"required":["id","name","default","declared","inactive_stages","motors","unread"],"type":"object"},"MotorMount":{"additionalProperties":false,"properties":{"overhang_m":{"type":"number"}},"type":"object"},"Motors":{"additionalProperties":false,"properties":{"configurations":{"items":{"$ref":"#/$defs/MotorConfiguration"},"type":"array"}},"required":["configurations"],"type":"object"},"MountedMotor":{"additionalProperties":false,"properties":{"delay":{"anyOf":[{"$ref":"#/$defs/Delay"},{"type":"null"}]},"designation":{"type":"string"},"diameter_m":{"type":"number"},"failed_tubes":{"items":{"format":"uint","minimum":0,"type":"integer"},"type":"array"},"ignition":{"$ref":"#/$defs/Ignition"},"length_m":{"type":"number"},"motor":{"$ref":"#/$defs/SolidMotor"},"mount":{"type":"string"}},"required":["mount","diameter_m","length_m","motor"],"type":"object"},"NoCurve":{"oneOf":[{"const":"hybrid","type":"string"},{"const":"no_designation","type":"string"},{"const":"not_found","type":"string"},{"const":"ambiguous","type":"string"},{"const":"unusable","type":"string"}]},"Node":{"oneOf":[{"additionalProperties":false,"properties":{"attributes":{"items":{"maxItems":2,"minItems":2,"prefixItems":[{"type":"string"},{"type":"string"}],"type":"array"},"type":"array"},"children":{"items":{"$ref":"#/$defs/Node"},"type":"array"},"kind":{"const":"element","type":"string"},"name":{"type":"string"}},"required":["kind","name","attributes","children"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"text","type":"string"},"text":{"type":"string"}},"required":["kind","text"],"type":"object"}]},"NoseCone":{"additionalProperties":false,"properties":{"base_radius_m":{"type":"number"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"shape":{"$ref":"#/$defs/NoseShape"},"shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"wall":{"$ref":"#/$defs/Wall"}},"required":["shape","length_m","base_radius_m","wall","material"],"type":"object"},"NoseShape":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"conical","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"ogive","type":"string"},"radius_ratio":{"type":"number"}},"required":["kind","radius_ratio"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"elliptical","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"exponent":{"type":"number"},"kind":{"const":"power_series","type":"string"}},"required":["kind","exponent"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"parabolic_series","type":"string"},"parameter":{"type":"number"}},"required":["kind","parameter"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"haack","type":"string"},"parameter":{"type":"number"}},"required":["kind","parameter"],"type":"object"}]},"NotFlown":{"oneOf":[{"const":"unread_motor","type":"string"},{"const":"no_motor","type":"string"},{"const":"inactive_stage","type":"string"},{"const":"no_curve","type":"string"},{"const":"no_size","type":"string"},{"const":"ignition_not_flown","type":"string"},{"const":"airframe_not_as_written","type":"string"},{"const":"separation_not_flown","type":"string"}]},"Nozzle":{"additionalProperties":false,"properties":{"exit_radius_m":{"type":"number"},"reference_pressure_pa":{"type":["number","null"]},"throat_radius_m":{"type":["number","null"]}},"required":["exit_radius_m","reference_pressure_pa"],"type":"object"},"OpenRocketExtension":{"additionalProperties":false,"properties":{"attributes":{"items":{"$ref":"#/$defs/KeptAttribute"},"type":"array"},"parts":{"items":{"$ref":"#/$defs/Kept"},"type":"array"},"sections":{"items":{"$ref":"#/$defs/Kept"},"type":"array"},"tags":{"items":{"$ref":"#/$defs/Kept"},"type":"array"}},"type":"object"},"OrkIgnition":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"event":{"$ref":"#/$defs/IgnitionEvent"}},"required":["event","delay_s"],"type":"object"},"OrkMotor":{"additionalProperties":false,"properties":{"curve":{"$ref":"#/$defs/Curve"},"delay":{"anyOf":[{"$ref":"#/$defs/Delay"},{"type":"null"}]},"designation":{"type":"string"},"diameter_m":{"type":["number","null"]},"digest":{"type":["string","null"]},"ignition":{"$ref":"#/$defs/OrkIgnition"},"kind":{"type":["string","null"]},"length_m":{"type":["number","null"]},"manufacturer":{"type":"string"},"mount":{"type":"string"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["mount","stage","manufacturer","designation","ignition","curve"],"type":"object"},"Overrides":{"additionalProperties":false,"properties":{"cg_aft_m":{"type":["number","null"]},"cg_xy_m":{"items":{"type":"number"},"maxItems":2,"minItems":2,"type":["array","null"]},"inertia":{"anyOf":[{"$ref":"#/$defs/InertiaOverride"},{"type":"null"}]},"mass_kg":{"type":["number","null"]}},"type":"object"},"Packing":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"length_m":{"type":"number"},"radial_offset_m":{"type":"number"},"radius_m":{"type":"number"}},"required":["length_m","radius_m"],"type":"object"},"Parachute":{"additionalProperties":false,"properties":{"canopy_material":{"$ref":"#/$defs/Material"},"diameter_m":{"type":"number"},"line_count":{"format":"uint32","minimum":0,"type":"integer"},"line_length_m":{"type":"number"},"line_material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"}},"required":["diameter_m","canopy_material","line_count","line_length_m","line_material","packing"],"type":"object"},"Part":{"oneOf":[{"additionalProperties":false,"properties":{"nose_cone":{"$ref":"#/$defs/NoseCone"}},"required":["nose_cone"],"type":"object"},{"additionalProperties":false,"properties":{"body_tube":{"$ref":"#/$defs/BodyTube"}},"required":["body_tube"],"type":"object"},{"additionalProperties":false,"properties":{"transition":{"$ref":"#/$defs/Transition"}},"required":["transition"],"type":"object"},{"additionalProperties":false,"properties":{"inner_tube":{"$ref":"#/$defs/InnerTube"}},"required":["inner_tube"],"type":"object"},{"additionalProperties":false,"properties":{"centering_ring":{"$ref":"#/$defs/CenteringRing"}},"required":["centering_ring"],"type":"object"},{"additionalProperties":false,"properties":{"fin_set":{"$ref":"#/$defs/FinSet"}},"required":["fin_set"],"type":"object"},{"additionalProperties":false,"properties":{"tube_fin_set":{"$ref":"#/$defs/TubeFinSet"}},"required":["tube_fin_set"],"type":"object"},{"additionalProperties":false,"properties":{"launch_lug":{"$ref":"#/$defs/LaunchLug"}},"required":["launch_lug"],"type":"object"},{"additionalProperties":false,"properties":{"rail_button":{"$ref":"#/$defs/RailButton"}},"required":["rail_button"],"type":"object"},{"additionalProperties":false,"properties":{"pod_set":{"$ref":"#/$defs/PodSet"}},"required":["pod_set"],"type":"object"},{"additionalProperties":false,"properties":{"mass_component":{"$ref":"#/$defs/MassComponent"}},"required":["mass_component"],"type":"object"},{"additionalProperties":false,"properties":{"parachute":{"$ref":"#/$defs/Parachute"}},"required":["parachute"],"type":"object"},{"additionalProperties":false,"properties":{"streamer":{"$ref":"#/$defs/Streamer"}},"required":["streamer"],"type":"object"},{"additionalProperties":false,"properties":{"shock_cord":{"$ref":"#/$defs/ShockCord"}},"required":["shock_cord"],"type":"object"}]},"PodSet":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"radial_offset_m":{"type":"number"}},"required":["count","radial_offset_m"],"type":"object"},"Position":{"oneOf":[{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"top","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"middle","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"bottom","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"aft_offset_m":{"type":"number"},"from":{"const":"after","type":"string"}},"required":["from"],"type":"object"},{"additionalProperties":false,"properties":{"from":{"const":"absolute","type":"string"},"station_m":{"type":"number"}},"required":["from","station_m"],"type":"object"}]},"Propellant":{"oneOf":[{"additionalProperties":false,"properties":{"center_m":{"type":"number"},"inner_radius_m":{"type":"number"},"length_m":{"type":"number"},"mass_kg":{"type":"number"},"model":{"const":"column","type":"string"},"outer_radius_m":{"type":"number"}},"required":["model","mass_kg","center_m","outer_radius_m","inner_radius_m","length_m"],"type":"object"},{"additionalProperties":false,"properties":{"center_m":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"density_kg_m3":{"type":"number"},"inhibited_ends":{"type":"boolean"},"initial_height_m":{"type":"number"},"initial_inner_radius_m":{"type":"number"},"model":{"const":"grains","type":"string"},"outer_radius_m":{"type":"number"},"separation_m":{"type":"number"}},"required":["model","count","density_kg_m3","outer_radius_m","initial_inner_radius_m","initial_height_m","separation_m","center_m","inhibited_ends"],"type":"object"}]},"Provenance":{"additionalProperties":false,"properties":{"source":{"anyOf":[{"$ref":"#/$defs/Source"},{"type":"null"}]},"tool":{"type":"string"},"tool_version":{"type":"string"}},"required":["tool","tool_version"],"type":"object"},"RailButton":{"additionalProperties":false,"properties":{"angle_rad":{"type":"number"},"base_height_m":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"flange_height_m":{"type":"number"},"height_m":{"type":"number"},"inner_diameter_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_diameter_m":{"type":"number"},"spacing_m":{"type":"number"}},"required":["outer_diameter_m","inner_diameter_m","height_m","base_height_m","flange_height_m","material"],"type":"object"},"Recovery":{"additionalProperties":false,"properties":{"devices":{"items":{"$ref":"#/$defs/RecoveryDevice"},"type":"array"},"separations":{"items":{"$ref":"#/$defs/StageSeparation"},"type":"array"},"unread":{"items":{"$ref":"#/$defs/UnreadDevice"},"type":"array"},"unread_separations":{"items":{"$ref":"#/$defs/UnreadDevice"},"type":"array"}},"required":["devices","separations","unread","unread_separations"],"type":"object"},"RecoveryDevice":{"additionalProperties":false,"properties":{"cd":{"anyOf":[{"$ref":"#/$defs/Dimension"},{"type":"null"}]},"configurations":{"additionalProperties":{"$ref":"#/$defs/EventSetting_for_DeployEvent"},"type":"object"},"deployment":{"$ref":"#/$defs/EventSetting_for_DeployEvent"},"id":{"type":"string"},"kind":{"$ref":"#/$defs/DeviceKind"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["id","stage","kind","deployment","configurations"],"type":"object"},"ReferenceDiameter":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"maximum","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"nose_base","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"diameter_m":{"type":"number"},"kind":{"const":"custom","type":"string"}},"required":["kind","diameter_m"],"type":"object"}]},"Rocket":{"additionalProperties":false,"properties":{"configurations":{"items":{"$ref":"#/$defs/Configuration"},"type":"array"},"name":{"type":"string"},"reference_diameter":{"$ref":"#/$defs/ReferenceDiameter"},"stages":{"items":{"$ref":"#/$defs/Stage"},"type":"array"}},"required":["stages"],"type":"object"},"SeparationEvent":{"oneOf":[{"const":"launch","type":"string"},{"const":"ignition","type":"string"},{"const":"burnout","type":"string"},{"const":"ejection","type":"string"},{"const":"upper_ignition","type":"string"},{"const":"altitude_ascending","type":"string"},{"const":"apogee","type":"string"},{"const":"altitude_descending","type":"string"},{"const":"never","type":"string"},{"additionalProperties":false,"properties":{"other":{"type":"string"}},"required":["other"],"type":"object"}]},"ShockCord":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"}},"required":["length_m","material","packing"],"type":"object"},"Shoulder":{"additionalProperties":false,"properties":{"capped":{"type":"boolean"},"length_m":{"type":"number"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["length_m","outer_radius_m","thickness_m"],"type":"object"},"SolidMotor":{"additionalProperties":false,"properties":{"curve":{"$ref":"#/$defs/ThrustCurve"},"dry":{"$ref":"#/$defs/MassElement"},"nozzle":{"anyOf":[{"$ref":"#/$defs/Nozzle"},{"type":"null"}]},"propellant":{"$ref":"#/$defs/Propellant"}},"required":["curve","propellant","dry"],"type":"object"},"Source":{"additionalProperties":false,"properties":{"airframe_not_as_written":{"type":["string","null"]},"format":{"$ref":"#/$defs/SourceFormat"},"sha256":{"pattern":"^[0-9a-f]{64}$","type":"string"}},"required":["format","sha256"],"type":"object"},"SourceFile":{"additionalProperties":false,"properties":{"content":{"$ref":"#/$defs/Content"},"name":{"type":"string"}},"required":["name","content"],"type":"object"},"SourceFormat":{"oneOf":[{"const":"ork","type":"string"},{"const":"hpr_design","type":"string"}]},"Stage":{"additionalProperties":false,"properties":{"components":{"items":{"$ref":"#/$defs/Component"},"type":"array"},"id":{"type":"string"},"name":{"type":"string"},"overrides":{"$ref":"#/$defs/Overrides"}},"required":["id","components"],"type":"object"},"StageSeparation":{"additionalProperties":false,"properties":{"configurations":{"additionalProperties":{"$ref":"#/$defs/EventSetting_for_SeparationEvent"},"type":"object"},"id":{"type":"string"},"separation":{"$ref":"#/$defs/EventSetting_for_SeparationEvent"},"stage":{"format":"uint","minimum":0,"type":"integer"}},"required":["id","stage","separation","configurations"],"type":"object"},"Staging":{"additionalProperties":false,"properties":{"after_stage":{"format":"uint","minimum":0,"type":"integer"},"time_s":{"type":"number"},"trigger":{"$ref":"#/$defs/StagingTrigger"}},"required":["after_stage","trigger","time_s"],"type":"object"},"StagingTrigger":{"oneOf":[{"additionalProperties":false,"properties":{"time":{"additionalProperties":false,"properties":{"time_s":{"type":"number"}},"required":["time_s"],"type":"object"}},"required":["time"],"type":"object"},{"additionalProperties":false,"properties":{"burnout":{"additionalProperties":false,"properties":{"delay_s":{"type":"number"},"mount":{"type":"string"}},"required":["mount","delay_s"],"type":"object"}},"required":["burnout"],"type":"object"}]},"StoredBranch":{"additionalProperties":false,"properties":{"events":{"items":{"$ref":"#/$defs/StoredEvent"},"type":"array"},"name":{"type":"string"},"rows":{"items":{"items":{"type":["number","null"]},"type":"array"},"type":"array"},"types":{"items":{"type":"string"},"type":"array"}},"required":["name","types","rows","events"],"type":"object"},"StoredEvent":{"additionalProperties":false,"properties":{"kind":{"type":"string"},"source":{"type":["string","null"]},"time_s":{"type":"number"}},"required":["time_s","kind"],"type":"object"},"StoredResults":{"additionalProperties":false,"properties":{"branches":{"items":{"$ref":"#/$defs/StoredBranch"},"type":"array"},"deployment_speed_m_s":{"type":["number","null"]},"flight_time_s":{"type":["number","null"]},"ground_hit_speed_m_s":{"type":["number","null"]},"max_acceleration_m_s2":{"type":["number","null"]},"max_altitude_m":{"type":["number","null"]},"max_mach":{"type":["number","null"]},"max_speed_m_s":{"type":["number","null"]},"optimum_delay_s":{"type":["number","null"]},"rod_exit_speed_m_s":{"type":["number","null"]},"time_to_apogee_s":{"type":["number","null"]},"warnings":{"items":{"type":"string"},"type":"array"}},"required":["branches","warnings"],"type":"object"},"StoredSimulation":{"additionalProperties":false,"properties":{"calculator":{"type":["string","null"]},"conditions":{"anyOf":[{"$ref":"#/$defs/LaunchConditions"},{"type":"null"}]},"name":{"type":"string"},"parser_warnings":{"type":"boolean"},"results":{"anyOf":[{"$ref":"#/$defs/StoredResults"},{"type":"null"}]},"simulator":{"type":["string","null"]},"status":{"type":["string","null"]}},"required":["name"],"type":"object"},"Streamer":{"additionalProperties":false,"properties":{"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"packing":{"$ref":"#/$defs/Packing"},"width_m":{"type":"number"}},"required":["length_m","width_m","material","packing"],"type":"object"},"ThrustCurve":{"additionalProperties":false,"properties":{"thrusts_n":{"items":{"type":"number"},"type":"array"},"times_s":{"items":{"type":"number"},"type":"array"}},"required":["times_s","thrusts_n"],"type":"object"},"Transition":{"additionalProperties":false,"properties":{"aft_radius_m":{"type":"number"},"aft_shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"clipped":{"type":"boolean"},"fore_radius_m":{"type":"number"},"fore_shoulder":{"anyOf":[{"$ref":"#/$defs/Shoulder"},{"type":"null"}]},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"shape":{"$ref":"#/$defs/NoseShape"},"wall":{"$ref":"#/$defs/Wall"}},"required":["shape","length_m","fore_radius_m","aft_radius_m","wall","material"],"type":"object"},"TubeFinSet":{"additionalProperties":false,"properties":{"base_angle_rad":{"type":"number"},"count":{"format":"uint32","minimum":0,"type":"integer"},"length_m":{"type":"number"},"material":{"$ref":"#/$defs/Material"},"outer_radius_m":{"type":"number"},"thickness_m":{"type":"number"}},"required":["count","length_m","outer_radius_m","thickness_m","material"],"type":"object"},"UnreadDevice":{"additionalProperties":false,"properties":{"at":{"type":"string"},"inside":{"type":"string"},"tag":{"type":"string"}},"required":["at","tag","inside"],"type":"object"},"UnreadMotor":{"additionalProperties":false,"properties":{"at":{"type":"string"},"designation":{"type":"string"},"inside":{"type":"string"},"reason":{"type":"string"}},"required":["at","designation","inside","reason"],"type":"object"},"Version":{"pattern":"^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$","type":"string"},"Wall":{"oneOf":[{"additionalProperties":false,"properties":{"kind":{"const":"filled","type":"string"}},"required":["kind"],"type":"object"},{"additionalProperties":false,"properties":{"kind":{"const":"shell","type":"string"},"thickness_m":{"type":"number"}},"required":["kind","thickness_m"],"type":"object"}]},"WindLevel":{"additionalProperties":false,"properties":{"altitude_m":{"type":["number","null"]},"from_rad":{"type":["number","null"]},"speed_m_s":{"type":["number","null"]},"standard_deviation_m_s":{"type":["number","null"]}},"type":"object"}},"additionalProperties":false,"properties":{"extensions":{"$ref":"#/$defs/Extensions"},"format":{"$ref":"#/$defs/Format"},"motors":{"$ref":"#/$defs/Motors"},"provenance":{"$ref":"#/$defs/Provenance"},"recovery":{"$ref":"#/$defs/Recovery"},"rocket":{"$ref":"#/$defs/Rocket"},"simulations":{"items":{"$ref":"#/$defs/StoredSimulation"},"type":"array"},"source_files":{"items":{"$ref":"#/$defs/SourceFile"},"type":"array"},"version":{"$ref":"#/$defs/Version"}},"required":["format","version","provenance","rocket","motors","recovery","simulations","extensions","source_files"],"type":"object"};

/** A document the reader refused: not JSON, not an hpr design, another version, or not valid. */
export class DesignFormatError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DesignFormatError";
    // Keeps `instanceof DesignFormatError` true when compiled for ES5.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Reads a design document (`.hpr` text) and checks it against the format's schema, so what comes
 * back has the types above.
 *
 * It checks what the schema says: every required key present, no unknown key, each value of its
 * type, each tagged union one of its forms. hpr's own reader checks a few things more that no
 * schema can say, such as that two source files don't share a name, so hpr can still refuse a
 * document this takes. Like hpr, it refuses a number too large for a 64-bit float, a lone UTF-16
 * surrogate (`"\ud800"`), and nesting 128 levels deep. `JSON.parse` keeps the last of two equal
 * keys, where hpr refuses them, and can't tell `2.0` from `2`, which hpr refuses where it wants a
 * whole number; and a whole number of 2^53 or more, which it would round, is refused.
 *
 * @throws {DesignFormatError} The document is not one of this version.
 */
export function readDesign(text: string): DesignFile {
  let value: unknown;
  try {
    // A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
    value = JSON.parse(text.startsWith("\uFEFF") ? text.slice(1) : text);
  } catch (error) {
    throw new DesignFormatError(`not JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  const unread = scan(value);
  if (unread !== null) {
    throw new DesignFormatError(`not JSON: ${unread}`);
  }
  if (!isObject(value) || value.format !== FORMAT) {
    throw new DesignFormatError(`not an hpr design: its "format" is not "${FORMAT}"`);
  }
  if (value.version !== VERSION) {
    throw new DesignFormatError(versionMessage(value.version));
  }
  const problem = check(value, SCHEMA, "$");
  if (problem !== null) {
    throw new DesignFormatError(`${problem.path}: ${problem.message}`);
  }
  return value as unknown as DesignFile;
}

/** Why a document of `version`, which isn't this one, is refused, and what to do. */
function versionMessage(version: unknown): string {
  const match = typeof version === "string" ? /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.exec(version) : null;
  if (match === null) {
    return `its "version" is ${shown(version)}, not a version such as "${VERSION}"`;
  }
  const [major, minor] = VERSION.split(".").map(Number);
  const [theirMajor, theirMinor] = [Number(match[1]), Number(match[2])];
  if (theirMajor > major || (theirMajor === major && theirMinor > minor)) {
    return `written in version ${version}, newer than these types, which read ${VERSION}: take the types from a newer hpr`;
  }
  return `written in version ${version}; these types read ${VERSION} only (\`hpr convert\` rewrites an older document at ${VERSION})`;
}

/** A JSON Schema node, as far as the reader uses one. */
interface SchemaNode {
  $defs?: { [name: string]: SchemaNode };
  $ref?: string;
  type?: string | string[];
  const?: string;
  oneOf?: SchemaNode[];
  anyOf?: SchemaNode[];
  properties?: { [name: string]: SchemaNode };
  required?: string[];
  additionalProperties?: boolean | SchemaNode;
  items?: SchemaNode;
  prefixItems?: SchemaNode[];
  minItems?: number;
  maxItems?: number;
  minimum?: number;
  pattern?: string;
  format?: string;
}

/**
 * Where in the document a check failed, and why; for a value that isn't one of the constants a
 * union allows there, the value and those constants, so the message can list them all.
 */
interface Problem {
  path: string;
  message: string;
  found?: string;
  expected?: string[];
}

/** The deepest nesting hpr reads: serde_json refuses a 128th level of arrays and objects. */
const MOST_LEVELS = 127;

/** A lone UTF-16 surrogate, which JSON can escape (`"\ud800"`) but hpr refuses. */
const LONE_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?:^|[^\uD800-\uDBFF])[\uDC00-\uDFFF]/;

/** Why hpr couldn't read `value` as JSON although `JSON.parse` did, or `null`. */
function scan(value: unknown): string | null {
  const stack: Array<[unknown, number]> = [[value, 0]];
  while (stack.length > 0) {
    const [item, level] = stack.pop() as [unknown, number];
    if (typeof item === "number" && !Number.isFinite(item)) {
      return "a number too large for a 64-bit float";
    }
    if (typeof item === "string" && LONE_SURROGATE.test(item)) {
      return `a lone UTF-16 surrogate in ${shown(item)}`;
    }
    if (typeof item === "object" && item !== null) {
      if (level + 1 > MOST_LEVELS) {
        return `nested more than ${MOST_LEVELS} levels deep`;
      }
      for (const [key, child] of Object.entries(item)) {
        if (LONE_SURROGATE.test(key)) {
          return `a lone UTF-16 surrogate in the key ${shown(key)}`;
        }
        stack.push([child, level + 1]);
      }
    }
  }
  return null;
}

/** `value` as JSON, cut to its first 40 characters. */
function shown(value: unknown): string {
  const text = JSON.stringify(value) ?? String(value);
  return text.length > 40 ? `${text.slice(0, 40)}…` : text;
}

/** Whether `object` has `key` of its own, not from its prototype, as `constructor` would. */
function has(object: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function isObject(value: unknown): value is { [key: string]: unknown } {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isType(value: unknown, type: string): boolean {
  switch (type) {
    case "null":
      return value === null;
    case "boolean":
      return typeof value === "boolean";
    case "string":
      return typeof value === "string";
    case "number":
      return typeof value === "number" && Number.isFinite(value);
    case "integer":
      return typeof value === "number" && Number.isInteger(value);
    case "array":
      return Array.isArray(value);
    case "object":
      return isObject(value);
    default:
      return false;
  }
}

/** `path` followed by the key `key`: `.name` where it can be, else `["na-me"]`. */
function member(path: string, key: string): string {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(key) ? `${path}.${key}` : `${path}[${JSON.stringify(key)}]`;
}

/**
 * The problem with `value` against the union `forms`, or `null` if a form holds (exactly one,
 * for `oneOf`): where the forms fail deepest, every constant they wanted there, or else the
 * first deepest problem, or else that the value is none of them.
 */
function union(value: unknown, forms: SchemaNode[], path: string, exactlyOne: boolean): Problem | null {
  let matched = 0;
  const problems: Problem[] = [];
  for (const form of forms) {
    const problem = check(value, form, path);
    if (problem === null) {
      matched += 1;
    } else {
      problems.push(problem);
    }
  }
  if (exactlyOne && matched > 1) {
    return { path, message: `matches ${matched} of its forms, not one` };
  }
  if (matched > 0) {
    return null;
  }
  const depth = Math.max(...problems.map((p) => p.path.length));
  const deepest = problems.filter((p) => p.path.length === depth);
  const constants = deepest.filter((p) => p.expected !== undefined);
  if (constants.length > 0 && deepest.every((p) => p.path === deepest[0].path)) {
    const choices = [...new Set(constants.flatMap((p) => p.expected ?? []))];
    const listed =
      choices.length === 1 ? choices[0] : `${choices.slice(0, -1).join(", ")} or ${choices[choices.length - 1]}`;
    const found = constants[0].found;
    return { path: deepest[0].path, message: `is ${found}, not ${listed}`, found, expected: choices };
  }
  if (depth > path.length) {
    return deepest[0];
  }
  return { path, message: `${shown(value)} is none of the ${forms.length} forms allowed here` };
}

/** The first problem with `value` against `node`, or `null` when it holds. */
function check(value: unknown, node: SchemaNode, path: string): Problem | null {
  if (node.$ref !== undefined) {
    const name = node.$ref.replace("#/$defs/", "");
    const defs = SCHEMA.$defs ?? {};
    if (!has(defs, name)) {
      return { path, message: `the schema has no ${node.$ref}` };
    }
    const problem = check(value, defs[name], path);
    if (problem !== null) {
      return problem;
    }
  }
  if (node.type !== undefined) {
    const types = typeof node.type === "string" ? [node.type] : node.type;
    if (!types.some((type) => isType(value, type))) {
      return { path, message: `is ${shown(value)}, not ${types.join(" or ")}` };
    }
  }
  if (node.const !== undefined && value !== node.const) {
    const [found, expected] = [shown(value), JSON.stringify(node.const)];
    return { path, message: `is ${found}, not ${expected}`, found, expected: [expected] };
  }
  if (node.oneOf !== undefined) {
    const problem = union(value, node.oneOf, path, true);
    if (problem !== null) {
      return problem;
    }
  }
  if (node.anyOf !== undefined) {
    const problem = union(value, node.anyOf, path, false);
    if (problem !== null) {
      return problem;
    }
  }
  if (typeof value === "number") {
    if (node.minimum !== undefined && value < node.minimum) {
      return { path, message: `is ${value}, less than ${node.minimum}` };
    }
    // A `uint` is 64 bits in hpr, but JavaScript holds whole numbers exactly only below 2^53.
    const most = node.format === "uint32" ? 4294967295 : node.format === "uint" ? Number.MAX_SAFE_INTEGER : null;
    if (most !== null && value > most) {
      return { path, message: `is ${value}, more than ${node.format} holds exactly here (${most})` };
    }
  }
  if (typeof value === "string" && node.pattern !== undefined && !new RegExp(node.pattern, "u").test(value)) {
    return { path, message: `${shown(value)} doesn't match ${node.pattern}` };
  }
  if (Array.isArray(value)) {
    if (node.minItems !== undefined && value.length < node.minItems) {
      return { path, message: `has ${value.length} items, fewer than ${node.minItems}` };
    }
    if (node.maxItems !== undefined && value.length > node.maxItems) {
      return { path, message: `has ${value.length} items, more than ${node.maxItems}` };
    }
    const prefix = node.prefixItems ?? [];
    for (let i = 0; i < value.length; i++) {
      const itemNode = i < prefix.length ? prefix[i] : node.items;
      const problem = itemNode === undefined ? null : check(value[i], itemNode, `${path}[${i}]`);
      if (problem !== null) {
        return problem;
      }
    }
  }
  if (isObject(value)) {
    for (const key of node.required ?? []) {
      if (!has(value, key)) {
        return { path, message: `has no ${JSON.stringify(key)}, which it needs` };
      }
    }
    for (const [key, item] of Object.entries(value)) {
      const itemNode = node.properties !== undefined && has(node.properties, key) ? node.properties[key] : undefined;
      if (itemNode !== undefined) {
        const problem = check(item, itemNode, member(path, key));
        if (problem !== null) {
          return problem;
        }
      } else if (node.additionalProperties === false) {
        return { path, message: `has the unknown key ${JSON.stringify(key)}` };
      } else if (typeof node.additionalProperties === "object") {
        const problem = check(item, node.additionalProperties, member(path, key));
        if (problem !== null) {
          return problem;
        }
      }
    }
  }
  return null;
}
