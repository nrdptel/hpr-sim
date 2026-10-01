//! The parts catalogue OpenRocket 24.12 ships, compiled in.

use std::sync::OnceLock;

use super::{Catalog, read};

/// The `openrocket-database` project's catalogue files (Apache-2.0; the licence is
/// `data/openrocket-database/LICENSE` beside them), unchanged, at the commit
/// `validation/refs.lock.toml` pins (`1512874a`, 2025-07-27), which OpenRocket 24.12 ships byte
/// for byte: each name with its text, in byte order of the names. `generic_materials.orc` defines
/// materials and no parts.
pub const BUNDLED_FILES: &[(&str, &str)] = &[
    (
        "BMS.ORC",
        include_str!("../../data/openrocket-database/BMS.ORC"),
    ),
    (
        "ROCKETARIUM.ORC",
        include_str!("../../data/openrocket-database/ROCKETARIUM.ORC"),
    ),
    (
        "apogee.orc",
        include_str!("../../data/openrocket-database/apogee.orc"),
    ),
    (
        "bluetube.orc",
        include_str!("../../data/openrocket-database/bluetube.orc"),
    ),
    (
        "competition_chutes.orc",
        include_str!("../../data/openrocket-database/competition_chutes.orc"),
    ),
    (
        "estes_classic.orc",
        include_str!("../../data/openrocket-database/estes_classic.orc"),
    ),
    (
        "estes_ps2.orc",
        include_str!("../../data/openrocket-database/estes_ps2.orc"),
    ),
    (
        "generic_materials.orc",
        include_str!("../../data/openrocket-database/generic_materials.orc"),
    ),
    (
        "giantleaprocketry.orc",
        include_str!("../../data/openrocket-database/giantleaprocketry.orc"),
    ),
    (
        "loc_precision.orc",
        include_str!("../../data/openrocket-database/loc_precision.orc"),
    ),
    (
        "madcow.orc",
        include_str!("../../data/openrocket-database/madcow.orc"),
    ),
    (
        "mpc.orc",
        include_str!("../../data/openrocket-database/mpc.orc"),
    ),
    (
        "publicmissiles.orc",
        include_str!("../../data/openrocket-database/publicmissiles.orc"),
    ),
    (
        "quest.orc",
        include_str!("../../data/openrocket-database/quest.orc"),
    ),
    (
        "semroc.orc",
        include_str!("../../data/openrocket-database/semroc.orc"),
    ),
    (
        "top_flight.orc",
        include_str!("../../data/openrocket-database/top_flight.orc"),
    ),
];

/// Every part of [`BUNDLED_FILES`], file by file in their order: 3,449 parts. Read once, on the
/// first call.
#[must_use]
pub fn bundled() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut catalog = Catalog::default();
        for (name, text) in BUNDLED_FILES {
            // Invariant: the bundled files are fixed when this crate is built, and the test
            // `every_bundled_file_reads` reads each of them, with no part left out. A file that
            // didn't read would add no parts rather than stop the program.
            if let Ok(read) = read(text, name) {
                catalog.extend(read.catalog);
            }
        }
        catalog
    })
}
