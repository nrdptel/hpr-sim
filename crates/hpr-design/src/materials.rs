//! Built-in materials, each with the public source its density comes from.
//!
//! Values are nominal. The basis of each is recorded:
//!
//! - **Published** values are stated by the source: a manufacturer's data sheet, a handbook
//!   table or a specification.
//! - **Derived** values are computed from published numbers. Wood densities come from the Wood
//!   Handbook's specific gravity at 12% moisture, `ρ₁₂ = 1000 G₁₂ (1 + 0.12)`, where `G₁₂` is
//!   oven-dry mass over volume at 12% moisture (Forest Products Laboratory, FPL-GTR-190, 2010,
//!   eq. 4-12, p. 4-10). The handbook's own example, white ash at `G₁₂ = 0.605`, gives 678 kg/m³.
//!   Hobby tube densities are mass over wall volume, `m / (π/4 (OD² − ID²) L)`, from the maker's
//!   published weights and sizes.
//! - **Maximum** values are a specification's upper limit on weight; real material is often
//!   lighter. Measured 9/16" tubular nylon is 12% under its limit.
//! - **Vendor** values come from a seller's listing or measurements, where no specification
//!   exists.
//!
//! Units: 1 oz/yd² = 33.9057 g/m², 1 oz/yd = 0.0310034 kg/m, and a minimum `L` ft/lb gives a
//! maximum `1.488164 / L` kg/m. See `docs/physics/mass.md`.
//!
//! **Shear moduli.** [`SHEAR_MODULI`] gives some of these materials the in-plane shear modulus a
//! fin's flutter speed needs (NACA TN 4197's `G_E`), each with its own source; the others have
//! none. Among fin materials, no source found states one for G10/FR-4, eastern white pine, PLA,
//! ABS, PETG, polycarbonate or acrylic.
//! Where a source gives a range, the lower value is kept: a lower modulus gives a lower flutter
//! speed.
//!
//! - Metals are stated by MIL-HDBK-5J, in 10³ ksi (1 psi = 6894.757 Pa).
//! - A wood's is `G_LT = (G_LT/E_L) · 1.10 E_bend`: the Wood Handbook's elastic ratio (Table
//!   5-1, p. 5-2) times its bending modulus at 12% moisture raised by 10%, as the table's
//!   footnote a says for `E_L`. `G_LT` is the smaller of the two in-plane ratios for every wood
//!   here: a fin whose grain runs along its span or chord shears in the L-T or L-R plane.
//! - An unfilled plastic's is `E / (2 (1 + ν))`, taking it as isotropic, from its data sheet's
//!   tensile modulus and Poisson's ratio; for nylon, the conditioned (moist) values.
//! - The carbon laminate's is its unidirectional ply's `G₁₂`: a 0/90 laminate's in-plane shear
//!   modulus, and less than one with ±45° plies.

use serde::Serialize;

use crate::material::{Density, Material};

/// How a built-in value relates to its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// Stated by the source.
    Published,
    /// Computed from the source's numbers, as the module docs describe.
    Derived,
    /// A specification's upper limit.
    Maximum,
    /// A seller's listing or measurement.
    Vendor,
}

/// A built-in material.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BuiltinMaterial {
    /// Stable identifier.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Density.
    pub density: Density,
    /// The source, with the table or page.
    pub source: &'static str,
    /// Where the source was read.
    pub url: &'static str,
    /// How the value relates to the source.
    pub basis: Basis,
}

impl BuiltinMaterial {
    /// The material as a design stores it.
    pub fn material(&self) -> Material {
        Material {
            name: self.name.to_owned(),
            density: self.density,
        }
    }
}

/// Wood density at 12% moisture from the Wood Handbook's specific gravity `G₁₂`.
const fn wood(g12: f64) -> Density {
    Density::Bulk {
        kg_m3: 1000.0 * g12 * 1.12,
    }
}

const fn bulk(kg_m3: f64) -> Density {
    Density::Bulk { kg_m3 }
}

const fn surface(kg_m2: f64) -> Density {
    Density::Surface { kg_m2 }
}

const fn line(kg_m: f64) -> Density {
    Density::Line { kg_m }
}

