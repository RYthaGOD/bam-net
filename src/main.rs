//! `bam-net` CLI — peek under the hood of Jito's Block Assembly Marketplace.

use bam_net::{BamExplorerClient, NetworkSnapshot, Result};
use clap::{Parser, Subcommand};
use owo_colors::{OwoColorize, Style, Stream::Stdout};

#[derive(Parser)]
#[command(
    name = "bam-net",
    version,
    about = "⚡ Peek under the hood of Jito's Block Assembly Marketplace.",
    long_about = "bam-net — typed, queryable access to the live Jito BAM network: \
                  scheduler nodes, connected validators, and how much of Solana's \
                  stake runs BAM.",
    after_help = "Tip: add --json to pipe machine-readable output into jq."
)]
struct Cli {
    /// Output raw JSON instead of formatted text.
    #[arg(long, global = true)]
    json: bool,

    /// Disable coloured output (also honours the NO_COLOR env var).
    #[arg(long, global = true)]
    no_color: bool,

    /// Override the API base URL.
    #[arg(long, global = true)]
    base_url: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Network-wide summary (default).
    Summary,
    /// List BAM nodes.
    Nodes,
    /// List validators, optionally filtered/limited.
    Validators {
        /// Only validators connected to this BAM node.
        #[arg(long)]
        node: Option<String>,
        /// Show only the top N validators by stake.
        #[arg(long)]
        top: Option<usize>,
    },
    /// Aggregate BAM stake.
    Stake,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        owo_colors::set_override(false);
    }

    let client = match &cli.base_url {
        Some(url) => BamExplorerClient::with_base_url(url.clone()),
        None => BamExplorerClient::new(),
    };

    match cli.command.unwrap_or(Command::Summary) {
        Command::Summary => summary(&client, cli.json)?,
        Command::Nodes => nodes(&client, cli.json)?,
        Command::Validators { node, top } => validators(&client, cli.json, node, top)?,
        Command::Stake => stake(&client, cli.json)?,
    }

    Ok(())
}

// ── commands ────────────────────────────────────────────────────────────────

fn summary(client: &BamExplorerClient, json: bool) -> Result<()> {
    let snap: NetworkSnapshot = client.snapshot()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snap).unwrap());
        return Ok(());
    }

    banner("BAM Network");
    row("Stake", &format!("{} SOL", bold(&fmt_sol(snap.stake.bam_stake))));
    row(
        "",
        &format!(
            "{}  {} of network",
            bar(snap.stake.bam_stake_percentage / 100.0, 28),
            pct(snap.stake.bam_stake_percentage),
        ),
    );
    row("Nodes", &bold(&snap.node_count().to_string()));
    row("Validators", &bold(&snap.validator_count().to_string()));
    if let Some(node) = snap.busiest_node() {
        row(
            "Busiest",
            &format!(
                "{} {}",
                purple(&node.bam_node),
                dim(&format!("({} validators)", node.connected_validators)),
            ),
        );
    }
    println!();
    println!("{}", dim("  run `bam-net nodes` or `bam-net validators --top 10` for detail"));
    Ok(())
}

fn nodes(client: &BamExplorerClient, json: bool) -> Result<()> {
    let mut nodes = client.nodes()?;
    nodes.sort_by(|a, b| b.node_stake.total_cmp(&a.node_stake));

    if json {
        println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
        return Ok(());
    }

    let max = nodes.first().map(|n| n.node_stake).unwrap_or(1.0).max(1.0);

    banner("BAM Nodes");
    println!(
        "  {}  {}  {}  {}",
        header(&format!("{:<26}", "NODE")),
        header(&format!("{:>10}", "VALIDATORS")),
        header(&format!("{:>16}", "STAKE (SOL)")),
        header("SHARE"),
    );
    for n in &nodes {
        println!(
            "  {}  {}  {}  {}",
            purple(&format!("{:<26}", n.bam_node)),
            format!("{:>10}", n.connected_validators),
            bold(&format!("{:>16}", fmt_sol(n.node_stake))),
            bar(n.node_stake / max, 16),
        );
    }
    println!();
    println!("  {} nodes", bold(&nodes.len().to_string()));
    Ok(())
}

