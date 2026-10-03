//! `hpr motors search`: motor.fusionspace.co's motors, who has them in stock and at what price
//! (ADR-131).
//!
//! The list comes from the network through `hpr_net`'s cache, from the cache alone
//! (`--offline`), or from a list saved earlier (`--from`), and is read with
//! `hpr_net::motor_finder`'s own checks. The filters are this module's; every value printed is
//! the list's. Every list, empty or not, in text or JSON, carries the site's credit with its
//! caution, as its data licence (CC BY 4.0) and terms ask, and ThrustCurve.org's, whose figures
//! the site repeats.

use std::cmp::Ordering;

use hpr::hpr_net::motor_finder::{self, MANUFACTURERS, Motor, MotorFinderError, MotorType};
use hpr::hpr_net::{NetError, thrustcurve};

use crate::motors::class_label;
use crate::output::{FoundMotor, FoundOffer, MotorKind, MotorSearch, ReadFrom};
use crate::weather::{client, format_utc, now_s, read_file, read_from, read_from_line};
use crate::{Failure, Out};

/// The currency `--max-price` is in.
const DOLLARS: &str = "USD";

/// `hpr motors search`'s filters and where the list comes from. Each filter given must match.
#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    /// Only motors in stock at one vendor or more
    #[arg(long)]
    pub in_stock: bool,
    /// Only this impulse class, such as L
    #[arg(long)]
    pub class: Option<String>,
    /// Only this diameter, mm (within 0.5 mm)
    #[arg(long, value_name = "MM", allow_negative_numbers = true)]
    pub diameter: Option<f64>,
    /// Only this manufacturer: AeroTech, Cesaroni or Loki, or its full name, in any case
    #[arg(long)]
    pub manufacturer: Option<String>,
    /// Only motors in stock whose price for one motor, at the cheapest vendor, is at most this
    /// many U.S. dollars, such as 150 or 149.99
    #[arg(long, value_name = "DOLLARS", allow_negative_numbers = true)]
    pub max_price: Option<String>,
    /// Read a list saved earlier, motor.fusionspace.co's motors.json or in-stock.json, instead of
    /// fetching it
    #[arg(long, value_name = "FILE")]
    pub from: Option<String>,
    /// Answer from the cache only, never the network
    #[arg(long, conflicts_with = "from")]
    pub offline: bool,
}

/// The filters, checked before anything is fetched.
#[derive(Clone)]
struct Filters {
    in_stock: bool,
    /// The class label, as [`class_label`] writes it.
    class: Option<String>,
    diameter_mm: Option<f64>,
    /// The manufacturer's name as the site writes it.
    manufacturer: Option<&'static str>,
    max_price_cents: Option<u64>,
}

impl Filters {
    fn new(args: &SearchArgs) -> Result<Self, Failure> {
        let class = args.class.as_deref().map(class_label).transpose()?;
        if let Some(diameter) = args.diameter
            && !(diameter.is_finite() && diameter > 0.0)
        {
            return Err(Failure::Input(format!(
                "--diameter must be a positive number of millimetres, not {diameter}"
            )));
        }
        let manufacturer = args.manufacturer.as_deref().map(maker).transpose()?;
        let max_price_cents = args.max_price.as_deref().map(cents).transpose()?;
        Ok(Self {
            in_stock: args.in_stock,
            class,
            diameter_mm: args.diameter,
            manufacturer,
            max_price_cents,
        })
    }

    /// Whether only motors in stock can pass: `--in-stock`, or `--max-price`, whose price is
    /// the cheapest in-stock offer's.
    fn only_in_stock(&self) -> bool {
        self.in_stock || self.max_price_cents.is_some()
    }

    fn keep(&self, motor: &Motor) -> bool {
        (!self.in_stock || motor.in_stock)
            && self
                .class
                .as_ref()
                .is_none_or(|class| motor.impulse_class == *class)
            && self
                .diameter_mm
                .is_none_or(|mm| (motor.diameter_mm - mm).abs() <= 0.5)
            && self
                .manufacturer
                .is_none_or(|name| motor.manufacturer == name)
            && self
                .max_price_cents
                .is_none_or(|most| dollar_price(motor).is_some_and(|cents| cents <= most))
    }

