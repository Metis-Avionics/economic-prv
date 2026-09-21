#![allow(
    clippy::needless_raw_string_hashes,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::let_underscore_must_use
)]
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

mod cache;
use cache::CliCache;

#[derive(Parser)]
#[command(name = "prv-cli")]
#[command(about = "PRV Spec-and-Go CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Spec {
        #[command(subcommand)]
        command: SpecCommands,
    },
    Living {
        #[command(subcommand)]
        command: LivingCommands,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
    Status,
}

#[derive(Subcommand)]
enum SpecCommands {
    Start,
    Validate,
    Plan,
}

#[derive(Subcommand)]
enum LivingCommands {
    Update,
}

#[derive(Subcommand)]
enum SessionCommands {
    Handover,
}

fn main() {
    let cli = Cli::parse();
    let cache = CliCache::new();

    match &cli.command {
        Commands::Spec { command } => match command {
            SpecCommands::Start => println!("Starting spec..."),
            SpecCommands::Validate => validate_spec(&cache),
            SpecCommands::Plan => println!("Planning mode..."),
        },
        Commands::Living { command } => match command {
            LivingCommands::Update => update_living(&cache),
        },
        Commands::Session { command } => match command {
            SessionCommands::Handover => session_handover(&cache),
        },
        Commands::Status => print_status(&cache),
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::shadow_reuse
)]
fn validate_spec(cache: &CliCache) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let spec_path = Path::new("spec.toml");
    let specs_dir = Path::new("specs");

    if !spec_path.exists() {
        errors.push("spec.toml not found".to_string());
        print_validation_results(&errors, &warnings);
        return;
    }

    if !specs_dir.exists() {
        errors.push("specs/ directory not found".to_string());
        print_validation_results(&errors, &warnings);
        return;
    }

    let spec_content = match cache.get("spec:spec.toml") {
        Some(cached) => cached,
        None => match fs::read_to_string(spec_path) {
            Ok(c) => {
                cache.set("spec:spec.toml", c.clone());
                c
            }
            Err(e) => {
                errors.push(format!("Failed to read spec.toml: {e}"));
                print_validation_results(&errors, &warnings);
                return;
            }
        },
    };

    let spec_doc = match spec_content.parse::<DocumentMut>() {
        Ok(d) => d,
        Err(e) => {
            errors.push(format!("spec.toml is not valid TOML: {e}"));
            print_validation_results(&errors, &warnings);
            return;
        }
    };

    let required_top_sections = [
        "project",
        "dependencies",
        "workspace",
        "numerical_stack",
        "quality",
    ];
    for section in &required_top_sections {
        if spec_doc.get(section).is_none() {
            errors.push(format!(
                "Missing required section: [{section}] in spec.toml"
            ));
        }
    }

    let project = spec_doc.get("project");
    if let Some(project) = project {
        let required_project_keys = [
            "name",
            "version",
            "edition",
            "rust_version",
            "license",
            "description",
        ];
        for key in &required_project_keys {
            if project.get(key).is_none() {
                errors.push(format!("Missing required key: project.{key} in spec.toml"));
            }
        }
    }

    let workspace = spec_doc.get("workspace");
    if let Some(workspace) = workspace
        && workspace.get("members").is_none()
    {
        errors.push("Missing required key: workspace.members in spec.toml".to_string());
    }

    let quality = spec_doc.get("quality");
    if let Some(quality) = quality {
        let required_quality_keys = ["formatting", "linting", "forbid_unsafe", "tests_required"];
        for key in &required_quality_keys {
            if quality.get(key).is_none() {
                errors.push(format!("Missing required key: quality.{key} in spec.toml"));
            }
        }
    }

    let expected_specs = vec![
        "cache.toml",
        "cli.toml",
        "data.toml",
        "dependencies.toml",
        "ekf.toml",
        "evaluation.toml",
        "methodology.toml",
        "monte_carlo.toml",
        "policy_engine.toml",
        "quaternion_model.toml",
        "state_model.toml",
    ];

    for spec_file in &expected_specs {
        let path = specs_dir.join(spec_file);
        if !path.exists() {
            errors.push(format!("Missing spec file: specs/{spec_file}"));
            continue;
        }
        let cache_key = format!("spec:{spec_file}");
        let content = match cache.get(&cache_key) {
            Some(cached) => cached,
            None => match fs::read_to_string(&path) {
                Ok(c) => {
                    cache.set(&cache_key, c.clone());
                    c
                }
                Err(e) => {
                    errors.push(format!("Failed to read specs/{spec_file}: {e}"));
                    continue;
                }
            },
        };
        if content.parse::<DocumentMut>().is_err() {
            errors.push(format!("specs/{spec_file} is not valid TOML"));
        }
    }

    let unexpected_specs = match fs::read_dir(specs_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(".toml"))
            })
            .filter(|e| !expected_specs.contains(&e.file_name().to_str().unwrap_or_default()))
            .collect::<Vec<_>>(),
        Err(e) => {
            errors.push(format!("Failed to read specs/ directory: {e}"));
            Vec::new()
        }
    };

    for unexpected in &unexpected_specs {
        warnings.push(format!(
            "Unexpected spec file: {}",
            unexpected.file_name().to_string_lossy()
        ));
    }

    if let Some(deps) = spec_doc.get("dependencies").and_then(|d| d.as_table()) {
        for (dep_name, dep_value) in deps {
            if let Some(dep_table) = dep_value.as_table() {
                if dep_table.get("crate_name").is_none() {
                    warnings.push(format!("Dependency '{dep_name}' missing crate_name field"));
                }
                if dep_table.get("source").is_none() {
                    warnings.push(format!("Dependency '{dep_name}' missing source field"));
                }
            }
        }
    }

    if let Some(workspace_members) = workspace
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        for member in workspace_members {
            if let Some(member_str) = member.as_str() {
                let member_path = Path::new(member_str);
                if !member_path.exists() {
                    errors.push(format!(
                        "Workspace member path does not exist: {member_str}"
                    ));
                }
            }
        }
    }

    print_validation_results(&errors, &warnings);
}

