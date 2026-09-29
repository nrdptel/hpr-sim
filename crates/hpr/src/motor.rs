//! A solid rocket motor, ready to go in a rocket's motor tube.

use hpr_motor::{Catalog, Delay, SolidMotor, eng};

use crate::error::{Error, non_negative, positive};

/// A commercial solid motor: its designation, its case's size, its thrust curve and masses, and
/// the delay of its ejection charge, if it has one.
///
/// Three ways to get one:
///
/// - [`Motor::from_catalog`]: a motor from the catalog built into hpr-sim, by its designation or
///   common name, with the catalog's size, masses and thrust curve.
/// - [`Motor::from_eng`]: a RASP `.eng` file's text, as ThrustCurve.org serves it.
/// - [`Motor::new`]: a [`SolidMotor`] you built with [`hpr_motor`], and its case's size.
///
/// ```
/// use hpr::Motor;
///
/// let motor = Motor::from_catalog("H54")?.with_delay_s(10.0)?;
/// assert_eq!(motor.designation(), "168H54-10A");
/// assert_eq!(motor.diameter_m(), 0.029);
/// # Ok::<(), hpr::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Motor {
    designation: String,
    diameter_m: f64,
    length_m: f64,
    motor: SolidMotor,
    delay: Option<Delay>,
}

impl Motor {
    /// A motor of case diameter `diameter_m` and length `length_m`, called `designation`, with no
    /// ejection delay.
    ///
    /// # Errors
    ///
    /// [`Error::Domain`] for a diameter or length that isn't finite and positive.
    pub fn new(
        designation: &str,
        motor: SolidMotor,
        diameter_m: f64,
        length_m: f64,
    ) -> Result<Self, Error> {
        Ok(Self {
            designation: designation.to_owned(),
            diameter_m: positive("motor diameter, m", diameter_m)?,
            length_m: positive("motor length, m", length_m)?,
            motor,
            delay: None,
        })
    }

    /// The motor in hpr-sim's built-in catalog whose designation or common name is `name`,
    /// ignoring case, spaces and hyphens (`"H54"`, `"168H54-10A"`, `"k 400"`), with its first
    /// bundled thrust curve. Only motors with a bundled curve can be found: 32 today, listed on
    /// the [motor page][motor-page]. The delay is not set, whatever the designation says: give it
    /// with [`Motor::with_delay_s`].
    ///
    /// [motor-page]: https://nrdptel.github.io/hpr-sim/physics/motor.html
    ///
    /// # Errors
    ///
    /// - [`Error::NoSuchMotor`] if no motor with a bundled curve matches.
    /// - [`Error::AmbiguousMotor`] if different designations match, as a common name can.
    /// - [`Error::Motor`] if the catalog can't be read (never, for the bundled one: its tests
    ///   read it).
    pub fn from_catalog(name: &str) -> Result<Self, Error> {
        let catalog = Catalog::bundled()?;
        let mut matches = Vec::new();
        for entry in catalog.find(name) {
            let known = matches
                .iter()
                .any(|seen: &&hpr_motor::CatalogMotor| seen.designation == entry.designation);
            if !known && entry.bundled_motor().is_ok() {
                matches.push(entry);
            }
        }
        let entry = match matches[..] {
            [] => return Err(Error::NoSuchMotor(name.to_owned())),
            [entry] => entry,
            _ => {
                return Err(Error::AmbiguousMotor {
                    name: name.to_owned(),
                    candidates: matches
                        .iter()
                        .map(|entry| entry.designation.clone())
                        .collect(),
                });
            }
        };
        Self::new(
            &entry.designation,
            entry.bundled_motor()?,
            entry.diameter_mm / 1000.0,
            entry.length_mm / 1000.0,
        )
    }

    /// The one motor in the text of a RASP `.eng` file, with the file's size, masses and thrust
    /// curve ([`SolidMotor::from_envelope`]). The file's warnings are dropped; read the file with
    /// [`hpr_motor::eng::parse`] to see them.
    ///
    /// # Errors
    ///
    /// [`Error::Motor`] if the text isn't a motor file or its numbers don't make a motor, and
    /// [`Error::MotorCount`] if it holds more than one motor or none.
    pub fn from_eng(text: &str) -> Result<Self, Error> {
        let parsed = eng::parse(text)?;
        let [entry] = &parsed.value.entries[..] else {
            return Err(Error::MotorCount(parsed.value.entries.len()));
        };
        let diameter_m = entry.diameter_mm / 1000.0;
        let length_m = entry.length_mm / 1000.0;
        let motor = SolidMotor::from_envelope(
            entry.thrust_curve()?,
            diameter_m,
            length_m,
            entry.propellant_mass_kg,
            entry.total_mass_kg,
        )?;
        Self::new(&entry.name, motor, diameter_m, length_m)
    }

    /// The same motor with an ejection charge `delay_s` seconds after burnout.
    ///
    /// # Errors
    ///
    /// [`Error::Domain`] for a delay that is negative or not finite.
    pub fn with_delay_s(mut self, delay_s: f64) -> Result<Self, Error> {
        self.delay = Some(Delay::Seconds(non_negative("motor delay, s", delay_s)?));
        Ok(self)
    }

    /// The same motor with `delay` as its ejection delay: a time, or a plugged motor
    /// ([`Delay::Plugged`]), which has no ejection charge.
    #[must_use]
    pub fn with_delay(mut self, delay: Delay) -> Self {
        self.delay = Some(delay);
        self
    }

    /// The designation, such as `168H54-10A`.
    #[must_use]
    pub fn designation(&self) -> &str {
        &self.designation
    }

    /// The case's outer diameter, m.
    #[must_use]
    pub fn diameter_m(&self) -> f64 {
        self.diameter_m
    }

    /// The case's length, m.
    #[must_use]
    pub fn length_m(&self) -> f64 {
        self.length_m
    }

    /// The ejection delay, if one is set.
    #[must_use]
    pub fn delay(&self) -> Option<Delay> {
        self.delay
    }

    /// The motor model: thrust curve, propellant and case masses.
    #[must_use]
    pub fn solid_motor(&self) -> &SolidMotor {
        &self.motor
    }
}
