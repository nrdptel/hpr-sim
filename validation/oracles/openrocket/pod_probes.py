"""Small designs with pods of bodies and fins, for OpenRocket 24.12 to fly (M1.13c2).

M1.13's second *done when* asks for a pod design that matches OpenRocket within the per-case
tolerance. OpenRocket's own two pod examples do not fly in hpr for reasons outside the pods
(ADR-092), and the one design in the corpus whose pods hold anything holds only a lug on each. So
this script writes probe designs that exercise the pod model: one airframe (a conical nose, a tube
60 mm across with three fins, and an AeroTech H128W in a 29 mm mount) carrying, in turn,

- three pods of a nose and a tube, with nothing else;
- two pods of a nose, a tube and three fins each;
- four pods of a nose, a tube, a tail cone and four fins each;
- two pods of no length, each hanging two fins off the airframe (the way OpenRocket's own example
  hangs its winglets);
- two pods of a nose, a tube and three fins, each with its own H128W beside the airframe's.

Every part is the conventions probe's material (1,000 kg/m³) and every element has a fixed id, so
hpr and OpenRocket name each part the same way. The motor names OpenRocket's database curve by its
digest, so both codes fly one curve (ADR-067). The designs have no recovery device: what is
compared is the climb.

`flights.py` flies them with the other public designs, and `cargo xtask ork-flights` flies hpr on
the same configurations. Run from the repository root, then `flights.py` as its docstring says:

    refs/venv/bin/python validation/oracles/openrocket/pod_probes.py \\
        validation/fixtures/ork/pod-flights

No OpenRocket is needed to write them; the files are plain XML, which OpenRocket and hpr both read.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import conventions  # noqa: E402 - the fixed ids and the probe material

MATERIAL = conventions.MATERIAL
#: The finish every outside part states: OpenRocket's default, regular paint, 60 µm.
SURFACE = f"{MATERIAL}<finish>normal</finish>"
uid = conventions.uid

#: The motor configuration every probe flies, and the motor in each mount.
CONFIGURATION = uid(97)
#: AeroTech H128W as OpenRocket 24.12's database holds it, 177.8 N·s.
DIGEST = "501239de7374691072270406476cb243"


def fins(n, name, count, root, tip, sweep, span, angle="0.0", where="bottom", at="0.0"):
    """A trapezoidal fin set of `count` fins, 3 mm thick with square edges, the first at `angle`
    degrees, placed `at` metres from its tube's `where` end."""
    return (
        f"<trapezoidfinset><name>{name}</name><id>{uid(n)}</id>"
        f"<instancecount>{count}</instancecount><fincount>{count}</fincount>"
        '<radiusoffset method="surface">0.0</radiusoffset>'
        f'<angleoffset method="relative">{angle}</angleoffset><rotation>{angle}</rotation>'
        f'<axialoffset method="{where}">{at}</axialoffset><position type="{where}">{at}</position>'
        f"<rootchord>{root}</rootchord><tipchord>{tip}</tipchord>"
        f"<sweeplength>{sweep}</sweeplength><height>{span}</height>"
        f"<thickness>0.003</thickness><crosssection>square</crosssection>{SURFACE}"
        "</trapezoidfinset>"
    )


def mount(n, name):
    """A 29 mm motor mount 0.2 m long at the bottom of its tube, holding the H128W."""
    return (
        f"<innertube><name>{name}</name><id>{uid(n)}</id>"
        '<axialoffset method="bottom">0.0</axialoffset><position type="bottom">0.0</position>'
        f"<length>0.2</length><outerradius>0.0145</outerradius><thickness>0.0005</thickness>"
        f"{MATERIAL}<motormount><ignitionevent>automatic</ignitionevent>"
        "<ignitiondelay>0.0</ignitiondelay><overhang>0.0</overhang>"
        f'<motor configid="{CONFIGURATION}"><type>single</type>'
        "<manufacturer>AeroTech</manufacturer><designation>H128W</designation>"
        f"<digest>{DIGEST}</digest><diameter>0.029</diameter><length>0.194</length>"
        "<delay>0.0</delay></motor></motormount></innertube>"
    )


def pod_nose(n, radius, length="0.06"):
    """A conical pod nose with a 1 mm wall."""
    return (
        f"<nosecone><name>Pod nose</name><id>{uid(n)}</id><length>{length}</length>"
        f"<thickness>0.001</thickness><shape>conical</shape><aftradius>{radius}</aftradius>"
        f"{SURFACE}</nosecone>"
    )


def pod_tube(n, radius, length, children=""):
    """A pod tube with a 1 mm wall, holding `children`."""
    inside = f"<subcomponents>{children}</subcomponents>" if children else ""
    return (
        f"<bodytube><name>Pod tube</name><id>{uid(n)}</id><length>{length}</length>"
        f"<thickness>0.001</thickness><radius>{radius}</radius>{SURFACE}{inside}</bodytube>"
    )


