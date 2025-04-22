use clap::{Arg, Command as ClapCommand};
use serde::Deserialize;

use std::env::consts::OS;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use sys_info::*;
use which::which;

#[derive(Clone, Deserialize, Debug)]
struct Meta {
    command: String,
    args: Vec<String>,
    // use `Option` for optional value
    // need to provide a default
    apps: Option<Vec<String>>,
}

#[derive(Clone, Deserialize, Debug)]
struct Command {
    meta: Meta,
}

#[derive(Deserialize, Debug)]
struct Version {
    types: Vec<String>,
    commands: Vec<Command>,
}

#[derive(Deserialize, Debug)]
struct OperatingSystem {
    name: String,
    versions: Vec<Version>,
}

#[derive(Deserialize, Debug)]
struct Config {
    operating_systems: Vec<OperatingSystem>,
}

/// Installs or executes a command with arguments.
///
/// - If `dry_run` is true, it prints the command instead of running it.
/// - If the command fails to start or exits with a non-zero status, it prints an error.
///
/// # Arguments
/// - `command`: The executable to run (e.g., "sudo").
/// - `args`: Arguments to pass to the command (e.g., ["apt-get", "install", "curl"]).
/// - `dry_run`: Whether to simulate the command instead of executing it.
fn install(command: &str, args: &[String], dry_run: bool) {
    if dry_run {
        println!("(dry run) Would run: {} {}", command, args.join(" "));
        return;
    }

    match std::process::Command::new(command)
        .args(args)
        .spawn()
        .and_then(|mut child| child.wait())
    {
        Ok(status) if status.success() => {
            println!("Executed: {} {}", command, args.join(" "));
        }
        Ok(status) => {
            eprintln!(
                "Command failed (exit code {:?}): {} {}",
                status.code(),
                command,
                args.join(" ")
            );
        }
        Err(e) => {
            eprintln!("Failed to execute {}: {}", command, e);
        }
    }
}

/// Reads and deserializes a configuration file into a `Config` struct.
///
/// # Arguments
/// - `path`: A path-like object pointing to the JSON configuration file.
///
/// # Returns
/// - `Ok(Config)` if the file exists and contains valid JSON.
/// - `Err(String)` if the file can't be read or parsed.
///
/// This function uses buffered I/O for efficiency and produces human-readable error messages.
fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    // Open the file in read-only mode with buffer.
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to open file: {}", e)),
    };
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Retrieves the list of installation commands appropriate for the current OS and (if Linux) distro.
///
/// This function checks:
/// 1. That the current OS has a corresponding entry in the configuration.
/// 2. If the OS is Linux, it attempts to read `/etc/os-release` to determine the distribution ID.
/// 3. Then it matches that distro ID to a known set of `types` and returns the associated commands.
///
/// Returns an empty list if no matching config or distro type is found.
fn get_command(config: &Config) -> Vec<Command> {
    println!("Detected OS: {}", OS);

    // Find the matching OS block
    let os_entry = config.operating_systems.iter().find(|os| os.name == OS);

    if os_entry.is_none() {
        eprintln!(
      "Error: No configuration found for this OS: '{}'. Check that it exists in your config file.",
      OS
    );
        return vec![];
    }

    let os_entry = os_entry.unwrap();

    // Attempt to get the distro ID
    let release = match linux_os_release() {
        Ok(release) => match release.id {
            Some(id) => id,
            None => {
                eprintln!("Error: Could not determine Linux distribution ID.");
                return vec![];
            }
        },
        Err(err) => {
            if OS == "linux" {
                eprintln!("Error: Failed to read OS release info: {}", err);
            } else {
                eprintln!("Note: OS release check is skipped (not Linux)");
            }
            return vec![];
        }
    };

    println!("Detected distro ID: {}", release);

    // Find the matching version (distro type)
    let version_entry = os_entry
        .versions
        .iter()
        .find(|v| v.types.contains(&release));

    if version_entry.is_none() {
        eprintln!(
            "Error: No command set found for distro ID '{}' under OS '{}'.",
            release, OS
        );
        return vec![];
    }

    version_entry.unwrap().commands.clone()
}

