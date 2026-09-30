//! The registered commands: what each does, what it reads and writes, and whether it is
//! available yet. The README's and the guide's command tables are generated from here
//! ([`command_table`]), so neither can claim a command or a format the tool doesn't have.
//!
//! The names and summaries come from clap's own command tree, the one `hpr --help` prints.
//! [`availability`] adds, for each name, what clap doesn't know: the formats a command reads and
//! writes, or the milestone it waits for. Tests hold the two to each other both ways.

use clap::{CommandFactory, ValueEnum};
use clap_complete::Shell;

use crate::Cli;
use crate::motors::MotorFile;

/// The words a planned command's summary ends with in `hpr --help`.
pub const NOT_AVAILABLE: &str = " (not available yet)";

/// Whether a command can be used, and what it reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// The command works.
    Available {
        /// The inputs it reads, such as `.eng`.
        reads: Vec<String>,
        /// The outputs it writes.
        writes: Vec<String>,
    },
    /// The command is registered, and refuses with exit status 3 until its milestone.
    Planned {
        /// The milestone that brings it, such as `M4.2b`.
        milestone: &'static str,
    },
}

/// What `name` reads and writes, or the milestone it waits for; `None` for an unknown name.
pub fn availability(name: &str) -> Option<Availability> {
    let text_or_json = || vec!["text".to_owned(), "JSON".to_owned()];
    let planned = |milestone| Some(Availability::Planned { milestone });
    match name {
        "motors" => Some(Availability::Available {
            reads: MotorFile::ALL
                .iter()
                .map(|format| format!("`{}`", format.extension()))
                .chain(["the bundled catalog".to_owned()])
                .collect(),
            writes: text_or_json(),
        }),
        "sim" => Some(Availability::Available {
            reads: vec![
                "`.ork`".to_owned(),
                "`.hpr` or `.hprz`".to_owned(),
                "a rocket's `.json`".to_owned(),
                "a motor from the bundled catalog, `.eng` or `.rse`".to_owned(),
            ],
            writes: vec![
                "text".to_owned(),
                "JSON".to_owned(),
                "a recording as `.csv`, `.json`, `.parquet`, `.geojson` or `.kml`".to_owned(),
            ],
        }),
        "validate" => Some(Availability::Available {
            reads: vec![
                "a copy of the hpr-sim repository: its cases, references and committed reports"
                    .to_owned(),
            ],
            writes: text_or_json(),
        }),
        "convert" => Some(Availability::Available {
            reads: MotorFile::ALL
                .iter()
                .map(|format| format!("`{}`", format.extension()))
                .chain([
                    "the bundled catalog".to_owned(),
                    "a design as `.ork`, `.hpr` or `.hprz`".to_owned(),
                ])
                .collect(),
            writes: vec![
                "`.eng` or `.rse`".to_owned(),
                "`.ork`, `.hpr` or `.hprz`".to_owned(),
                "text".to_owned(),
                "JSON".to_owned(),
            ],
        }),
        "analyze" => Some(Availability::Available {
            reads: vec!["a PerfectFlite `.pf2` flight log".to_owned()],
            writes: text_or_json(),
        }),
        "completions" => Some(Availability::Available {
            reads: Vec::new(),
            writes: vec![format!("a {} script", shells()), "JSON".to_owned()],
        }),
        _ => PLANNED
            .iter()
            .find(|(command, _)| *command == name)
            .and_then(|(_, milestone)| planned(*milestone)),
    }
}

/// The commands registered before their milestone, each with the milestone that brings it.
pub const PLANNED: [(&str, &str); 5] = [
    ("weather", "M5.2d"),
    ("mc", "M6.1"),
    ("optimize", "M6.2"),
    ("compare", "M7.3"),
    ("diagnose", "M7.4"),
];

/// The shells `hpr completions` writes for: `bash, elvish, fish, powershell or zsh`.
fn shells() -> String {
    let names: Vec<String> = Shell::value_variants()
        .iter()
        .map(ToString::to_string)
        .collect();
    match names.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// One registered command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInfo {
    /// Its name, as typed after `hpr`.
    pub name: String,
    /// What it does: its summary in `hpr --help`, without [`NOT_AVAILABLE`].
    pub summary: String,
    /// Its availability; `None` if [`availability`] has no entry, which a test rules out.
    pub availability: Option<Availability>,
}

/// Every command clap registers, in `hpr --help`'s order.
pub fn commands() -> Vec<CommandInfo> {
    Cli::command()
        .get_subcommands()
        .map(|command| {
            let name = command.get_name().to_owned();
            let about = command
                .get_about()
                .map(ToString::to_string)
                .unwrap_or_default();
            CommandInfo {
                summary: about
                    .strip_suffix(NOT_AVAILABLE)
                    .unwrap_or(&about)
                    .to_owned(),
                availability: availability(&name),
                name,
            }
        })
        .collect()
}

