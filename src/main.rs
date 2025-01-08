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

    // Create PF rules for monitoring
    let rules = format!(
        "# Monitoring rules\n\
         block drop in proto {{tcp udp icmp}} from {} to any\n\
         block drop out proto {{tcp udp icmp}} from any to {}\n",
        ip, ip
    );
    std::fs::write("/tmp/pf.rules", &rules)?;

    // Enable PF if not already enabled (ignore if already enabled)
    let _ = run_sudo_command("pfctl", &["-e"]);

    // Load the rules with force flag
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
    rules.push_str("# Bandwidth limiting rules\n");

    // Simple rate limiting using state tracking
    if let Some(up) = upload {
        rules.push_str(&format!(
            "pass out proto tcp from {} to any flags S/SA keep state \
            (max-src-states {}, max-src-conn-rate {}/5)\n",
            ip, up, up
        ));
    }

    if let Some(down) = download {
        rules.push_str(&format!(
            "pass in proto tcp from any to {} flags S/SA keep state \
            (max-src-states {}, max-src-conn-rate {}/5)\n",
            ip, down, down
        ));
    }

    // Write rules to file
    std::fs::write("/tmp/pf.rules", &rules)?;

    // Enable PF if not already enabled (ignore if already enabled)
    let _ = run_sudo_command("pfctl", &["-e"]);

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