fn print_validation_results(errors: &[String], warnings: &[String]) {
    if errors.is_empty() && warnings.is_empty() {
        println!("Validation passed: all checks successful");
        return;
    }

    if !errors.is_empty() {
        println!("Validation failed with {} error(s):", errors.len());
        for error in errors {
            println!("  ERROR: {error}");
        }
    }

    if !warnings.is_empty() {
        println!("Validation completed with {} warning(s):", warnings.len());
        for warning in warnings {
            println!("  WARNING: {warning}");
        }
    }
}

#[allow(clippy::too_many_lines, clippy::shadow_reuse)]
fn update_living(cache: &CliCache) {
    println!("Updating living documents...");
    let docs_dir = Path::new("docs");
    if !docs_dir.exists() {
        let _ = fs::create_dir_all(docs_dir);
    }

    let root = Path::new(".");
    let spec_path = root.join("spec.toml");
    let _specs_dir = root.join("specs");
    let living_path = root.join("living.toml");

    let project_desc = cache
        .get("cli:spec.toml")
        .unwrap_or_else(|| {
            let content = fs::read_to_string(&spec_path).unwrap_or_default();
            cache.set("cli:spec.toml", content.clone());
            content
        })
        .parse::<DocumentMut>()
        .ok()
        .and_then(|d| {
            d.get("project").and_then(|p| {
                p.get("description")
                    .and_then(|n| n.as_str().map(String::from))
            })
        })
        .unwrap_or_else(|| "PRV Spec-and-Go workspace".to_string());

    let mut crate_descriptions: Vec<String> = Vec::new();
    let crate_dir = root.join("crates");
    if let Ok(entries) = fs::read_dir(&crate_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let readme = p.join("README.md");
                let desc = fs::read_to_string(&readme)
                    .ok()
                    .and_then(|s| {
                        s.lines()
                            .next()
                            .map(|l| l.trim_start_matches('#').trim().to_string())
                    })
                    .unwrap_or_else(|| name.to_string());
                crate_descriptions.push(format!("- `crates/{name}` - {desc}"));
            }
        }
    }
    crate_descriptions.sort();

    let changelog_entries = fs::read_to_string(&living_path)
        .ok()
        .map(|raw| {
            let mut entries = Vec::new();
            for line in raw.lines() {
                let line = line.trim();
                if !line.starts_with('{') && !line.starts_with("type") {
                    continue;
                }
                let etype = line
                    .split("type = \"")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .unwrap_or("");
                let desc = line
                    .split("description = \"")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .unwrap_or("");
                let scope = line
                    .split("scope = \"")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .unwrap_or("");
                if !etype.is_empty() || !desc.is_empty() {
                    entries.push(format!("- {etype}: {desc} (`{scope}`)"));
                }
            }
            entries
        })
        .unwrap_or_default();

    let living_md = format!(
        "# Living Documentation\n\n\
         This file is auto-generated by `prv-cli living update`.\n\n\
         ## Project Overview\n\n\
         {project_desc}\n\n\
         ## Architecture\n\n\
         {}\n\n\
         ## Dependencies\n\n\
         - thesix 0.2 (data-access-and-cache-abstraction)\n\
         - themql-runtime 0.1 (message-query-and-dispatch)\n\
         - nalgebra 0.35, ndarray 0.17, rand 0.10, rand_distr 0.6, statrs 0.19\n\
         - serde 1, thiserror 2, clap 4, csv 1.3, chrono 0.4\n",
        crate_descriptions.join("\n")
    );

    let changelog_md = format!(
        "# Changelog\n\n\
         This file is auto-generated by `prv-cli living update`.\n\n\
         ## From living.toml\n\n\
         {}\n",
        if changelog_entries.is_empty() {
            "- (no entries yet)".to_string()
        } else {
            changelog_entries.join("\n")
        }
    );

    let _ = fs::write(docs_dir.join("living.md"), living_md);
    let _ = fs::write(docs_dir.join("changelog.md"), changelog_md);
    println!("Updated docs/living.md and docs/changelog.md from spec.toml + living.toml");
}