def tail_cone(n, fore, aft):
    """A conical tail cone 40 mm long with a 1 mm wall."""
    return (
        f"<transition><name>Pod tail cone</name><id>{uid(n)}</id><length>0.04</length>"
        f"<thickness>0.001</thickness><shape>conical</shape>"
        f"<foreradius>{fore}</foreradius><aftradius>{aft}</aftradius>{SURFACE}</transition>"
    )


def phantom(n, children):
    """A pod tube of no length and no radius holding `children`, as OpenRocket's example writes
    the pod that hangs its winglets."""
    return (
        f"<bodytube><name>(phantom body)</name><id>{uid(n)}</id><length>0.0</length>"
        f"<thickness>0.0</thickness><radius>0.0</radius>{SURFACE}"
        f"<subcomponents>{children}</subcomponents></bodytube>"
    )


def pod_set(count, offset, angle, top, parts):
    """`count` pods of `parts`, `offset` metres out from the airframe's surface (the `relative`
    method), the first at `angle` degrees, their top `top` metres below the airframe tube's."""
    return (
        f"<podset><name>Pods</name><id>{uid(20)}</id>"
        f"<instancecount>{count}</instancecount>"
        f'<radiusoffset method="relative">{offset}</radiusoffset>'
        f'<angleoffset method="relative">{angle}</angleoffset>'
        f'<axialoffset method="top">{top}</axialoffset><position type="top">{top}</position>'
        f"<subcomponents>{parts}</subcomponents></podset>"
    )


def document(name, pods, ballast="0.15", motor=True):
    """The airframe with `pods` in its tube, as a whole `.ork` document: `ballast` kilograms in
    the nose, and the airframe's own motor unless `motor` is false."""
    nose = (
        f"<nosecone><name>Nose</name><id>{uid(1)}</id><length>0.2</length>"
        "<thickness>0.002</thickness><shape>conical</shape><aftradius>0.03</aftradius>"
        f"{SURFACE}<subcomponents><masscomponent><name>Ballast</name><id>{uid(5)}</id>"
        '<axialoffset method="top">0.1</axialoffset><position type="top">0.1</position>'
        "<packedlength>0.02</packedlength><packedradius>0.01</packedradius>"
        f"<mass>{ballast}</mass></masscomponent></subcomponents></nosecone>"
    )
    tube = (
        f"<bodytube><name>Tube</name><id>{uid(2)}</id><length>0.6</length>"
        f"<thickness>0.001</thickness><radius>0.03</radius>{SURFACE}<subcomponents>"
        + (mount(3, "Motor mount") if motor else "")
        + fins(4, "Fins", 3, "0.12", "0.06", "0.06", "0.08")
        + pods
        + "</subcomponents></bodytube>"
    )
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        '<openrocket version="1.10" creator="hpr-sim pod probe">'
        f"<rocket><name>{name}</name><id>{uid(98)}</id>"
        f'<motorconfiguration configid="{CONFIGURATION}" default="true">'
        '<stage number="0" active="true"/></motorconfiguration>'
        "<referencetype>maximum</referencetype><subcomponents>"
        f"<stage><name>Stage</name><id>{uid(99)}</id><subcomponents>{nose}{tube}"
        "</subcomponents></stage></subcomponents></rocket></openrocket>\n"
    )


#: Each probe's pods, and what else its airframe carries (`document`'s keywords).
PROBES = {
    "pods-none": ("", {}),
    "pods-bodies-3": (
        pod_set(3, "0.01", "0.0", "0.25", pod_nose(21, "0.012") + pod_tube(22, "0.012", "0.25")),
        {},
    ),
    "pods-fins-2": (
        pod_set(
            2,
            "0.01",
            "90.0",
            "0.25",
            pod_nose(21, "0.012")
            + pod_tube(
                22, "0.012", "0.25", fins(23, "Pod fins", 3, "0.05", "0.025", "0.02", "0.03")
            ),
        ),
        {},
    ),
    "pods-fins-tail-4": (
        pod_set(
            4,
            "0.005",
            "45.0",
            "0.25",
            pod_nose(21, "0.01")
            + pod_tube(
                22, "0.01", "0.22", fins(23, "Pod fins", 4, "0.04", "0.02", "0.015", "0.025")
            )
            + tail_cone(24, "0.01", "0.006"),
        ),
        {},
    ),
    "pods-winglets-2": (
        pod_set(
            2,
            "0.0",
            "90.0",
            "0.45",
            phantom(21, fins(22, "Winglets", 2, "0.05", "0.03", "0.02", "0.03", angle="90.0")),
        ),
        {},
    ),
    "pods-motors-2": (
        pod_set(
            2,
            "0.005",
            "90.0",
            "0.3",
            pod_nose(21, "0.016", "0.08")
            + pod_tube(
                22,
                "0.016",
                "0.3",
                mount(23, "Pod motor mount")
                + fins(24, "Pod fins", 3, "0.06", "0.03", "0.025", "0.035"),
            ),
        ),
        {"ballast": "0.35", "motor": False},
    ),
}


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: pod_probes.py DIRECTORY")
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, (pods, airframe) in PROBES.items():
        text = document(name, pods, **airframe)
        (out / f"{name}.ork").write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