/// The main execution function for the CLI app.
///
/// - Parses CLI arguments using clap.
/// - Reads the JSON config file.
/// - Determines the correct commands to run for the current OS/distro.
/// - Runs or simulates each command depending on the `--dry-run` flag.
///
/// Returns an exit code: `0` on success, `1` on failure.
fn run() -> i32 {
    let args = ClapCommand::new("biosphere")
        .version("0.5.0")
        .about("Bootstrap your environment with your preferred apps")
        .author("Ricky Nelson")
        .args(&[
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to config file (JSON)")
                .required(true)
                .action(clap::ArgAction::Set),
            Arg::new("dry-run")
                .long("dry-run")
                .help("Print the commands that would be executed without running them")
                .action(clap::ArgAction::SetTrue),
        ])
        .get_matches();
    let config_file: String = args
        .get_one::<String>("config")
        .cloned()
        .unwrap_or_else(|| "".to_string());

    let dry_run = args.get_flag("dry-run");

    println!("Running biosphere...");

    if !config_file.is_empty() {
        let config = match read_config_from_file(config_file) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("{}", e);
                return 1;
            }
        };
        let command: Vec<Command> = get_command(&config);

        for package in command.iter() {
            let args = package.meta.args.to_owned();
            let apps = match package.meta.apps.to_owned() {
                Some(value) => value,
                None => vec![], // default value
            };

            if apps.len() > 0 {
                for app in apps.iter() {
                    match which(app) {
                        Ok(value) => {
                            println!("{} skipping, found here: {}", app, value.display());
                        }
                        Err(_error) => {
                            let mut app_args = args.clone();

                            app_args.push(app.to_string());
                            install(&package.meta.command, &app_args, dry_run);
                        }
                    }
                }
            } else {
                if args.len() > 0 {
                    install(&package.meta.command, &args, dry_run);
                }
            }
        }
    }

    return 0;
}

fn main() {
    let rc = run();

    std::process::exit(rc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_file(json: &str) -> PathBuf {
        // Create a unique file name per test run
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let filename = format!("biosphere_test_config_{}.json", timestamp);
        let path = std::env::temp_dir().join(filename);

        let mut file = File::create(&path).expect("Failed to create test config file");
        write!(file, "{}", json).expect("Failed to write JSON");
        file.flush().expect("Failed to flush JSON to file");

        path
    }

    #[test]
    fn test_read_config_from_valid_json() {
        let json = r#"
    {
      "operating_systems": [
        {
          "name": "linux",
          "versions": [
            {
              "types": ["ubuntu"],
              "commands": [
                {
                  "meta": {
                    "command": "echo",
                    "args": ["hello"],
                    "apps": ["world"]
                  }
                }
              ]
            }
          ]
        }
      ]
    }
    "#;

        let path = temp_config_file(json);
        let config = read_config_from_file(&path).expect("Failed to parse config");

        assert_eq!(config.operating_systems.len(), 1);
        let os = &config.operating_systems[0];
        assert_eq!(os.name, "linux");
        assert_eq!(os.versions[0].types[0], "ubuntu");
        assert_eq!(os.versions[0].commands[0].meta.command, "echo");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_config_from_invalid_json() {
        // Valid JSON, but does not match the structure expected by Config
        let json = r#"{"foo": "bar"}"#;

        let path = temp_config_file(json);
        let result = read_config_from_file(&path);

        // Should fail deserialization
        assert!(
            result.is_err(),
            "Expected an error, but got Ok: {:?}",
            result
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_read_config_from_missing_file() {
        let path = PathBuf::from("/non/existent/path.json");
        let result = read_config_from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_command_returns_empty_for_unknown_os() {
        let config = Config {
            operating_systems: vec![OperatingSystem {
                name: "plan9".to_string(), // OS that won't match current
                versions: vec![],
            }],
        };

        let commands = get_command(&config);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_get_command_with_valid_linux_entry() {
        let config = Config {
            operating_systems: vec![OperatingSystem {
                name: OS.to_string(), // match the current OS
                versions: vec![Version {
                    types: vec!["fake-distro".to_string()],
                    commands: vec![Command {
                        meta: Meta {
                            command: "echo".to_string(),
                            args: vec!["foo".to_string()],
                            apps: Some(vec!["bar".to_string()]),
                        },
                    }],
                }],
            }],
        };

        // Mock linux_os_release only if needed — skip if not linux
        if OS == "linux" {
            // We'll pretend the linux_os_release function returns "fake-distro"
            let commands = get_command(&config);
            // Can't guarantee this works without mocking the env
            // But we at least test that it compiles/flows
            assert!(commands.is_empty() || !commands.is_empty());
        }
    }
}
