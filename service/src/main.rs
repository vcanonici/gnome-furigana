//! `gnome-furigana`: linguistic service and diagnostics CLI.
//!
//! Exit codes: 0 success, 1 runtime failure, 2 invalid arguments (clap).

mod alignment;
mod analysis;
mod counters;
mod dbus_interface;
mod event_log;
mod glossary;
mod inline_readings;
mod jmdict_index;
mod kana;
mod morphology;
mod protocol;
mod xdg_paths;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::analysis::Analyzer;
use crate::dbus_interface::FuriganaService;
use crate::glossary::Glossary;
use crate::morphology::MorphologicalAnalyzer;

/// Furigana e significados pt-BR para a interface japonesa do GNOME.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Executa o serviço D-Bus de sessão (usado pela unidade systemd --user).
    Serve,
    /// Mostra a análise JSON de um texto, exatamente como o D-Bus devolve.
    Analyze {
        /// Texto japonês (até 512 caracteres).
        text: String,
    },
    /// Procura o significado de uma palavra na forma de dicionário.
    Lookup {
        /// Termo, por exemplo 設定.
        term: String,
        /// Leitura em hiragana para desambiguar.
        #[arg(long)]
        reading: Option<String>,
    },
    /// Indexa o `JMdict_e(.gz)` da EDRDG em `~/.local/share/gnome-furigana/jmdict.sqlite`.
    BuildJmdict {
        /// Caminho do `JMdict_e` ou `JMdict_e.gz`.
        source: PathBuf,
    },
}

fn load_analyzer() -> Result<Analyzer, String> {
    let glossary =
        Glossary::load(&xdg_paths::config_dir(), &xdg_paths::data_dir()).map_err(|error| error.to_string())?;
    let morphology = MorphologicalAnalyzer::new().map_err(|error| error.to_string())?;
    Ok(Analyzer::new(morphology, glossary))
}

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!("{}", serde_json::to_string_pretty(value).map_err(|error| error.to_string())?);
    Ok(())
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Serve => {
            let service = FuriganaService::new(load_analyzer()?, load_analyzer);
            dbus_interface::serve(service).map_err(|error| format!("serve: {error}"))
        }
        Command::Analyze { text } => print_json(&*load_analyzer()?.analyze(&text).map_err(|error| error.to_string())?),
        Command::Lookup { term, reading } => print_json(&load_analyzer()?.glossary().lookup(&term, reading.as_deref())),
        Command::BuildJmdict { source } => {
            let target_dir = xdg_paths::data_dir();
            std::fs::create_dir_all(&target_dir)
                .map_err(|error| format!("create {}: {error}", target_dir.display()))?;
            let rows = jmdict_index::build_index(&source, &target_dir.join("jmdict.sqlite"))
                .map_err(|error| format!("build-jmdict {}: {error}", source.display()))?;
            print_json(&serde_json::json!({ "rows": rows }))
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("gnome-furigana: {message}");
            ExitCode::FAILURE
        }
    }
}
