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
        source: "Public Missiles phenolic tubing weight table, PT-1.1 to PT-7.5 (909 to 974 kg/m3)",
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
        source: "Public Missiles Quantum Airframe Tubing weight table, QT-2.1 to QT-3.9 (1051 to 1173 kg/m3, walls approximate), mean",
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
}
