//! Follow the transmission chain on a terminal — so W28 can be seen without a
//! window (BUILDER.md §0.3).
//!
//! ```sh
//! # how a line of Gemara became halacha
//! cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
//!   forward girsa:bavli/berakhot/2a:1#1
//!
//! # and where a ruling came from
//! cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
//!   back girsa:mishnah-berurah/58:1#1
//!
//! # how two texts are connected, if they are
//! cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
//!   path girsa:bavli/berakhot/2a:1#1 girsa:shulchan-arukh/orach-chayim/58:1#1
//!
//! # and where two readings of one line were argued out later
//! cargo run --release -p girsa-app --bin girsa-chain -- corpus personal \
//!   fork girsa:bavli/berakhot/2a:1#1
//! ```
//!
//! Every command ends with the same paragraph: what the walk **did not**
//! follow. A chain is a claim about how a halacha travelled, and the number of
//! seforim that were left out of it because nobody wrote down when they were
//! written is part of the claim.

// A tool that prints a report. The library it calls does not print.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use girsa_app::naming::Names;
use girsa_app::session::Language;
use girsa_app::Shelf;
use girsa_corpus::era::Timeline;
use girsa_corpus::segment::SegmentId;
use girsa_link::chain::{self, Direction, Found, Graph, Limits, Refused, Trace};
use girsa_link::Anchor;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (flags, rest): (Vec<&String>, Vec<&String>) =
        args.iter().partition(|a| a.starts_with("--"));
    let [root, personal, command, arguments @ ..] = rest.as_slice() else {
        eprintln!(
            "usage: girsa-chain <corpus-root> <personal-root> <forward|back|path|fork> <segment-id…> \
             [--depth N] [--width N]"
        );
        return std::process::ExitCode::from(2);
    };
    let (root, personal) = (
        PathBuf::from(root.as_str()),
        PathBuf::from(personal.as_str()),
    );

    let limits = Limits {
        depth: flag(&flags, "--depth").unwrap_or(Limits::default().depth),
        width: flag(&flags, "--width").unwrap_or(Limits::default().width),
        budget: flag(&flags, "--budget").unwrap_or(Limits::default().budget),
    };

    let shelf = match Shelf::open(&root, &personal) {
        Ok(shelf) => shelf,
        Err(e) => {
            eprintln!("cannot open the shelf: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let timeline = match Timeline::of(&root) {
        Ok(timeline) => timeline,
        Err(e) => {
            eprintln!(
                "cannot read {}: {e}",
                root.join("works/index.jsonl").display()
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    if !girsa_link::inbound::built(&root) {
        eprintln!(
            "warning: no inbound cache under {} — half of every link is stored at its far end,\n\
             so this walk will be short. Run girsa-link-types to build it.",
            root.join("links").display()
        );
    }

    let mut printer = Printer::new(&shelf, &timeline);
    let mut graph = Graph::new(&root, &timeline, shelf.repairs());
    let started = Instant::now();

    let code = match command.as_str() {
        "forward" | "back" => {
            let direction = if command.as_str() == "forward" {
                Direction::Forward
            } else {
                Direction::Back
            };
            let Some(at) = parse(arguments.first().map(|a| a.as_str())) else {
                return std::process::ExitCode::from(2);
            };
            let trace = chain::trace(&mut graph, &at, direction, limits);
            printer.trace(&trace);
            std::process::ExitCode::SUCCESS
        }
        "path" => {
            let (Some(from), Some(to)) = (
                parse(arguments.first().map(|a| a.as_str())),
                parse(arguments.get(1).map(|a| a.as_str())),
            ) else {
                return std::process::ExitCode::from(2);
            };
            printer.path(&chain::path(&mut graph, &from, &to, limits), &from, &to);
            std::process::ExitCode::SUCCESS
        }
        "fork" => {
            let Some(at) = parse(arguments.first().map(|a| a.as_str())) else {
                return std::process::ExitCode::from(2);
            };
            let (forks, refused) = chain::forks(&mut graph, &at, limits);
            printer.forks(&at, &forks);
            printer.refused(&refused);
            std::process::ExitCode::SUCCESS
        }
        other => {
            eprintln!("no such command: {other}");
            return std::process::ExitCode::from(2);
        }
    };

    eprintln!(
        "\n{} works read, {:.1}s",
        graph.works_read(),
        started.elapsed().as_secs_f64()
    );
    code
}

fn flag(flags: &[&String], name: &str) -> Option<usize> {
    flags
        .iter()
        .find_map(|f| f.strip_prefix(name)?.strip_prefix('=')?.parse().ok())
}

fn parse(text: Option<&str>) -> Option<SegmentId> {
    let text = text?;
    match text.parse::<SegmentId>() {
        Ok(id) => Some(id),
        Err(e) => {
            eprintln!("{text} is not a segment id: {e}");
            None
        }
    }
}

/// Draws the answers, and holds the seforim it has already opened.
///
/// A trace lands in a few dozen works and asks each of them for one line.
/// Opening a sefer is reading its whole text off disk, so each is opened once.
struct Printer<'a> {
    shelf: &'a Shelf,
    timeline: &'a Timeline,
    open: HashMap<String, Option<girsa_app::shelf::Open>>,
}

impl<'a> Printer<'a> {
    fn new(shelf: &'a Shelf, timeline: &'a Timeline) -> Self {
        Self {
            shelf,
            timeline,
            open: HashMap::new(),
        }
    }

    /// `שולחן ערוך, אורח חיים 58:1  [1565]`.
    ///
    /// This used to be twelve lines of it, and it was the best of the four
    /// composers — the only one that said `[no date]` rather than leaving the
    /// column blank, on the argument that a blank years column in a trace reads
    /// as *earlier than the row above*. That argument won, and it is now in
    /// `girsa_app::Naming::dated`, which every surface can reach.
    fn said(&self, at: &Anchor) -> String {
        Names::new(self.shelf, Some(self.timeline), Language::Hebrew)
            .of(&at.from)
            .dated()
    }

    /// The first words of a segment, so a row can be recognised.
    fn words(&mut self, at: &Anchor, take: usize) -> String {
        let slug = at.from.work().to_string();
        let open = self
            .open
            .entry(slug.clone())
            .or_insert_with(|| self.shelf.read(&slug).ok());
        let Some(open) = open.as_ref() else {
            return String::new();
        };
        let Some(nth) = open.position_of(&at.from) else {
            return String::new();
        };
        let Some(segment) = open.segments.get(nth) else {
            return String::new();
        };
        let words: Vec<&str> = segment.text.split_whitespace().take(take).collect();
        if words.is_empty() {
            String::new()
        } else if segment.text.split_whitespace().count() > take {
            format!("{} …", words.join(" "))
        } else {
            words.join(" ")
        }
    }

    fn trace(&mut self, trace: &Trace) {
        let start = Anchor::point(trace.start.clone());
        println!(
            "{} from {}",
            match trace.direction {
                Direction::Forward => "forward",
                Direction::Back => "back",
            },
            self.said(&start)
        );
        println!("  {}", self.words(&start, 12));
        println!();

        if trace.steps.is_empty() {
            println!("  nothing this walk could follow.");
        }
        // A tree, not a list of chains. The same three seforim would otherwise
        // be reprinted under every leaf below them, which is how a walk eight
        // rows wide prints as two hundred rows and reads as noise.
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); trace.steps.len() + 1];
        for (i, step) in trace.steps.iter().enumerate() {
            children[step.parent.map_or(0, |p| p + 1)].push(i);
        }
        self.branch(trace, &children, 0, 0);

        let ends = trace.ends();
        let carried: usize = ends.iter().filter(|i| trace.is_transmission(**i)).count();
        println!(
            "\n{} chains, {carried} of them a transmission all the way — the rest pass through a\n\
             link that only says the two are connected somehow, which is 49% of this graph.",
            ends.len()
        );
        self.refused(&trace.refused);
    }

    /// One node's children, and theirs.
    fn branch(&mut self, trace: &Trace, children: &[Vec<usize>], node: usize, depth: usize) {
        let Some(here) = children.get(node) else {
            return;
        };
        for i in here.clone() {
            let Some(step) = trace.steps.get(i) else {
                continue;
            };
            let pad = "  ".repeat(depth + 1);
            println!(
                "{pad}└ {}   ({}{})",
                self.said(&step.at),
                step.edge_type.as_str(),
                if step.label.is_empty() {
                    ", the corpus said nothing".to_string()
                } else if step.label == step.edge_type.as_str() {
                    String::new()
                } else {
                    format!(", the corpus said `{}`", step.label)
                }
            );
            let words = self.words(&step.at, 9);
            if !words.is_empty() {
                println!("{pad}    {words}");
            }
            self.branch(trace, children, i + 1, depth + 1);
        }
    }

    fn path(&mut self, found: &Found, from: &SegmentId, to: &SegmentId) {
        let (from, to) = (Anchor::point(from.clone()), Anchor::point(to.clone()));
        println!("from {}", self.said(&from));
        println!("to   {}", self.said(&to));
        println!();
        match found {
            Found::Path(links) if links.is_empty() => {
                println!("  they are the same place.");
            }
            Found::Path(links) => {
                println!("  {} links:", links.len());
                let unasserted = links.iter().filter(|l| !l.edge_type.is_asserted()).count();
                for (n, link) in links.iter().enumerate() {
                    println!(
                        "    {}. {}   ({})",
                        n + 1,
                        self.said(&link.at),
                        link.edge_type.as_str()
                    );
                }
                if unasserted > 0 {
                    println!(
                        "\n  {unasserted} of these links only say the two are connected somehow, \
                         so this path is\n  not evidence that anything travelled along it."
                    );
                }
            }
            Found::NotWithin { opened, depth } => {
                println!(
                    "  not found within {depth} hops of either end, after opening {opened} places.\n\
                     \n  This is not the same as there being no path. Raise --depth or --budget \
                     to look further."
                );
            }
            Found::None => {
                println!(
                    "  no path. Everything reachable from both ends was opened and they never met."
                );
            }
        }
    }

    fn forks(&mut self, at: &SegmentId, forks: &[chain::Fork]) {
        let start = Anchor::point(at.clone());
        println!("readings of {}", self.said(&start));
        println!("  {}", self.words(&start, 12));
        println!();
        if forks.is_empty() {
            println!(
                "  no fork here: no two seforim that read this line are read together by a third."
            );
            return;
        }
        println!(
            "  {} {} read this line and {} later cited together. Nothing here says they\n  \
             disagree — the corpus has no `disputes` edge anywhere in it. This is where to look.\n",
            forks.len(),
            if forks.len() == 1 { "pair" } else { "pairs" },
            if forks.len() == 1 { "is" } else { "are" }
        );
        for fork in forks {
            println!(
                "  {}\n  {}{}",
                self.said(&fork.a),
                self.said(&fork.b),
                if fork.joined {
                    "\n    — and a link joins these two directly, so one may be answering the other"
                } else {
                    ""
                }
            );
            for witness in fork.witnesses.iter().take(4) {
                println!("      both cited by {}", self.said(witness));
            }
            if fork.witnesses.len() > 4 {
                println!("      … and {} more", fork.witnesses.len() - 4);
            }
            println!();
        }
    }

    /// What was left out. Printed every time, including when it is nothing.
    fn refused(&self, refused: &Refused) {
        println!("not followed:");
        if refused.is_empty() {
            println!("  nothing — every link on the way was taken.");
            return;
        }
        if refused.wrong_way > 0 {
            println!(
                "  {:>7}  the other way in time, which is the bulk of any graph",
                refused.wrong_way
            );
        }
        if refused.contemporary > 0 {
            println!(
                "  {:>7}  written at the same time, so neither came from the other",
                refused.contemporary
            );
        }
        if refused.undated > 0 {
            println!(
                "  {:>7}  no date and no era in either corpus, so which way the hop goes is not known",
                refused.undated
            );
        }
        if refused.over_budget > 0 {
            println!(
                "  {:>7}  dropped by --width, best first",
                refused.over_budget
            );
        }
        if refused.rejected > 0 {
            println!("  {:>7}  rejected in your layer", refused.rejected);
        }
        if !refused.incoming_unknown.is_empty() {
            println!(
                "  {:>7}  works whose incoming links could not be read at all — run girsa-link-types",
                refused.incoming_unknown.len()
            );
        }
    }
}