fn validators(
    client: &BamExplorerClient,
    json: bool,
    node: Option<String>,
    top: Option<usize>,
) -> Result<()> {
    let mut vs = client.validators()?;
    if let Some(ref node) = node {
        vs.retain(|v| v.bam_node_connection.as_deref() == Some(node.as_str()));
    }
    vs.sort_by(|a, b| b.stake.total_cmp(&a.stake));
    let total = vs.len();
    if let Some(top) = top {
        vs.truncate(top);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&vs).unwrap());
        return Ok(());
    }

    banner("BAM Validators");
    println!(
        "  {}  {}  {}  {}",
        header(&format!("{:<44}", "VALIDATOR")),
        header(&format!("{:<24}", "NODE")),
        header(&format!("{:>16}", "STAKE (SOL)")),
        header(&format!("{:>8}", "SHARE")),
    );
    for v in &vs {
        println!(
            "  {}  {}  {}  {}",
            v.validator_pubkey,
            dim(&format!("{:<24}", v.bam_node_connection.as_deref().unwrap_or("-"))),
            bold(&format!("{:>16}", fmt_sol(v.stake))),
            share(&format!("{:>7.4}%", v.stake_percentage)),
        );
    }
    println!();
    match node {
        Some(n) => println!("  {} validators on {}", bold(&vs.len().to_string()), purple(&n)),
        None => println!(
            "  showing {} of {} validators",
            bold(&vs.len().to_string()),
            bold(&total.to_string())
        ),
    }
    Ok(())
}

fn stake(client: &BamExplorerClient, json: bool) -> Result<()> {
    let s = client.bam_stake()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&s).unwrap());
        return Ok(());
    }
    banner("BAM Stake");
    row("Total", &format!("{} SOL", bold(&fmt_sol(s.bam_stake))));
    row(
        "Network",
        &format!("{}  {}", bar(s.bam_stake_percentage / 100.0, 28), pct(s.bam_stake_percentage)),
    );
    Ok(())
}

// ── styling helpers ──────────────────────────────────────────────────────────
//
// Palette: yellow for titles, meters, and percentages; purple (magenta) for
// table headers and node identifiers; bold for key figures; dim for labels.
// Everything routes through `styled`, which honours NO_COLOR / non-TTY / the
// --no-color flag automatically.

/// Apply an owo-colors [`Style`], or pass the text through untouched when
/// colour is unsupported or disabled.
fn styled(s: &str, style: Style) -> String {
    s.if_supports_color(Stdout, |t| t.style(style)).to_string()
}

/// A styled section header with a leading lightning bolt.
fn banner(title: &str) {
    println!();
    println!(
        "  {} {}",
        styled("⚡", Style::new().yellow()),
        styled(title, Style::new().bold().yellow()),
    );
    println!("  {}", dim(&"─".repeat(title.chars().count() + 2)));
}

/// A `label  value` row with a dim, fixed-width label.
fn row(label: &str, value: &str) {
    println!("  {} {}", dim(&format!("{:<11}", label)), value);
}

/// A horizontal meter (yellow) for a 0.0–1.0 fraction.
fn bar(fraction: f64, width: usize) -> String {
    let frac = fraction.clamp(0.0, 1.0);
    let filled = (frac * width as f64).round() as usize;
    let meter = format!(
        "{}{}",
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    );
    styled(&meter, Style::new().yellow())
}

/// A percentage, bold yellow.
fn pct(p: f64) -> String {
    styled(&format!("{:.2}%", p), Style::new().bold().yellow())
}

fn dim(s: &str) -> String {
    styled(s, Style::new().dimmed())
}
fn bold(s: &str) -> String {
    styled(s, Style::new().bold())
}
/// Purple accent — node identifiers and emphasis.
fn purple(s: &str) -> String {
    styled(s, Style::new().magenta())
}
/// Bold-yellow share/percentage cell for tables.
fn share(s: &str) -> String {
    styled(s, Style::new().bold().yellow())
}
/// Purple, bold + underlined table column header.
fn header(s: &str) -> String {
    styled(s, Style::new().bold().underline().magenta())
}

/// Format a SOL amount with thousands separators and two decimals.
fn fmt_sol(amount: f64) -> String {
    let rounded = (amount * 100.0).round() / 100.0;
    let int_part = rounded.trunc().abs() as u64;
    let frac = ((rounded.abs() - int_part as f64) * 100.0).round() as u64;

    let mut digits = int_part.to_string();
    let mut grouped = String::new();
    while digits.len() > 3 {
        let split = digits.len() - 3;
        grouped = format!(",{}{}", &digits[split..], grouped);
        digits.truncate(split);
    }
    grouped = format!("{}{}", digits, grouped);

    let sign = if rounded < 0.0 { "-" } else { "" };
    format!("{}{}.{:02}", sign, grouped, frac)
}
