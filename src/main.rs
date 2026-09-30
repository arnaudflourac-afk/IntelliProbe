use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use intelliprobe::output;
use intelliprobe::report::Report;
use std::fs;
use std::path::PathBuf;

/// Profil réel d'une carte SBC ou d'une station : matériel, accélérateurs IA,
/// environnements de développement et librairies disponibles.
#[derive(Parser)]
#[command(name = "intelliprobe", version)]
struct Cli {
    /// Dossier où écrire les rapports (JSON, Markdown, HTML, prompt)
    #[arg(short, long, default_value = "rapport-intelliprobe")]
    output: PathBuf,

    /// Relire un rapport JSON existant (ex. copié depuis une carte) au lieu d'analyser cette machine
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Ne pas lancer les mesures CPU / mémoire / disque
    #[arg(long)]
    no_bench: bool,

    /// Ne pas importer les frameworks Python (plus rapide, mais n'indique pas les accélérateurs vus)
    #[arg(long)]
    no_python_import: bool,

    /// Écrire uniquement le JSON sur la sortie standard
    #[arg(long)]
    json: bool,

    /// Servir le dashboard web à la fin
    #[arg(long)]
    dashboard: bool,

    /// Adresse d'écoute du dashboard (0.0.0.0 pour y accéder depuis un autre poste)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port du dashboard
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Afficher la progression détaillée et la durée de chaque sonde
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let report: Report = if let Some(path) = &cli.input {
        let data = fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
        serde_json::from_str(&data).with_context(|| format!("{} n'est pas un rapport IntelliProbe valide", path.display()))?
    } else {
        if !cli.json {
            print_banner();
        }
        // Le test disque écrit dans le dossier de sortie (donc sur le support réellement utilisé).
        let bench_dir = if cli.json { std::env::temp_dir() } else { fs::create_dir_all(&cli.output).map(|_| cli.output.clone())? };
        let quiet = cli.json;
        let verbose = cli.verbose;
        let log = move |m: &str| {
            // Le détail framework par framework n'est affiché qu'en mode verbeux.
            if !quiet && (verbose || !m.starts_with("python : ")) {
                eprintln!("  {} {}", "▸".cyan(), m);
            }
        };
        intelliprobe::collect(&intelliprobe::Options { bench: !cli.no_bench, python_import: !cli.no_python_import, bench_dir: &bench_dir, log: &log })
    };

    if cli.json {
        println!("{}", output::to_json(&report));
        return Ok(());
    }

    output::terminal::print(&report);

    fs::create_dir_all(&cli.output)?;
    let files = [
        ("rapport.json", output::to_json(&report)),
        ("rapport.md", output::markdown::render(&report)),
        ("rapport.html", output::html::render(&report)),
        ("prompt.txt", output::prompt::render(&report)),
    ];
    println!("\n{}", "── Fichiers ".bold().cyan());
    for (name, content) in &files {
        let path = cli.output.join(name);
        fs::write(&path, content).with_context(|| format!("écriture de {}", path.display()))?;
        println!("  {}", path.display());
    }
    println!("  {}", "rapport.html s'ouvre dans n'importe quel navigateur, sans serveur ni Internet.".dimmed());

    if cli.verbose {
        println!("\n{}", "── Durées ".bold().cyan());
        for (name, t) in &report.meta.probe_timings {
            println!("  {:<22} {:>6.2} s", name, t);
        }
        println!("  {:<22} {:>6.1} s", "total", report.meta.duration_s);
    }

    if cli.dashboard {
        output::html::serve(&report, &cli.host, cli.port)?;
    }
    Ok(())
}

fn print_banner() {
    println!("{}", "╔════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", format!("║  IntelliProbe {:<41}║", env!("CARGO_PKG_VERSION")).bright_cyan().bold());
    println!("{}", "║  Matériel · accélérateurs IA · dev · librairies        ║".bright_cyan());
    println!("{}", "╚════════════════════════════════════════════════════════╝".bright_cyan());
}