    /// The filters given, in words: `in stock, class L, at most $150.00 each`.
    fn words(&self) -> Vec<String> {
        let mut words = Vec::new();
        if self.in_stock {
            words.push("in stock".to_owned());
        }
        if let Some(class) = &self.class {
            words.push(format!("class {class}"));
        }
        if let Some(mm) = self.diameter_mm {
            words.push(format!("{mm} mm"));
        }
        if let Some(name) = self.manufacturer {
            words.push(name.to_owned());
        }
        if let Some(cents) = self.max_price_cents {
            words.push(format!("at most ${} each", money(cents)));
        }
        words
    }
}

/// Runs `hpr motors search`.
pub(crate) fn run(args: &SearchArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let filters = Filters::new(args)?;
    let (list, read_from) = match &args.from {
        Some(path) => (
            motor_finder::parse_motors(&read_file(path)?)
                .map_err(|error| Failure::Input(format!("{path}: {error}")))?,
            ReadFrom::File { path: path.clone() },
        ),
        None => {
            // In-stock motors have a file of their own, under two-thirds the size of the whole;
            // `--max-price` keeps only motors in stock, so it needs no more.
            let client = client(args.offline, "hpr motors search")?;
            let now = now_s();
            let (list, fetched) = if filters.only_in_stock() {
                match motor_finder::fetch_in_stock(&client, now) {
                    // Offline with no copy of the in-stock list, the whole list's copy answers
                    // the same search; with neither, the refusal names the one asked for.
                    Err(not_cached @ MotorFinderError::Net(NetError::NotCached { .. })) => {
                        motor_finder::fetch_motors(&client, now).map_err(|_| not_cached)
                    }
                    other => other,
                }
            } else {
                motor_finder::fetch_motors(&client, now)
            }
            .map_err(|error| Failure::Input(format!("motor.fusionspace.co: {error}")))?;
            (list, read_from(&fetched))
        }
    };
    let mut motors: Vec<&Motor> = list
        .motors
        .iter()
        .filter(|motor| filters.keep(motor))
        .collect();
    motors.sort_by(|a, b| cheapest_first(a, b));
    let document = MotorSearch {
        attribution: vec![
            motor_finder::ATTRIBUTION.to_owned(),
            thrustcurve::ATTRIBUTION.to_owned(),
        ],
        read_from,
        generated_at: list.generated_at.clone(),
        motors: motors.into_iter().map(found).collect::<Result<_, _>>()?,
    };
    let mut lines = text_lines(&document, &filters.words());
    if document.motors.is_empty()
        && let Some(hint) = priced_out(&filters, &list.motors)
    {
        lines.push(hint);
    }
    to.emit(&document, |out| {
        lines.iter().try_for_each(|line| writeln!(out, "{line}"))
    })
}

/// Why an empty list under `--max-price` is empty, when the other filters pass motors: the
/// cheapest of them, so the example `--in-stock --class L --max-price 150` says what an L costs.
fn priced_out(filters: &Filters, motors: &[Motor]) -> Option<String> {
    let most = filters.max_price_cents?;
    // In stock, as `--max-price` keeps only motors in stock: the count is the same whichever
    // list was read.
    let others = Filters {
        in_stock: true,
        max_price_cents: None,
        ..filters.clone()
    };
    let passing: Vec<&Motor> = motors.iter().filter(|m| others.keep(m)).collect();
    let count = match passing.len() {
        0 => return None,
        1 => "the 1 motor in stock".to_owned(),
        n => format!("the {n} motors in stock"),
    };
    let cheapest = passing
        .iter()
        .filter_map(|m| dollar_price(m).map(|cents| (cents, *m)))
        .min_by(|(_, a), (_, b)| cheapest_first(a, b));
    Some(match cheapest {
        Some((cents, motor)) => format!(
            "None costs ${} or less: of {count} the other filters pass, the cheapest is {} {} at \
             ${}.",
            money(most),
            printable(&motor.manufacturer),
            printable(&motor.designation),
            money(cents)
        ),
        None => format!(
            "None costs ${} or less: of {count} the other filters pass, none has a price in U.S. \
             dollars.",
            money(most)
        ),
    })
}

/// The manufacturer `--manufacturer` names, by the site's name or its slug (`cesaroni`), in any
/// case; or a refusal listing them, as a misspelt maker would otherwise list nothing.
fn maker(name: &str) -> Result<&'static str, Failure> {
    let wanted = name.trim();
    MANUFACTURERS
        .iter()
        .find(|(full, slug)| wanted.eq_ignore_ascii_case(full) || wanted.eq_ignore_ascii_case(slug))
        .map(|&(full, _)| full)
        .ok_or_else(|| {
            Failure::Input(format!(
                "--manufacturer {name} is no maker motor.fusionspace.co lists: use AeroTech, \
                 Cesaroni or Loki, or a maker's full name"
            ))
        })
}