#[allow(clippy::too_many_lines, clippy::shadow_reuse)]
fn session_handover(cache: &CliCache) {
    println!("Writing handover...");
    let docs_dir = Path::new("docs");
    if !docs_dir.exists() {
        let _ = fs::create_dir_all(docs_dir);
    }

    let living_path = Path::new("living.toml");
    let raw = cache.get("cli:living.toml").unwrap_or_else(|| {
        let content = fs::read_to_string(living_path).unwrap_or_default();
        cache.set("cli:living.toml", content.clone());
        content
    });
    let (handover_md, session_md) = raw
        .parse::<DocumentMut>()
        .ok()
        .map_or_else(
            || {
                (
                    "# Handover\n\n- (living.toml unreadable)\n".to_string(),
                    "# Session\n\n- (living.toml unreadable)\n".to_string(),
                )
            },
            |d| {
                let handover = d.get("handover");
                let summary = handover.and_then(|h| h.get("summary")).and_then(|v| v.as_str()).unwrap_or("");
                let status = handover.and_then(|h| h.get("status")).and_then(|v| v.as_str()).unwrap_or("");
                let phase = handover.and_then(|h| h.get("phase")).and_then(|v| v.as_str()).unwrap_or("");
                let last_updated = handover.and_then(|h| h.get("last_updated")).and_then(|v| v.as_str()).unwrap_or("");
                let next_action = handover.and_then(|h| h.get("next_action")).and_then(|v| v.as_str()).unwrap_or("");

                let completed: Vec<String> = handover
                    .and_then(|h| h.get("completed"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|entry| entry.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let changed: Vec<String> = handover
                    .and_then(|h| h.get("changed"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|entry| entry.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let validated: Vec<String> = handover
                    .and_then(|h| h.get("validated"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|entry| entry.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let remaining: Vec<String> = handover
                    .and_then(|h| h.get("remaining"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|entry| entry.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let bullets = |xs: &[String]| {
                    if xs.is_empty() {
                        "None.".to_string()
                    } else {
                        xs.iter().map(|x| format!("- {x}")).collect::<Vec<_>>().join("\n")
                    }
                };

                let handover_md = format!(
                    "# Handover\n\n\
                     This file is auto-generated by `prv-cli session handover` from `living.toml`.\n\n\
                     ## Status\n\n\
                     - Status: {status}\n\
                     - Phase: {phase}\n\
                     - Last updated: {last_updated}\n\n\
                     ## Summary\n\n\
                     {summary}\n\n\
                     ## Completed\n\n\
                     {}\n\n\
                     ## Changed\n\n\
                     {}\n\n\
                     ## Validated\n\n\
                     {}\n\n\
                     ## Remaining\n\n\
                     {}\n\n\
                     ## Next Action\n\n\
                     {next_action}\n",
                    bullets(&completed),
                    bullets(&changed),
                    bullets(&validated),
                    bullets(&remaining),
                );

                let session_md = format!(
                    "# Session\n\n\
                     - Last updated: {last_updated}\n\
                     - Phase: {phase}\n\
                     - Status: {status}\n\n\
                     ## Summary\n\n\
                     {summary}\n\n\
                     ## Remaining\n\n\
                     {}\n",
                    bullets(&remaining)
                );

                (handover_md, session_md)
            },
        );

    let _ = fs::write(docs_dir.join("handover.md"), handover_md);
    let _ = fs::write(docs_dir.join("session.md"), session_md);
    println!("Updated docs/handover.md and docs/session.md from living.toml");
}

fn print_status(cache: &CliCache) {
    println!("PRV Spec-and-Go Workspace Status");
    println!("==================================");
    let root = Path::new(".");
    let living_path = root.join("living.toml");
    let raw = cache.get("cli:living.toml").unwrap_or_else(|| {
        let content = fs::read_to_string(&living_path).unwrap_or_default();
        cache.set("cli:living.toml", content.clone());
        content
    });
    if let Ok(doc) = raw.parse::<DocumentMut>() {
        let handover = doc.get("handover");
        if let Some(h) = handover {
            println!(
                "Phase: {}",
                h.get("phase").and_then(|v| v.as_str()).unwrap_or("?")
            );
            println!(
                "Status: {}",
                h.get("status").and_then(|v| v.as_str()).unwrap_or("?")
            );
            println!(
                "Last updated: {}",
                h.get("last_updated")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            if let Some(next) = h.get("next_action").and_then(|v| v.as_str()) {
                println!("Next action: {next}");
            }
        }
    }
    println!("Crates: core, filter, monte_carlo, policy, geometry, data, evaluation, cli");
    println!("Specs loaded from: specs/*.toml");
    println!("Living docs: docs/living.md, docs/changelog.md, docs/session.md, docs/handover.md");
}