const WOOD_HANDBOOK: &str = "https://www.fpl.fs.usda.gov/documnts/fplgtr/fpl_gtr190.pdf";
const WOOD_TABLE: &str =
    "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-3a";

/// Every built-in material.
pub const BUILTIN: &[BuiltinMaterial] = &[
    // Hobby airframe tubes.
    BuiltinMaterial {
        id: "cardboard",
        name: "Cardboard (spiral kraft tube)",
        density: bulk(790.0),
        source: "LOC Precision cardboard airframe weights and sizes (3.0 x 3.1 x 34 in, 7.8 oz; 5.38 x 5.54 x 24 in, 15 oz; 5.38 x 5.54 x 34 in, 20.4 oz): 828, 788, 756 kg/m3, mean",
        url: "https://locprecision.com/products.json",
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "kraft_phenolic",
        name: "Kraft phenolic tube",
        density: bulk(950.0),
        source: "Public Missiles phenolic tubing weight table, PT-1.1 to PT-7.5 (909 to 974 kg/m3), as published in LOC Precision's product data",
        url: "https://locprecision.com/products.json",
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "blue_tube",
        name: "Blue Tube 2.0 (vulcanized fibre)",
        density: bulk(1250.0),
        source: "Always Ready Rocketry, BTRSData.zip, MATERIAL.CSV: Vulcanized Fiber, 1.25 g/cm3",
        url: "http://alwaysreadyrocketry.com/Downloads/BTRSData.zip",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "quantum_tube",
        name: "Quantum tube",
        density: bulk(1090.0),
        source: "Public Missiles Quantum Airframe Tubing weight table, QT-2.1 to QT-3.9 (1051 to 1173 kg/m3, walls approximate), mean, as published in LOC Precision's product data",
        url: "https://locprecision.com/products.json",
        basis: Basis::Derived,
    },
    // Composites.
    BuiltinMaterial {
        id: "fiberglass_g10",
        name: "Fiberglass laminate (G10/FR-4)",
        density: bulk(1800.0),
        source: "Norplex-Micarta NP130 (NEMA LI 1 grade FR-4) data sheet, 2022, p. 1: specific gravity (ASTM D792) 1.80",
        url: "https://www.norplex-micarta.com/",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "fiberglass_filament_wound",
        name: "Filament-wound E-glass/epoxy (G12)",
        density: bulk(1990.0),
        source: "Comptec Inc., filament wound technical data: density 0.072 lb/in3; consistent with MIL-HDBK-17 rule of mixtures at 59% fibre (E-glass 2.54 g/cm3)",
        url: "https://comptecinc.com/filament-wound-technical-data/",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "carbon_fiber",
        name: "Carbon fibre/epoxy laminate",
        density: bulk(1580.0),
        source: "Hexcel HexPly 8552 product data sheet (AS4 unidirectional, 57.4% fibre volume): nominal laminate density 1.58 g/cm3",
        url: "https://www.hexcel.com/",
        basis: Basis::Published,
    },
    // Metals.
    BuiltinMaterial {
        id: "aluminum_6061",
        name: "Aluminium 6061",
        density: bulk(2700.0),
        source: "Kaiser Aluminum, 6061 Sheet, Coil and Plate data sheet, rev. 05/06: 2.70 Mg/m3",
        url: "https://online.kaiseraluminum.com/depot/PublicProductInformation/Document/1015/Kaiser_Aluminum_6061_Sheet_Coil_and_Plate.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "aluminum_7075",
        name: "Aluminium 7075",
        density: bulk(2800.0),
        source: "Kaiser Aluminum, 7075 Sheet, Coil and Plate data sheet, rev. 05/06: 2.80 Mg/m3",
        url: "https://online.kaiseraluminum.com/depot/PublicProductInformation/Document/1017/Kaiser_Aluminum_7075_Sheet_Coil_and_Plate.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "steel",
        name: "Steel (plain carbon)",
        density: bulk(7850.0),
        source: "MIL-HDBK-5J (2003), Table 2.2.1.0(b), AISI 1025, p. 2-8: 0.284 lb/in3 (7847 to 7875 kg/m3 at the printed rounding)",
        url: "https://everyspec.com/MIL-HDBK/MIL-HDBK-0001-0099/download.php?spec=MIL_HDBK_5J.139.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "titanium_6al4v",
        name: "Titanium Ti-6Al-4V",
        density: bulk(4430.0),
        source: "TIMET, TIMETAL 6-4 properties, p. 1: 0.160 lb/in3 (4.43 g/cm3)",
        url: "https://www.timet.com/assets/local/documents/technicalmanuals/TIMETAL_6-4_Properties.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "brass",
        name: "Brass (C36000)",
        density: bulk(8500.0),
        source: "Copper Development Association, alloy C36000: specific gravity 8.5",
        url: "https://alloys.copper.org/alloy/C36000",
        basis: Basis::Published,
    },
    // Woods, at 12% moisture.
    BuiltinMaterial {
        id: "balsa",
        name: "Balsa",
        density: bulk(180.0),
        source: "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), p. 2-21: about 180 kg/m3 when dry, often as little as 100",
        url: WOOD_HANDBOOK,
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "basswood",
        name: "Basswood (American)",
        density: wood(0.37),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "birch",
        name: "Birch (yellow)",
        density: wood(0.62),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "spruce",
        name: "Spruce (Sitka)",
        density: wood(0.40),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "pine",
        name: "Pine (eastern white)",
        density: wood(0.35),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "maple",
        name: "Maple (sugar)",
        density: wood(0.63),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "oak",
        name: "Oak (northern red)",
        density: wood(0.63),
        source: WOOD_TABLE,
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "birch_plywood",
        name: "Birch plywood",
        density: bulk(680.0),
        source: "Riga Wood, Plywood Handbook (2022), section 3.2, p. 62: birch plywood density (EN 323, 20 C and 65% RH), 680 kg/m3 +-50",
        url: "https://www.finieris.com/wp-content/uploads/2024/04/Riga-Wood_Plywood-Handbook_2022-1.pdf",
        basis: Basis::Published,
    },
    // Plastics.
    BuiltinMaterial {
        id: "pla",
        name: "PLA",
        density: bulk(1240.0),
        source: "NatureWorks, Ingeo 4043D technical data sheet, p. 1: density 1.24 g/cc (ASTM D1505)",
        url: "https://natureworksllc.com/getContentAsset/64260fd8-bfe0-49a4-af38-3dfdc4ef7f5c/53ffd608-340f-457b-b656-6b8cc0000bd4/TechnicalDataSheet_4043D_films.pdf?language=en",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "abs",
        name: "ABS",
        density: bulk(1040.0),
        source: "SABIC, Cycolac MG47 technical data sheet, p. 1: density 1.04 g/cm3 (ISO 1183)",
        url: "https://pc-api-public.sabic.com/api/v1/services/DocumentInfoService/?id=9ac7102f-8df7-e611-819b-06b69393ae39&language=en",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "petg",
        name: "PETG",
        density: bulk(1270.0),
        source: "Eastman, Eastar 6763 technical data sheet (2021): density 1.27 g/cm3 (ISO 1183)",
        url: "https://productcatalog.eastman.com/tds/ProdDatasheet.aspx?product=71040786&pn=eastar-6763-copolyester",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "nylon",
        name: "Nylon 6/6",
        density: bulk(1140.0),
        source: "Celanese, Zytel 101L NC010 data sheet (2023), p. 3: density 1140 kg/m3 (ISO 1183)",
        url: "https://quickparts.com/wp-content/uploads/2023/04/zytel%C2%AE-101l-nc010-gb.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "polycarbonate",
        name: "Polycarbonate",
        density: bulk(1200.0),
        source: "Covestro, Makrolon 2405 ISO data sheet (2015), p. 3: density 1200 kg/m3 (ISO 1183-1)",
        url: "https://www.okw.com/en/PC/RW_Makrolon_2405_en.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "acrylic",
        name: "Acrylic (PMMA)",
        density: bulk(1190.0),
        source: "Roehm, ACRYLITE cast sheet physical properties (1235G, 2024), p. 6: specific gravity 1.19 (ASTM D792)",
        url: "https://www.acrylite.co/files/content/acrylite.co/documents/product-information/ACRYLITE-cast-Physical-Properties.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "acetal",
        name: "Acetal (POM, Delrin)",
        density: bulk(1420.0),
        source: "Delrin 100P NC010 data sheet, p. 1: density 1.42 g/cm3 (ISO 1183)",
        url: "https://quickparts.com/wp-content/uploads/2023/05/QP-Materials-Delrin-100P-NC010.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "polystyrene",
        name: "Polystyrene",
        density: bulk(1040.0),
        source: "AmSty, STYRON 685D product information (2015), p. 1: specific gravity 1.04 (ASTM D792)",
        url: "https://amsty.com/images/documents/gpps/styron685d-tech-en.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "pvc",
        name: "PVC (rigid)",
        density: bulk(1400.0),
        source: "Charlotte Pipe, PVC Schedule 40 DWV submittal, p. 5: specific gravity 1.40 (ASTM D792)",
        url: "https://www.charlottepipe.com/uploads/documents/technical/SUB-PAC-PVC-DWV.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "hdpe",
        name: "Polyethylene (HDPE)",
        density: bulk(955.0),
        source: "Chevron Phillips, Marlex HHM 5502BN technical data sheet (2020), p. 1: density 0.955 g/cm3 (ASTM D1505)",
        url: "https://www.cpchem.com/sites/default/files/2020-10/TDS%20-%20Marlex%C2%AE%20HHM%205502BN%20Polyethylene.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "epoxy",
        name: "Epoxy (cured laminating resin)",
        density: bulk(1180.0),
        source: "Gougeon Brothers, West System 105/205 technical data sheet (2014), p. 1: cured specific gravity 1.18",
        url: "https://www.westsystem.com/app/uploads/2022/09/105_205-207-Combined.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "depron",
        name: "Depron foam (3 mm)",
        density: bulk(40.0),
        source: "Depron insulation tiles technical data sheet (2013): foam density 40 kg/m3 at 3 mm (DIN EN ISO 845)",
        url: "https://www.depron-daemmplatte.de/site/assets/files/1044/pdb-depron-d_mmplatte_gb_20160301.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "paper_bulk",
        name: "Paper (office, bulk)",
        density: bulk(755.0),
        source: "HP Office paper product data sheet (2023), p. 1: 80 g/m2 (ISO 536) at 106 um (ISO 534)",
        url: "https://hp-papers.eu/wp-content/uploads/2024/01/HP-Office-Product-Data-Sheet-EN-301023.pdf",
        basis: Basis::Derived,
    },
    // Fabrics and films.
    BuiltinMaterial {
        id: "ripstop_nylon",
        name: "Ripstop nylon (1.1 oz/yd2)",
        density: surface(1.1 * 0.033_905_7),
        source: "MIL-C-7020H (1992), Table I, p. 5, types I and Ia: 1.1 oz/yd2 maximum",
        url: "http://www.arbitrarytechnology.com/rigging/milspecs/mil-c-7020_cloth-nylon-parachute-ripstop.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "ripstop_nylon_heavy",
        name: "Ripstop nylon (1.6 oz/yd2)",
        density: surface(1.6 * 0.033_905_7),
        source: "MIL-C-7020H (1992), Table I, p. 5, types III and IIIa: 1.6 oz/yd2 maximum",
        url: "http://www.arbitrarytechnology.com/rigging/milspecs/mil-c-7020_cloth-nylon-parachute-ripstop.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "silnylon",
        name: "Silicone-coated nylon",
        density: surface(0.042),
        source: "Ripstop by the Roll, 1.1 oz silnylon listing: finished weight about 42 g/m2",
        url: "https://ripstopbytheroll.com/products/1-1-oz-silnylon",
        basis: Basis::Vendor,
    },
    BuiltinMaterial {
        id: "mylar",
        name: "Mylar (polyester film, 1 mil)",
        density: surface(1390.0 * 25.4e-6),
        source: "DuPont Teijin Films, Mylar physical-thermal properties H-37232-3 (2003), Table 1: density 1.390 g/cm3 (ASTM D1505), at 25.4 um",
        url: "https://stenbacka.fi/wp-content/uploads/sites/3/2016/07/mylar_a_fysikaaliset_ominaisuudet.pdf",
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "polyethylene_film",
        name: "Polyethylene film (LDPE, 1 mil)",
        density: surface(925.0 * 25.4e-6),
        source: "Dow LDPE 352E data sheet (2011): specific gravity 0.925 (ASTM D792), at 25.4 um",
        url: "https://stavianchem.com/sites/default/files/product-specs/352E.pdf",
        basis: Basis::Derived,
    },
    BuiltinMaterial {
        id: "paper",
        name: "Paper (office, 80 g/m2)",
        density: surface(0.080),
        source: "HP Office paper product data sheet (2023), p. 1: 80 g/m2 (ISO 536)",
        url: "https://hp-papers.eu/wp-content/uploads/2024/01/HP-Office-Product-Data-Sheet-EN-301023.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "nomex_cloth",
        name: "Nomex cloth (6 oz/yd2)",
        density: surface(6.0 * 0.033_905_7),
        source: "MIL-C-83429B (1990), Table III, p. 7: 4.7 to 6.0 oz/yd2 for the aramid cloth types; 6.0 taken",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-C/download.php?spec=MIL-C-83429B.015471.PDF",
        basis: Basis::Maximum,
    },
    // Cords and lines.
    BuiltinMaterial {
        id: "nylon_cord_type_i",
        name: "Nylon cord, type I (shroud line)",
        density: line(1.488_164 / 950.0),
        source: "MIL-C-5040H (1994), Table II, p. 6, type I: at least 950 ft/lb",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-C/download.php?spec=MIL-C-5040H.031090.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "paracord_type_iii",
        name: "Nylon cord, type III (550 paracord)",
        density: line(1.488_164 / 225.0),
        source: "MIL-C-5040H (1994), Table II, p. 6, type III: at least 225 ft/lb",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-C/download.php?spec=MIL-C-5040H.031090.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "tubular_nylon_half_inch",
        name: "Tubular nylon webbing, 1/2 in",
        density: line(0.50 * 0.031_003_4),
        source: "MIL-W-5625K (1991), Table I, p. 4: 1/2 in, 0.50 oz/yd maximum",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-W/download.php?spec=MIL-W-5625K.014723.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "tubular_nylon_9_16_inch",
        name: "Tubular nylon webbing, 9/16 in",
        density: line(0.60 * 0.031_003_4),
        source: "MIL-W-5625K (1991), Table I, p. 4: 9/16 in, 0.60 oz/yd maximum",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-W/download.php?spec=MIL-W-5625K.014723.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "tubular_nylon_1_inch",
        name: "Tubular nylon webbing, 1 in",
        density: line(1.70 * 0.031_003_4),
        source: "MIL-W-5625K (1991), Table I, p. 4: 1 in, 1.70 oz/yd maximum",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-W/download.php?spec=MIL-W-5625K.014723.pdf",
        basis: Basis::Maximum,
    },
    BuiltinMaterial {
        id: "kevlar_cord_eighth_inch",
        name: "Kevlar cord, 1/8 in",
        density: line(1.43e-3 / 0.3048),
        source: "Giant Leap Rocketry, Weights and Measures (2017), p. 3: 1/8 in Kevlar, 1.43 g/ft",
        url: "https://cdn.shopify.com/s/files/1/0518/3809/1436/files/GLRWeightsMeasures_r1.pdf",
        basis: Basis::Vendor,
    },
    BuiltinMaterial {
        id: "kevlar_cord_quarter_inch",
        name: "Kevlar cord, 1/4 in",
        density: line(2.95e-3 / 0.3048),
        source: "Giant Leap Rocketry, Weights and Measures (2017), p. 3: 1/4 in Kevlar, 2.95 g/ft",
        url: "https://cdn.shopify.com/s/files/1/0518/3809/1436/files/GLRWeightsMeasures_r1.pdf",
        basis: Basis::Vendor,
    },
    BuiltinMaterial {
        id: "elastic_cord_quarter_inch",
        name: "Elastic shock cord (bungee), 1/4 in",
        density: line(2.4 * 0.453_592_37 / 30.48),
        source: "MIL-C-5651D (1985), Table III, p. 16: 1/4 in, 2.4 lb/100 ft",
        url: "https://everyspec.com/MIL-SPECS/MIL-SPECS-MIL-C/download.php?spec=MIL-C-5651D.013946.pdf",
        basis: Basis::Published,
    },
    BuiltinMaterial {
        id: "kevlar_thread",
        name: "Kevlar thread (Tex 80)",
        density: line(0.496_055 / 5000.0),
        source: "A-A-55220 (1995), Table I, p. 2: Tex 80, at least 5000 yd/lb",
        url: "https://everyspec.com/COMML_ITEM_DESC/A-A-55000_A-A-55999/download.php?spec=A-A-55220.012789.PDF",
        basis: Basis::Maximum,
    },
];

