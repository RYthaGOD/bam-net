//! `bam-net` CLI — query Jito BAM network data from the terminal.

use bam_net::{BamExplorerClient, NetworkSnapshot, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bam-net",
    version,
    about = "Query Jito BAM network data (nodes, validators, stake)."
)]
struct Cli {
    /// Output raw JSON instead of formatted text.
    #[arg(long, global = true)]
    json: bool,

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

fn summary(client: &BamExplorerClient, json: bool) -> Result<()> {
    let snap: NetworkSnapshot = client.snapshot()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snap).unwrap());
        return Ok(());
    }

    println!("BAM network summary");
    println!(
        "  BAM stake:   {} SOL ({:.2}% of network)",
        fmt_sol(snap.stake.bam_stake),
        snap.stake.bam_stake_percentage
    );
    println!("  BAM nodes:   {}", snap.node_count());
    println!("  Validators:  {}", snap.validator_count());
    if let Some(node) = snap.busiest_node() {
        println!(
            "  Busiest:     {} ({} validators, {} SOL)",
            node.bam_node,
            node.connected_validators,
            fmt_sol(node.node_stake)
        );
    }
    Ok(())
}

fn nodes(client: &BamExplorerClient, json: bool) -> Result<()> {
    let mut nodes = client.nodes()?;
    nodes.sort_by(|a, b| b.node_stake.total_cmp(&a.node_stake));

    if json {
        println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
        return Ok(());
    }

    println!("{:<28} {:>10} {:>18}", "NODE", "VALIDATORS", "STAKE (SOL)");
    for n in &nodes {
        println!(
            "{:<28} {:>10} {:>18}",
            n.bam_node,
            n.connected_validators,
            fmt_sol(n.node_stake)
        );
    }
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
    if let Some(top) = top {
        vs.truncate(top);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&vs).unwrap());
        return Ok(());
    }

    println!(
        "{:<46} {:<24} {:>16} {:>8}",
        "VALIDATOR", "NODE", "STAKE (SOL)", "PCT"
    );
    for v in &vs {
        println!(
            "{:<46} {:<24} {:>16} {:>7.4}%",
            v.validator_pubkey,
            v.bam_node_connection.as_deref().unwrap_or("-"),
            fmt_sol(v.stake),
            v.stake_percentage
        );
    }
    Ok(())
}

fn stake(client: &BamExplorerClient, json: bool) -> Result<()> {
    let s = client.bam_stake()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&s).unwrap());
        return Ok(());
    }
    println!(
        "BAM stake: {} SOL ({:.2}% of network)",
        fmt_sol(s.bam_stake),
        s.bam_stake_percentage
    );
    Ok(())
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
