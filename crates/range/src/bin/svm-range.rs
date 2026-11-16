use std::{env, process};

use svm_range::artifact::workspace_root;
use svm_range::{Mode, all_challenges, create_from_template, find_challenge};

const USAGE: &str = "\
svm-range - vulnerable Solana programs with deterministic exploits

USAGE:
  svm-range list
  svm-range run <slug> --mode vulnerable|patched
  svm-range test
  svm-range new <slug>

Run `svm-range list` to see the available challenges.";

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match dispatch(&arguments) {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(2);
        }
    }
}

fn dispatch(arguments: &[String]) -> Result<i32, svm_range::RangeError> {
    match arguments.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("list") => {
            list_command();
            Ok(0)
        }
        Some("run") => run_command(&arguments[1..]),
        Some("test") => test_command(),
        Some("new") => new_command(&arguments[1..]),
        Some(other) => {
            eprintln!("error: unknown command `{other}`\n");
            println!("{USAGE}");
            Ok(2)
        }
    }
}

fn list_command() {
    for challenge in all_challenges() {
        println!(
            "{:<20} {}: {}",
            challenge.slug, challenge.title, challenge.description
        );
    }
}

fn run_command(arguments: &[String]) -> Result<i32, svm_range::RangeError> {
    let Some(slug) = arguments.first() else {
        eprintln!("error: `run` needs a challenge slug\n");
        println!("{USAGE}");
        return Ok(2);
    };

    let mut mode = None;
    let mut index = 1;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == "--mode" {
            let Some(value) = arguments.get(index + 1) else {
                eprintln!("error: `--mode` needs a value\n");
                return Ok(2);
            };
            mode = Some(
                Mode::parse(value)
                    .ok_or_else(|| svm_range::RangeError::InvalidMode(value.to_owned()))?,
            );
            index += 2;
        } else if let Some(value) = argument.strip_prefix("--mode=") {
            mode = Some(
                Mode::parse(value)
                    .ok_or_else(|| svm_range::RangeError::InvalidMode(value.to_owned()))?,
            );
            index += 1;
        } else {
            eprintln!("error: unknown argument `{argument}`\n");
            println!("{USAGE}");
            return Ok(2);
        }
    }

    let Some(mode) = mode else {
        eprintln!("error: `run` requires --mode vulnerable|patched\n");
        println!("{USAGE}");
        return Ok(2);
    };

    let challenge = find_challenge(slug)
        .ok_or_else(|| svm_range::RangeError::UnknownChallenge(slug.clone()))?;
    let artifact = svm_range::artifact::artifact_path(slug, mode)?;

    println!("Challenge:  {} ({})", challenge.title, challenge.slug);
    println!("Mode:       {mode}");
    println!("Program id: {}", challenge.program_id());
    let root = workspace_root()?;
    let artifact_display = artifact.strip_prefix(&root).unwrap_or(artifact.as_path());
    println!("Artifact:   {}", artifact_display.display());
    println!();

    let outcome = challenge.run(mode)?;
    for change in &outcome.changes {
        println!("  {change}");
    }
    for note in &outcome.notes {
        println!("  note: {note}");
    }
    if let Some(error) = &outcome.error {
        println!("  transaction error: {error}");
    }
    println!();
    println!("{}", outcome.summary);
    println!();

    let expected = mode == Mode::Vulnerable;
    if outcome.succeeded == expected {
        if expected {
            println!("EXPLOIT SUCCEEDED (vulnerable)");
        } else {
            println!("EXPLOIT BLOCKED (patched)");
        }
        Ok(0)
    } else {
        if expected {
            println!("EXPLOIT BLOCKED (vulnerable) - expected the exploit to succeed");
        } else {
            println!("EXPLOIT SUCCEEDED (patched) - the patch did not stop the exploit");
        }
        Ok(1)
    }
}

fn test_command() -> Result<i32, svm_range::RangeError> {
    let mut failures = Vec::new();
    let mut details = Vec::new();

    println!(
        "{:<22} {:<20} {:<20} RESULT",
        "CHALLENGE", "VULNERABLE", "PATCHED"
    );

    for challenge in all_challenges() {
        let (vulnerable_cell, vulnerable_ok) = match challenge.run(Mode::Vulnerable) {
            Ok(outcome) if outcome.succeeded => ("EXPLOIT SUCCEEDED", true),
            Ok(_) => ("EXPLOIT BLOCKED", false),
            Err(error) => {
                details.push(format!("{} (vulnerable):\n{error}", challenge.slug));
                ("ERROR", false)
            }
        };
        let (patched_cell, patched_ok) = match challenge.run(Mode::Patched) {
            Ok(outcome) if outcome.succeeded => ("EXPLOIT SUCCEEDED", false),
            Ok(_) => ("EXPLOIT BLOCKED", true),
            Err(error) => {
                details.push(format!("{} (patched):\n{error}", challenge.slug));
                ("ERROR", false)
            }
        };

        let passed = vulnerable_ok && patched_ok;
        if !passed {
            failures.push(challenge.slug);
        }
        println!(
            "{:<22} {:<20} {:<20} {}",
            challenge.slug,
            vulnerable_cell,
            patched_cell,
            if passed { "PASS" } else { "FAIL" }
        );
    }

    for detail in &details {
        println!("\n{detail}");
    }

    let total = all_challenges().len();
    if failures.is_empty() {
        println!("\nAll {total} challenges behaved as expected.");
        Ok(0)
    } else {
        println!(
            "\n{} of {total} challenges failed: {}",
            failures.len(),
            failures.join(", ")
        );
        Ok(1)
    }
}

fn new_command(arguments: &[String]) -> Result<i32, svm_range::RangeError> {
    let Some(slug) = arguments.first() else {
        eprintln!("error: `new` needs a challenge slug\n");
        println!("{USAGE}");
        return Ok(2);
    };
    if arguments.len() > 1 {
        eprintln!("error: `new` takes exactly one slug\n");
        return Ok(2);
    }

    let root = workspace_root()?;
    let created = create_from_template(slug)?;
    let display_path = created.path.strip_prefix(&root).unwrap_or(&created.path);

    println!("Created {}", display_path.display());
    for file in &created.files {
        let relative = file.strip_prefix(&root).unwrap_or(file);
        println!("  {}", relative.display());
    }
    println!();
    println!("Next:");
    println!(
        "  1. Implement challenges/{slug}/vulnerable/src/lib.rs and challenges/{slug}/patched/src/lib.rs."
    );
    println!(
        "  2. Add a module under crates/range/src/challenges/ and register it in challenges/mod.rs."
    );
    println!(
        "  3. Replace the placeholder exploit in crates/range/src/challenges/{}.rs.",
        slug.replace('-', "_")
    );
    println!("  4. Run `cargo run -- run {slug} --mode vulnerable`.");
    println!("  5. Write the root cause, exploit path, and fix in challenges/{slug}/README.md.");

    Ok(0)
}
