use anyhow::{Context, Result};
use clap::Parser;
use landlock::{
    Access, AccessFs, PathFd, Ruleset, RulesetAttr, ABI, RulesetCreatedAttr, PathBeneath,
};
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command to run
    #[arg(required = true)]
    command: String,

    /// Arguments for the command
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

fn apply_landlock() -> Result<()> {
    // Define access rights
    // ABI V1 supports basic file/dir operations.
    let abi = ABI::V1;
    
    // Read-only access: Execute, ReadFile, ReadDir, etc.
    // Note: Landlock V1 doesn't explicitly separate Execute, it's covered by file access/open.
    // But conceptually we treat system paths as RO.
    // We explicitly enable all Read bits.
    let access_ro = AccessFs::from_read(abi);
    
    // Read-Write access: All V1 operations
    let access_rw = AccessFs::from_all(abi);

    // Create ruleset
    // We handle all V1 access rights.
    let mut ruleset = Ruleset::default()
        .handle_access(access_rw)?
        .create()
        .context("Failed to create Landlock ruleset")?;

    // Common system paths (Read-Only)
    let ro_paths = [
        "/bin",
        "/usr",
        "/lib",
        "/lib64",
        "/etc",
        "/proc",
        "/sys",
    ];

    for path in ro_paths {
        if Path::new(path).exists() {
            // We ignore errors adding rules for specific paths (best effort)
            // e.g. permission denied on /proc might happen?
            if let Ok(path_fd) = PathFd::new(path) {
                // Use PathBeneath to create a rule
                ruleset = ruleset.add_rule(PathBeneath::new(path_fd, access_ro))?;
            }
        }
    }

    // Common writable paths (Read-Write)
    let rw_paths = [
        "/tmp",
        "/dev",
        "/run", // Often needed for sockets
    ];

    for path in rw_paths {
        if Path::new(path).exists() {
            if let Ok(path_fd) = PathFd::new(path) {
                ruleset = ruleset.add_rule(PathBeneath::new(path_fd, access_rw))?;
            }
        }
    }

    // Current Working Directory (Read-Write)
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(path_fd) = PathFd::new(cwd) {
            ruleset = ruleset.add_rule(PathBeneath::new(path_fd, access_rw))?;
        }
    }
    
    // Be careful with HOME. Usually we want to restrict HOME access?
    // For now, let's assume we ONLY allow CWD if it's in HOME.
    // If CWD is outside, we might break things.
    // The policy "Unprivileged sandboxing" usually means "Only access CWD".
    // But shells need to read config files from HOME (.bashrc etc).
    
    // Let's add HOME as Read-Only? Or maybe just .bashrc?
    // For a generic sandbox, restricting to CWD is the goal, but functionality is key.
    // If I restrict HOME, `bash` might complain but run.
    
    // Apply the ruleset
    ruleset.restrict_self().context("Failed to apply Landlock restrictions")?;
    
    Ok(())
}

fn main() -> Result<()> {
    // Initialize logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt::init();
    }
    
    let args = Args::parse();
    
    if std::env::var("RUST_LOG").is_ok() {
        tracing::info!("Starting sandbox for: {} {:?}", args.command, args.args);
    }
    
    // Apply Landlock restrictions
    // We wrap it in a block to log error but proceed? No, failure to sandbox should be fatal for security.
    // But if kernel doesn't support Landlock?
    match apply_landlock() {
        Ok(_) => {
            if std::env::var("RUST_LOG").is_ok() {
                tracing::info!("Landlock applied successfully");
            }
        }
        Err(e) => {
             // Check if it's "NotSupported"
             // RulesetError enum might help, but anyhow hides it.
             // For now, we assume if it fails, we should fail secure?
             // Or fallback if not supported?
             // "Unprivileged sandboxing" implies it's a feature.
             
             // If we are running on old kernel, we might want to warn and continue?
             // Let's fail for now to be explicit.
             return Err(e.context("Landlock failed (kernel too old?)"));
        }
    }
    
    let err = Command::new(&args.command)
        .args(&args.args)
        .exec();
        
    // If exec returns, it failed
    Err(anyhow::anyhow!("Failed to exec: {}", err))
}