/// The built-in material with `id`.
pub fn find(id: &str) -> Option<&'static BuiltinMaterial> {
    BUILTIN.iter().find(|m| m.id == id)
}

/// A built-in material's in-plane shear modulus, for fin flutter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BuiltinShearModulus {
    /// The [`BuiltinMaterial::id`] it belongs to.
    pub id: &'static str,
    /// Shear modulus, Pa.
    pub shear_modulus_pa: f64,
    /// The source, with the table or page.
    pub source: &'static str,
    /// Where the source was read.
    pub url: &'static str,
    /// How the value relates to the source.
    pub basis: Basis,
}

/// One pound per square inch, Pa (NIST SP 811, 2008, B.9).
const PSI_PA: f64 = 6_894.757;

/// A modulus in 10⁶ psi, Pa.
const fn msi(value: f64) -> f64 {
    value * 1e6 * PSI_PA
}

/// A wood's `G_LT` from the Wood Handbook's ratio `G_LT/E_L` and bending modulus in MPa, Pa.
const fn wood_shear(g_lt_over_e_l: f64, e_bend_mpa: f64) -> f64 {
    g_lt_over_e_l * 1.10 * e_bend_mpa * 1e6
}

/// An isotropic material's `E / (2 (1 + ν))`, Pa.
const fn isotropic_shear(e_pa: f64, poisson: f64) -> f64 {
    e_pa / (2.0 * (1.0 + poisson))
}