/// The anchor of a milestone's row in the guide's *Decisions and the roadmap* page: `M4.2b` is
/// `m4-2b`.
pub fn anchor(milestone: &str) -> String {
    milestone.to_lowercase().replace('.', "-")
}

/// Where the command table's links point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Links {
    /// From the README at the repository's root: the published guide's addresses.
    Readme,
    /// From a page of the guide: relative links.
    Guide,
}

/// The command table, in Markdown: every registered command, what it does, what it reads and
/// writes, and whether it is available or which milestone brings it.
pub fn command_table(links: Links) -> String {
    let base = match links {
        Links::Readme => "https://nrdptel.github.io/hpr-sim/",
        Links::Guide => "",
    };
    let page = |name: &str| match links {
        Links::Readme => name.replace(".md", ".html"),
        Links::Guide => name.to_owned(),
    };
    let mut table = String::from(
        "| command | what it does | reads | prints | status |\n|---|---|---|---|---|\n",
    );
    for command in commands() {
        let (reads, writes, status) = match &command.availability {
            Some(Availability::Available { reads, writes }) => (
                list(reads),
                list(writes),
                format!(
                    "available ([how to use it]({base}{}#hpr-{}))",
                    page("cli.md"),
                    command.name
                ),
            ),
            Some(Availability::Planned { milestone }) => (
                "-".to_owned(),
                "-".to_owned(),
                format!(
                    "not yet: [{milestone}]({base}{}#{})",
                    page("decisions-and-roadmap.md"),
                    anchor(milestone)
                ),
            ),
            None => ("-".to_owned(), "-".to_owned(), "unregistered".to_owned()),
        };
        table.push_str(&format!(
            "| `hpr {}` | {} | {reads} | {writes} | {status} |\n",
            command.name, command.summary
        ));
    }
    table
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command clap registers has an entry, available or planned, and `hpr --help` marks
    /// the planned ones and only those.
    #[test]
    fn every_registered_command_has_an_availability() {
        let commands = commands();
        assert!(!commands.is_empty());
        for command in &commands {
            let about = Cli::command()
                .find_subcommand(&command.name)
                .and_then(|c| c.get_about().map(ToString::to_string))
                .unwrap_or_default();
            match &command.availability {
                Some(Availability::Available { writes, .. }) => {
                    assert!(!about.ends_with(NOT_AVAILABLE), "{about}");
                    assert!(!writes.is_empty(), "hpr {} writes nothing", command.name);
                }
                Some(Availability::Planned { .. }) => {
                    assert!(about.ends_with(NOT_AVAILABLE), "{about}");
                }
                None => panic!("hpr {} has no availability", command.name),
            }
        }
    }

    /// Every planned command is one clap registers, so the table can't list a ghost.
    #[test]
    fn every_planned_command_is_registered() {
        let names: Vec<String> = commands().into_iter().map(|c| c.name).collect();
        for (name, milestone) in PLANNED {
            assert!(names.iter().any(|n| n == name), "{name} isn't registered");
            assert!(milestone.starts_with('M'), "{milestone}");
        }
    }

    /// The commands the roadmap's M4.2 names are all registered.
    #[test]
    fn the_roadmaps_commands_are_registered() {
        let names: Vec<String> = commands().into_iter().map(|c| c.name).collect();
        for name in [
            "sim", "validate", "convert", "motors", "mc", "optimize", "compare", "analyze",
            "diagnose",
        ] {
            assert!(names.iter().any(|n| n == name), "{name} isn't registered");
        }
    }

    /// The table names each registered command once, with its status, and lists formats only
    /// for the commands that are available.
    #[test]
    fn the_table_lists_every_command_once() {
        let table = command_table(Links::Guide);
        for command in commands() {
            let row = format!("| `hpr {}` |", command.name);
            assert_eq!(table.matches(&row).count(), 1, "{row}");
        }
        for (name, milestone) in PLANNED {
            let row = table
                .lines()
                .find(|line| line.starts_with(&format!("| `hpr {name}` |")))
                .unwrap();
            assert!(
                row.ends_with(&format!(
                    "| - | - | not yet: [{milestone}](decisions-and-roadmap.md#{}) |",
                    anchor(milestone)
                )),
                "{row}"
            );
        }
        let readme = command_table(Links::Readme);
        assert!(readme.contains("(https://nrdptel.github.io/hpr-sim/cli.html#hpr-motors)"));
        assert!(
            readme.contains("(https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m7-3)")
        );
    }

    #[test]
    fn anchors_follow_the_roadmap_page() {
        assert_eq!(anchor("M4.2b"), "m4-2b");
        assert_eq!(anchor("M7.3"), "m7-3");
    }
}
