#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

use clap::{Parser, Subcommand};

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

    match &cli.command {
        Commands::Spec { command } => match command {
            SpecCommands::Start => println!("Starting spec..."),
            SpecCommands::Validate => validate_spec(),
            SpecCommands::Plan => println!("Planning mode..."),
        },
        Commands::Living { command } => match command {
            LivingCommands::Update => update_living(),
        },
        Commands::Session { command } => match command {
            SessionCommands::Handover => session_handover(),
        },
        Commands::Status => print_status(),
    }
}

fn validate_spec() {
    println!("Validating spec...");
}

fn update_living() {
    println!("Updating living.toml...");
}

fn session_handover() {
    println!("Writing handover...");
}

fn print_status() {
    println!("Workspace status...");
}