const MIL_HDBK_5J: &str =
    "https://everyspec.com/MIL-HDBK/MIL-HDBK-0001-0099/download.php?spec=MIL_HDBK_5J.139.pdf";

/// Every built-in shear modulus.
pub const SHEAR_MODULI: &[BuiltinShearModulus] = &[
    BuiltinShearModulus {
        id: "aluminum_6061",
        shear_modulus_pa: msi(3.8),
        source: "MIL-HDBK-5J (2003), Table 3.6.2.0(b1), 6061 sheet T4 to T62, p. 3-264: \
                 G 3.8 x 10^3 ksi",
        url: MIL_HDBK_5J,
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "aluminum_7075",
        shear_modulus_pa: msi(3.9),
        source: "MIL-HDBK-5J (2003), Table 3.7.6.0(b1), 7075-T6 sheet and T651 plate, p. 3-371: \
                 G 3.9 x 10^3 ksi",
        url: MIL_HDBK_5J,
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "steel",
        shear_modulus_pa: msi(11.0),
        source: "MIL-HDBK-5J (2003), Table 2.2.1.0(b), AISI 1025, p. 2-8: G 11.0 x 10^3 ksi",
        url: MIL_HDBK_5J,
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "titanium_6al4v",
        shear_modulus_pa: msi(6.2),
        source: "MIL-HDBK-5J (2003), Table 5.4.1.0(b), Ti-6Al-4V annealed, p. 5-53: G 6.2 x 10^3 \
                 ksi; TIMET's TIMETAL 6-4 properties (p. 15) give 6.2 and 6.66 x 10^6 psi",
        url: MIL_HDBK_5J,
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "carbon_fiber",
        shear_modulus_pa: msi(0.70),
        source: "NCAMP NCP-RP-2010-008 Rev D (2011), Hexcel 8552 AS4 unitape, Table 3-3, p. 37: \
                 in-plane shear modulus G12 0.70 Msi, room temperature dry",
        url: "https://www.wichita.edu/industry_and_defense/NIAR/Research/hexcel-8552/AS4-Unitape-3.pdf",
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "balsa",
        shear_modulus_pa: wood_shear(0.037, 3_400.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(G_LT/E_L 0.037) and Table 5-5a, p. 5-18 (modulus of elasticity 3,400 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "basswood",
        shear_modulus_pa: wood_shear(0.046, 10_100.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(G_LT/E_L 0.046) and Table 5-3a, p. 5-4 (modulus of elasticity 10,100 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "birch",
        shear_modulus_pa: wood_shear(0.068, 13_900.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(G_LT/E_L 0.068) and Table 5-3a, p. 5-4 (modulus of elasticity 13,900 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "spruce",
        shear_modulus_pa: wood_shear(0.061, 10_800.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(G_LT/E_L 0.061) and Table 5-3a, p. 5-8 (modulus of elasticity 10,800 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "maple",
        shear_modulus_pa: wood_shear(0.063, 12_600.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(G_LT/E_L 0.063) and Table 5-3a, p. 5-5 (modulus of elasticity 12,600 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "oak",
        shear_modulus_pa: wood_shear(0.081, 12_500.0),
        source: concat!(
            "Forest Products Laboratory, Wood Handbook, FPL-GTR-190 (2010), Table 5-1, p. 5-2 ",
            "(red oak, G_LT/E_L 0.081) and Table 5-3a, p. 5-5 (northern red, modulus of ",
            "elasticity 12,500 MPa at 12%)"
        ),
        url: WOOD_HANDBOOK,
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "birch_plywood",
        shear_modulus_pa: 750e6,
        source: "Riga Wood, Plywood Handbook (2022), Table 4.11, p. 89: mean modulus of rigidity \
                 in panel shear (EN 789) 750 N/mm2, every thickness, both directions",
        url: "https://www.finieris.com/wp-content/uploads/2024/04/Riga-Wood_Plywood-Handbook_2022-1.pdf",
        basis: Basis::Published,
    },
    BuiltinShearModulus {
        id: "nylon",
        shear_modulus_pa: isotropic_shear(1_400e6, 0.43),
        source: "Celanese, Zytel 101L NC010 data sheet (2023): conditioned tensile modulus \
                 1400 MPa (p. 1) and Poisson's ratio 0.43 (p. 2)",
        url: "https://quickparts.com/wp-content/uploads/2023/04/zytel%C2%AE-101l-nc010-gb.pdf",
        basis: Basis::Derived,
    },
    BuiltinShearModulus {
        id: "acetal",
        shear_modulus_pa: isotropic_shear(421_000.0 * PSI_PA, 0.37),
        source: "Delrin 100P NC010 data sheet: tensile modulus 421000 psi (p. 1) and Poisson's \
                 ratio 0.37 (p. 2)",
        url: "https://quickparts.com/wp-content/uploads/2023/05/QP-Materials-Delrin-100P-NC010.pdf",
        basis: Basis::Derived,
    },
];

/// The built-in shear modulus of the material with `id`, if a source gives one.
pub fn shear_modulus(id: &str) -> Option<&'static BuiltinShearModulus> {
    SHEAR_MODULI.iter().find(|m| m.id == id)
}

/// The built-in shear modulus of `material`, a design's copy of a built-in one (its name and
/// density both match); `None` for a material that isn't built in or has no modulus.
pub fn shear_modulus_of(material: &Material) -> Option<&'static BuiltinShearModulus> {
    let builtin = BUILTIN
        .iter()
        .find(|m| m.name == material.name && m.density == material.density)?;
    shear_modulus(builtin.id)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_entry_has_a_unique_id_a_source_and_a_positive_density() {
        let mut ids = BTreeSet::new();
        for m in BUILTIN {
            assert!(ids.insert(m.id), "duplicate id {}", m.id);
            assert!(
                m.id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{}",
                m.id
            );
            assert!(!m.name.is_empty() && m.source.len() > 20, "{}", m.id);
            assert!(m.url.starts_with("http"), "{}", m.id);
            let value = match m.density {
                Density::Bulk { kg_m3 } => kg_m3,
                Density::Surface { kg_m2 } => kg_m2,
                Density::Line { kg_m } => kg_m,
            };
            assert!(value.is_finite() && value > 0.0, "{}", m.id);
        }
        assert_eq!(BUILTIN.len(), 49);
    }

    #[test]
    fn conversions_reproduce_the_sources() {
        let kg_m3 = |id: &str| find(id).unwrap().material().bulk_kg_m3("test").unwrap();
        let kg_m2 = |id: &str| find(id).unwrap().material().surface_kg_m2("test").unwrap();
        let kg_m = |id: &str| find(id).unwrap().material().line_kg_m("test").unwrap();
        // The Wood Handbook's own example: white ash, G12 = 0.605, is 678 kg/m³ (p. 4-10).
        let Density::Bulk { kg_m3: ash } = wood(0.605) else {
            unreachable!()
        };
        assert_eq!(ash.round(), 678.0);
        assert!((kg_m3("basswood") - 414.4).abs() < 1e-9);
        assert!((kg_m3("spruce") - 448.0).abs() < 1e-9);
        // 1.1 oz/yd² is 37.3 g/m²; type III paracord at 225 ft/lb is 6.61 g/m; 1/4" Kevlar at
        // 2.95 g/ft is 9.68 g/m.
        assert!((kg_m2("ripstop_nylon") - 0.037_296).abs() < 1e-6);
        assert!((kg_m("paracord_type_iii") - 0.006_614).abs() < 1e-6);
        assert!((kg_m("kevlar_cord_quarter_inch") - 0.009_678).abs() < 1e-6);
        assert!((kg_m("elastic_cord_quarter_inch") - 0.035_72).abs() < 1e-5);
        assert!((kg_m2("mylar") - 0.035_306).abs() < 1e-6);
        assert!(find("unobtainium").is_none());
    }

    #[test]
    fn every_shear_modulus_belongs_to_a_bulk_material_and_has_a_source() {
        let mut ids = BTreeSet::new();
        for m in SHEAR_MODULI {
            assert!(ids.insert(m.id), "duplicate id {}", m.id);
            let material = find(m.id).unwrap_or_else(|| panic!("no material {}", m.id));
            assert!(matches!(material.density, Density::Bulk { .. }), "{}", m.id);
            assert!(m.source.len() > 20 && m.url.starts_with("http"), "{}", m.id);
            assert!(
                m.shear_modulus_pa.is_finite() && m.shear_modulus_pa > 0.0,
                "{}",
                m.id
            );
        }
        assert_eq!(SHEAR_MODULI.len(), 14);
        for none in ["fiberglass_g10", "pine", "polycarbonate"] {
            assert!(shear_modulus(none).is_none(), "{none}");
        }
        // A design's copy of a built-in material finds its modulus by name.
        let plywood = find("birch_plywood").unwrap().material();
        assert_eq!(shear_modulus_of(&plywood).unwrap().id, "birch_plywood");
        assert!(shear_modulus_of(&Material::bulk("Birch plywood (mine)", 680.0)).is_none());
        assert!(shear_modulus_of(&Material::bulk("Birch plywood", 500.0)).is_none());
        let names: BTreeSet<_> = BUILTIN.iter().map(|m| m.name).collect();
        assert_eq!(names.len(), BUILTIN.len(), "built-in names must be unique");
        assert!(shear_modulus_of(&find("fiberglass_g10").unwrap().material()).is_none());
    }

    #[test]
    fn shear_moduli_reproduce_the_sources() {
        let gpa = |id: &str| shear_modulus(id).unwrap().shear_modulus_pa / 1e9;
        // 3.8, 3.9, 11.0 and 6.2 × 10⁶ psi.
        assert!((gpa("aluminum_6061") - 26.200_077).abs() < 1e-6);
        assert!((gpa("aluminum_7075") - 26.889_552).abs() < 1e-6);
        assert!((gpa("steel") - 75.842_327).abs() < 1e-6);
        assert!((gpa("titanium_6al4v") - 42.747_493).abs() < 1e-6);
        assert!((gpa("carbon_fiber") - 4.826_330).abs() < 1e-6);
        // 0.046 · 1.10 · 10 100 MPa and 0.037 · 1.10 · 3400 MPa.
        assert!((gpa("basswood") - 0.511_06).abs() < 1e-9);
        assert!((gpa("balsa") - 0.138_38).abs() < 1e-9);
        // 1400 / 2.86 MPa and 421 000 psi / 2.74.
        assert!((gpa("nylon") - 0.489_510).abs() < 1e-6);
        assert!((gpa("acetal") - 1.059_377).abs() < 1e-6);
        assert!((gpa("birch_plywood") - 0.75).abs() < 1e-12);
    }
}