/// A price in U.S. dollars, `150` or `149.99`, in cents; or a refusal. The digits are read
/// exactly, so `149.99` is 14,999 cents, never a float just under it.
fn cents(text: &str) -> Result<u64, Failure> {
    let refused = || {
        Failure::Input(format!(
            "--max-price {text} is not a price in U.S. dollars: use a number such as 150 or 149.99"
        ))
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let trimmed = text.trim();
    let (whole, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    let fraction_ok =
        trimmed.contains('.') && digits(fraction) && fraction.len() <= 2 || !trimmed.contains('.');
    if !digits(whole) || !fraction_ok {
        return Err(refused());
    }
    let whole: u64 = whole.parse().map_err(|_| refused())?;
    let fraction: u64 = format!("{fraction:0<2}").parse().map_err(|_| refused())?;
    whole
        .checked_mul(100)
        .and_then(|c| c.checked_add(fraction))
        .ok_or_else(refused)
}

/// `text` with each control character, such as an escape that would reach the terminal from a
/// vendor's page, shown as `?`.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// Cents as dollars and cents: `26099` as `260.99`.
fn money(cents: u64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// One motor's price at the cheapest vendor with it in stock, in U.S. cents; `None` when out of
/// stock, unpriced, or priced in another currency.
fn dollar_price(motor: &Motor) -> Option<u64> {
    motor
        .cheapest_in_stock
        .as_ref()
        .filter(|offer| offer.currency == DOLLARS)
        .and_then(|offer| offer.unit_price_cents)
}

/// Cheapest first, then by maker and designation; motors with no price in dollars last.
fn cheapest_first(a: &Motor, b: &Motor) -> Ordering {
    let price = |motor: &Motor| match dollar_price(motor) {
        Some(cents) => (false, cents),
        None => (true, 0),
    };
    price(a)
        .cmp(&price(b))
        .then_with(|| a.manufacturer.cmp(&b.manufacturer))
        .then_with(|| a.designation.cmp(&b.designation))
}

/// One motor, as listed.
fn found(motor: &Motor) -> Result<FoundMotor, Failure> {
    let motor_type = match motor.motor_type {
        None => None,
        Some(MotorType::SingleUse) => Some(MotorKind::SingleUse),
        Some(MotorType::Reload) => Some(MotorKind::Reload),
        Some(MotorType::Hybrid) => Some(MotorKind::Hybrid),
        // `hpr_net`'s enum is non-exhaustive: a kind this build doesn't know is refused, not
        // given the name of another.
        Some(other) => {
            return Err(Failure::Input(format!(
                "the motor type {other:?} is new to this build of hpr"
            )));
        }
    };
    Ok(FoundMotor {
        manufacturer: motor.manufacturer.clone(),
        designation: motor.designation.clone(),
        common_name: motor.common_name.clone(),
        impulse_class: motor.impulse_class.clone(),
        motor_type,
        diameter_mm: motor.diameter_mm,
        total_impulse_ns: motor.total_impulse_ns,
        average_thrust_n: motor.avg_thrust_n,
        burn_time_s: motor.burn_time_s,
        propellant: motor.propellant.clone(),
        delays: motor.delays.clone(),
        in_stock: motor.in_stock,
        in_stock_vendor_count: motor.in_stock_vendor_count,
        cheapest_in_stock: motor.cheapest_in_stock.as_ref().map(|offer| FoundOffer {
            vendor: offer.vendor.clone(),
            url: offer.url.clone(),
            unit_price_cents: offer.unit_price_cents,
            price_cents: offer.price_cents,
            pack_size: offer.pack_size,
            currency: offer.currency.clone(),
        }),
    })
}

/// The text output: how many motors and which, where the list is from, the credits, the table.
fn text_lines(document: &MotorSearch, filters: &[String]) -> Vec<String> {
    let count = match document.motors.len() {
        1 => "1 motor".to_owned(),
        n => format!("{n} motors"),
    };
    let which = if filters.is_empty() {
        String::new()
    } else {
        format!(" ({})", filters.join(", "))
    };
    let built = motor_finder::unix_s(&document.generated_at)
        .map_or_else(|| document.generated_at.clone(), format_utc);
    let mut lines = vec![
        format!("{count}{which} in motor.fusionspace.co's list built {built}."),
        read_from_line(&document.read_from),
    ];
    lines.extend(document.attribution.iter().cloned());
    if document.motors.is_empty() {
        return lines;
    }
    lines.push(String::new());
    let header = [
        "designation",
        "maker",
        "class",
        "dia mm",
        "impulse N·s",
        "avg N",
        "burn s",
        "each $",
        "pack",
        "vendor",
    ];
    let figure = |value: Option<f64>, places: usize| {
        value.map_or_else(|| "-".to_owned(), |v| format!("{v:.places$}"))
    };
    let rows: Vec<[String; 10]> = document
        .motors
        .iter()
        .map(|m| {
            let (price, pack, vendor) = match &m.cheapest_in_stock {
                Some(offer) => (
                    match (offer.unit_price_cents, offer.currency.as_str()) {
                        (None, _) => "-".to_owned(),
                        (Some(cents), DOLLARS) => money(cents),
                        (Some(cents), currency) => {
                            format!("{} {}", money(cents), printable(currency))
                        }
                    },
                    offer.pack_size.to_string(),
                    printable(&offer.vendor),
                ),
                None => ("-".to_owned(), "-".to_owned(), "out of stock".to_owned()),
            };
            [
                printable(&m.designation),
                // The maker's first word, as `hpr motors list` names them: `Cesaroni`.
                printable(m.manufacturer.split(' ').next().unwrap_or(&m.manufacturer)),
                printable(&m.impulse_class),
                format!("{}", m.diameter_mm),
                figure(m.total_impulse_ns, 1),
                figure(m.average_thrust_n, 1),
                figure(m.burn_time_s, 2),
                price,
                pack,
                vendor,
            ]
        })
        .collect();
    let mut widths = header.map(|h| h.chars().count());
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |cells: &[&str]| -> String {
        let mut text = String::new();
        for (i, (cell, width)) in cells.iter().zip(widths).enumerate() {
            let pad = width - cell.chars().count();
            // Text columns align left, numbers right; the last column isn't padded.
            if i == cells.len() - 1 {
                text.push_str(cell);
            } else if (3..=8).contains(&i) {
                text.push_str(&" ".repeat(pad));
                text.push_str(cell);
                text.push_str("  ");
            } else {
                text.push_str(cell);
                text.push_str(&" ".repeat(pad + 2));
            }
        }
        text.trim_end().to_owned()
    };
    lines.push(line(&header));
    for row in &rows {
        lines.push(line(&row.each_ref().map(String::as_str)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_read_to_the_cent() {
        for (text, expected) in [
            ("150", 15_000),
            ("150.5", 15_050),
            ("149.99", 14_999),
            ("0.01", 1),
            (" 7.00 ", 700),
            ("0", 0),
        ] {
            assert_eq!(cents(text).ok(), Some(expected), "{text}");
        }
        assert_eq!(money(26_099), "260.99");
        assert_eq!(money(5), "0.05");
        assert_eq!(money(cents("149.99").ok().unwrap_or_default()), "149.99");
    }

    #[test]
    fn control_characters_never_reach_the_terminal() {
        assert_eq!(printable("Shop\u{1b}]0;x\u{7}\n"), "Shop?]0;x??");
        assert_eq!(
            printable("Chris' Rocket Supplies"),
            "Chris' Rocket Supplies"
        );
    }

    #[test]
    fn malformed_prices_are_refused() {
        for text in [
            "",
            "-1",
            "1.234",
            "1.",
            ".5",
            "$150",
            "1e2",
            "abc",
            "1,000",
            "1.2.3",
            "+5",
            "184467440737095516.16",
        ] {
            assert!(
                matches!(&cents(text), Err(Failure::Input(message)) if message.contains("--max-price")),
                "{text}"
            );
        }
    }

    #[test]
    fn makers_by_name_or_slug_in_any_case() {
        for (name, expected) in [
            ("AeroTech", "AeroTech"),
            ("aerotech", "AeroTech"),
            ("Cesaroni", "Cesaroni Technology"),
            ("cesaroni technology", "Cesaroni Technology"),
            ("LOKI", "Loki Research"),
        ] {
            assert_eq!(maker(name).ok(), Some(expected), "{name}");
        }
        assert!(
            matches!(maker("Estes"), Err(Failure::Input(message)) if message.contains("--manufacturer Estes"))
        );
    }
}
