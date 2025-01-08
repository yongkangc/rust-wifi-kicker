use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info, warn};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

const PF_RULES_FILE: &str = "/tmp/pf.rules";
const PF_STATE_FILE: &str = "/tmp/pf.state";

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
        /// Enable persistent monitoring (survives reboots)
        #[arg(short, long)]
        persistent: bool,
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
        /// Enable persistent limiting (survives reboots)
        #[arg(short, long)]
        persistent: bool,
    },
    /// Remove all rules for a specific IP
    Remove {
        /// Target IP address
        #[arg(short, long)]
        ip: String,
    },
    /// Show current rules and monitored IPs
    Status,
}

fn check_root() -> Result<()> {
    if !Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
    {
        return Err(anyhow!(
            "This command requires root privileges. Please run with sudo."
        ));
    }
    Ok(())
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
        return Err(anyhow!(
            "Command failed: {}\nError: {}",
            cmd,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output)
}

fn save_state(rules: &str, persistent: bool) -> Result<()> {
    fs::write(PF_RULES_FILE, rules)?;

    if persistent {
        // Save to a permanent location for persistence
        run_sudo_command("cp", &[PF_RULES_FILE, "/etc/pf.anchors/com.wifi-kicker"])?;

        // Add anchor to main pf.conf if not already present
        let pf_conf = fs::read_to_string("/etc/pf.conf")?;
        if !pf_conf.contains("com.wifi-kicker") {
            let anchor_rule = "anchor \"com.wifi-kicker\"";
            let new_conf = format!("{}\n{}\n", pf_conf, anchor_rule);
            fs::write("/tmp/pf.conf", new_conf)?;
            run_sudo_command("cp", &["/tmp/pf.conf", "/etc/pf.conf"])?;
        }
    }

    Ok(())
}

fn scan_network(interface: &str) -> Result<()> {
    // Check if interface exists
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .output()
        .context("Failed to check interface")?;

    if !ifconfig_output.status.success() {
        return Err(anyhow!("Interface {} not found", interface));
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

    // Get network details including subnet
    let ifconfig_output = Command::new("ifconfig")
        .arg(interface)
        .output()
        .context("Failed to get interface details")?;
    let ifconfig_str = String::from_utf8_lossy(&ifconfig_output.stdout);

    // Perform active network scan using nmap
    println!("\nScanning network for active devices...");
    let nmap_output = Command::new("nmap")
        .args(["-sn", &format!("-e{}", interface), "-oG", "-"]) // -sn performs ping scan
        .output()
        .context("Failed to run nmap scan. Please ensure nmap is installed.")?;

    println!("\nDiscovered devices:");
    println!("{}", String::from_utf8_lossy(&nmap_output.stdout));

    // Still include ARP cache for recently seen devices
    let arp_output = Command::new("arp")
        .arg("-a")
        .output()
        .context("Failed to run ARP scan")?;

    println!("\nRecently active devices (ARP cache):");
    println!("{}", String::from_utf8_lossy(&arp_output.stdout));

    Ok(())
}

fn setup_monitoring(ip: &str, persistent: bool) -> Result<()> {
    check_root()?;

    // Create PF rules for monitoring
    let rules = format!(
        "# Monitoring rules for {}\n\
         block drop in proto {{tcp udp icmp}} from {} to any\n\
         block drop out proto {{tcp udp icmp}} from any to {}\n",
        ip, ip, ip
    );

    save_state(&rules, persistent)?;

    // Enable PF if not already enabled (ignore if already enabled)
    let _ = run_sudo_command("pfctl", &["-e"]);

    // Load the rules
    run_sudo_command("pfctl", &["-f", PF_RULES_FILE])?;

    info!("Started monitoring {} (persistent: {})", ip, persistent);
    Ok(())
}

fn setup_bandwidth_limit(
    ip: &str,
    upload: Option<u32>,
    download: Option<u32>,
    persistent: bool,
) -> Result<()> {
    check_root()?;

    let mut rules = String::new();
    rules.push_str(&format!("# Bandwidth limiting rules for {}\n", ip));

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

    save_state(&rules, persistent)?;

    // Enable PF if not already enabled (ignore if already enabled)
    let _ = run_sudo_command("pfctl", &["-e"]);

    // Load the rules
    run_sudo_command("pfctl", &["-f", PF_RULES_FILE])?;

    info!(
        "Bandwidth limits applied for {} (persistent: {})",
        ip, persistent
    );
    Ok(())
}

fn remove_rules(ip: &str) -> Result<()> {
    check_root()?;

    // Flush all rules for the IP
    run_sudo_command("pfctl", &["-F", "all"])?;

    // Remove persistent rules if they exist
    if Path::new("/etc/pf.anchors/com.wifi-kicker").exists() {
        run_sudo_command("rm", &["/etc/pf.anchors/com.wifi-kicker"])?;
    }

    info!("Removed all rules for {}", ip);
    Ok(())
}

fn show_status() -> Result<()> {
    check_root()?;

    println!("Current PF rules:");
    let rules_output = run_sudo_command("pfctl", &["-sr"])?;
    println!("{}", String::from_utf8_lossy(&rules_output.stdout));

    println!("\nCurrent states:");
    let states_output = run_sudo_command("pfctl", &["-ss"])?;
    println!("{}", String::from_utf8_lossy(&states_output.stdout));

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
        Commands::Monitor { ip, persistent } => {
            setup_monitoring(ip, *persistent)?;
        }
        Commands::Limit {
            ip,
            upload,
            download,
            persistent,
        } => {
            setup_bandwidth_limit(ip, *upload, *download, *persistent)?;
        }
        Commands::Remove { ip } => {
            remove_rules(ip)?;
        }
        Commands::Status => {
            show_status()?;
        }
    }

    Ok(())
}
