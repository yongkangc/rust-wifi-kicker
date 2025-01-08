use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info, warn};
use std::process::Command;
use std::process::Output;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for devices on the network
    Scan {
        /// Network interface (e.g., en0)
        #[arg(short, long, default_value = "en0")]
        interface: String,
    },
    /// Monitor a specific device
    Monitor {
        /// Target IP address
        #[arg(short, long)]
        ip: String,
    },
    /// Limit bandwidth for a device
    Limit {
        /// Target IP address
        #[arg(short, long)]
        ip: String,
        /// Upload speed limit in KB/s
        #[arg(short, long)]
        upload: Option<u32>,
        /// Download speed limit in KB/s
        #[arg(short, long)]
        download: Option<u32>,
    },
}

fn run_sudo_command(cmd: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new("sudo")
        .arg(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run sudo command: {} {:?}", cmd, args))?;

    if !output.status.success() {
        error!("Command failed: {} {:?}", cmd, args);
        error!("Error: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(output)
}

fn scan_network(interface: &str) -> Result<()> {
    // Check if interface exists
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .output()
        .context("Failed to check interface")?;

    if !ifconfig_output.status.success() {
        error!("Interface {} not found", interface);
        return Ok(());
    }

    // Get current WiFi network name
    let output = Command::new("networksetup")
        .args(["-getairportnetwork", interface])
        .output()
        .context("Failed to get current network")?;

    println!(
        "Current network: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    // Get network details
    let airport_output = run_sudo_command(
        "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport",
        &["-I"],
    )?;
    println!("\nNetwork details:");
    println!("{}", String::from_utf8_lossy(&airport_output.stdout));

    // Perform ARP scan
    let arp_output = Command::new("arp")
        .arg("-a")
        .output()
        .context("Failed to run ARP scan")?;

    println!("\nDiscovered devices:");
    println!("{}", String::from_utf8_lossy(&arp_output.stdout));

    Ok(())
}

fn setup_monitoring(ip: &str) -> Result<()> {
    // Check if running as root
    if !Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
    {
        error!("This command requires root privileges. Please run with sudo.");
        return Ok(());
    }

    // Enable packet filtering
    run_sudo_command("pfctl", &["-e"])?;

    // Create PF rules for monitoring
    let rules = format!(
        "table <monitored> {{ {} }}\n\
         block drop from <monitored> to any\n\
         block drop from any to <monitored>",
        ip
    );
    std::fs::write("/tmp/pf.rules", &rules)?;

    // Load the rules
    run_sudo_command("pfctl", &["-f", "/tmp/pf.rules"])?;

    info!("Started monitoring {}", ip);
    Ok(())
}

fn setup_bandwidth_limit(ip: &str, upload: Option<u32>, download: Option<u32>) -> Result<()> {
    // Check if running as root
    if !Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
    {
        error!("This command requires root privileges. Please run with sudo.");
        return Ok(());
    }

    let mut rules = String::new();
    rules.push_str("table <limited> { ");
    rules.push_str(ip);
    rules.push_str(" }\n");

    if let Some(up) = upload {
        rules.push_str(&format!("queue upload bandwidth {}K max {}K\n", up, up * 2));
    }

    if let Some(down) = download {
        rules.push_str(&format!(
            "queue download bandwidth {}K max {}K\n",
            down,
            down * 2
        ));
    }

    rules.push_str("block drop from <limited> to any\n");
    rules.push_str("block drop from any to <limited>\n");

    if upload.is_some() {
        rules.push_str("pass out from <limited> to any queue upload\n");
    }
    if download.is_some() {
        rules.push_str("pass in from any to <limited> queue download\n");
    }

    // Write rules to file
    std::fs::write("/tmp/pf.rules", &rules)?;

    // Enable PF if not already enabled
    run_sudo_command("pfctl", &["-e"])?;

    // Load the rules
    run_sudo_command("pfctl", &["-f", "/tmp/pf.rules"])?;

    info!("Bandwidth limits applied for {}", ip);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { interface } => {
            scan_network(interface)?;
        }
        Commands::Monitor { ip } => {
            setup_monitoring(ip)?;
        }
        Commands::Limit {
            ip,
            upload,
            download,
        } => {
            setup_bandwidth_limit(ip, *upload, *download)?;
        }
    }

    Ok(())
}
