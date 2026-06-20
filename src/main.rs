//! `bam-net` CLI — peek under the hood of Jito's Block Assembly Marketplace.

use std::path::PathBuf;

use bam_net::{cache, BamExplorerClient, NetworkSnapshot, Result, SnapshotStore};
use clap::{Parser, Subcommand};
use owo_colors::{OwoColorize, Stream::Stdout, Style};

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

    /// Output CSV instead of formatted text (ignored when --json is set).
    #[arg(long, global = true)]
    csv: bool,

    /// Disable coloured output (also honours the NO_COLOR env var).
    #[arg(long, global = true)]
    no_color: bool,

    /// Override the API base URL.
    #[arg(long, global = true)]
    base_url: Option<String>,

    /// History log file (JSONL). Defaults to $BAM_NET_CACHE or the OS data dir.
    #[arg(long, global = true, value_name = "PATH")]
    cache: Option<PathBuf>,

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
    /// Capture the current network state into the local history log.
    Snapshot,
    /// Show the adoption time series from the history log.
    History {
        /// Show only the most recent N captures.
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show validator-to-node changes between the two latest captures.
    Churn,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", friendly_error(&e));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.no_color {
        owo_colors::set_override(false);
    }

    let client = match &cli.base_url {
        Some(url) => BamExplorerClient::with_base_url(url.clone()),
        None => BamExplorerClient::new(),
    };

    let store = SnapshotStore::new(
        cli.cache
            .clone()
            .unwrap_or_else(SnapshotStore::default_path),
    );

    match cli.command.unwrap_or(Command::Summary) {
        Command::Summary => summary(&client, cli.json)?,
        Command::Nodes => nodes(&client, cli.json, cli.csv)?,
        Command::Validators { node, top } => validators(&client, cli.json, cli.csv, node, top)?,
        Command::Stake => stake(&client, cli.json)?,
        Command::Snapshot => snapshot(&client, cli.json, &store)?,
        Command::History { limit } => history(cli.json, cli.csv, &store, limit)?,
        Command::Churn => churn(cli.json, &store)?,
    }

    Ok(())
}

/// Turn a [`BamError`] into a short, actionable message for the terminal,
/// rather than leaking reqwest's internal representation.
fn friendly_error(err: &bam_net::BamError) -> String {
    use bam_net::BamError;
    let detail = match err {
        BamError::Http(e) if e.is_timeout() => {
            "timed out reaching the BAM explorer API — try again, or raise the timeout".to_string()
        }
        BamError::Http(e) if e.is_connect() => {
            "couldn't connect to the BAM explorer API — check your network or proxy".to_string()
        }
        BamError::Http(e) => match e.status().map(|s| s.as_u16()) {
            Some(404) => {
                "the BAM explorer API returned 404 — the endpoint may have moved".to_string()
            }
            Some(code) => format!("the BAM explorer API returned HTTP {code}"),
            None => format!("BAM explorer request failed: {e}"),
        },
        other => other.to_string(),
    };
    format!("error: {detail}")
}

// ── commands ────────────────────────────────────────────────────────────────

fn summary(client: &BamExplorerClient, json: bool) -> Result<()> {
    let snap: NetworkSnapshot = client.snapshot()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snap).unwrap());
        return Ok(());
    }

    banner("BAM Network");
    row(
        "Stake",
        &format!("{} SOL", bold(&fmt_sol(snap.stake.bam_stake))),
    );
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
    println!(
        "{}",
        dim("  run `bam-net nodes` or `bam-net validators --top 10` for detail")
    );
    Ok(())
}

fn nodes(client: &BamExplorerClient, json: bool, csv: bool) -> Result<()> {
    let mut nodes = client.nodes()?;
    nodes.sort_by(|a, b| b.node_stake.total_cmp(&a.node_stake));

    if json {
        println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
        return Ok(());
    }

    if csv {
        println!("bam_node,region,connected_validators,node_stake");
        for n in &nodes {
            println!(
                "{},{},{},{}",
                n.bam_node, n.region, n.connected_validators, n.node_stake
            );
        }
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
            "  {}  {:>10}  {}  {}",
            purple(&format!("{:<26}", n.bam_node)),
            n.connected_validators,
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
    csv: bool,
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

    if csv {
        println!("validator_pubkey,bam_node_connection,stake,stake_percentage");
        for v in &vs {
            println!(
                "{},{},{},{}",
                v.validator_pubkey,
                v.bam_node_connection.as_deref().unwrap_or(""),
                v.stake,
                v.stake_percentage
            );
        }
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
            dim(&format!(
                "{:<24}",
                v.bam_node_connection.as_deref().unwrap_or("-")
            )),
            bold(&format!("{:>16}", fmt_sol(v.stake))),
            share(&format!("{:>7.4}%", v.stake_percentage)),
        );
    }
    println!();
    match node {
        Some(n) => println!(
            "  {} validators on {}",
            bold(&vs.len().to_string()),
            purple(&n)
        ),
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
        &format!(
            "{}  {}",
            bar(s.bam_stake_percentage / 100.0, 28),
            pct(s.bam_stake_percentage)
        ),
    );
    Ok(())
}

fn snapshot(client: &BamExplorerClient, json: bool, store: &SnapshotStore) -> Result<()> {
    let snap = client.snapshot()?;
    let record = store.append(&snap)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&record)?);
        return Ok(());
    }

    banner("Snapshot captured");
    row("Time", &bold(&record.ts));
    row(
        "Stake",
        &format!(
            "{} SOL  {} of network",
            bold(&fmt_sol(snap.stake.bam_stake)),
            pct(snap.stake.bam_stake_percentage),
        ),
    );
    row("Nodes", &bold(&snap.node_count().to_string()));
    row("Validators", &bold(&snap.validator_count().to_string()));
    println!();
    println!(
        "{}",
        dim(&format!("  appended to {}", store.path().display()))
    );
    Ok(())
}

fn history(json: bool, csv: bool, store: &SnapshotStore, limit: Option<usize>) -> Result<()> {
    let records = store.load()?;
    if records.is_empty() {
        println!(
            "{}",
            dim("  no snapshots yet — run `bam-net snapshot` to capture one")
        );
        return Ok(());
    }

    let mut points = cache::history(&records);
    if let Some(n) = limit {
        if points.len() > n {
            points = points.split_off(points.len() - n);
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&points)?);
        return Ok(());
    }

    if csv {
        println!("ts,bam_stake,bam_stake_percentage,node_count,validator_count,top_node_share");
        for p in &points {
            println!(
                "{},{},{},{},{},{}",
                p.ts,
                p.bam_stake,
                p.bam_stake_percentage,
                p.node_count,
                p.validator_count,
                p.top_node_share
            );
        }
        return Ok(());
    }

    banner("BAM History");
    println!(
        "  {}  {}  {}  {}  {}",
        header(&format!("{:<20}", "TIME (UTC)")),
        header(&format!("{:>8}", "STAKE %")),
        header(&format!("{:>6}", "NODES")),
        header(&format!("{:>11}", "VALIDATORS")),
        header(&format!("{:>9}", "TOP NODE")),
    );
    for p in &points {
        println!(
            "  {}  {}  {}  {}  {}",
            dim(&format!("{:<20}", p.ts)),
            share(&format!("{:>7.2}%", p.bam_stake_percentage)),
            bold(&format!("{:>6}", p.node_count)),
            bold(&format!("{:>11}", p.validator_count)),
            purple(&format!("{:>8.1}%", p.top_node_share)),
        );
    }
    println!();
    println!("  {} snapshots", bold(&points.len().to_string()));
    Ok(())
}

fn churn(json: bool, store: &SnapshotStore) -> Result<()> {
    // churn only compares the two most recent captures, so read just the tail
    // rather than parsing the whole (ever-growing) log.
    let records = store.load_tail(2)?;
    if records.len() < 2 {
        println!(
            "{}",
            dim("  need at least two snapshots — run `bam-net snapshot` again later")
        );
        return Ok(());
    }

    let from = &records[0];
    let to = &records[1];
    let result = cache::churn(from, to);

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    banner("BAM Churn");
    row("From", &dim(&result.from_ts));
    row("To", &dim(&result.to_ts));
    println!();

    if result.is_empty() {
        println!(
            "{}",
            dim("  no validator-to-node changes between the last two snapshots")
        );
        return Ok(());
    }

    if !result.moved.is_empty() {
        println!("  {}", header(&format!("MOVED ({})", result.moved.len())));
        for m in &result.moved {
            println!(
                "    {}  {} → {}",
                purple(&abbrev(&m.validator_pubkey)),
                dim(m.from.as_deref().unwrap_or("(none)")),
                bold(m.to.as_deref().unwrap_or("(none)")),
            );
        }
    }
    if !result.joined.is_empty() {
        println!("  {}", header(&format!("JOINED ({})", result.joined.len())));
        for pk in &result.joined {
            println!("    {}", purple(&abbrev(pk)));
        }
    }
    if !result.left.is_empty() {
        println!("  {}", header(&format!("LEFT ({})", result.left.len())));
        for pk in &result.left {
            println!("    {}", dim(&abbrev(pk)));
        }
    }
    Ok(())
}

/// Abbreviate a base58 pubkey for compact display: `AbCdEfGh…WxyZ`.
fn abbrev(pubkey: &str) -> String {
    if pubkey.len() <= 16 {
        pubkey.to_string()
    } else {
        format!("{}…{}", &pubkey[..8], &pubkey[pubkey.len() - 4..])
    }
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
