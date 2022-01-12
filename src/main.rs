use clap::{App, Arg};
use serde::Deserialize;

use std::env::consts::OS;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use sys_info::*;
use which::which;

extern crate scuttle;
extern crate sys_info;

#[derive(Clone, Deserialize, Debug)]
struct Meta {
    command: String,
    args: Vec<String>,
    apps: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
struct PackageManager {
    meta: Meta,
}

#[derive(Deserialize, Debug)]
struct Version {
    types: Vec<String>,
    package_managers: Vec<PackageManager>,
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

fn read_config_from_file<P: AsRef<Path>>(path: P) -> Config {
    // Open the file in read-only mode with buffer.
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => panic!("File not found: {}", error),
    };
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `OS`.
    match serde_json::from_reader(reader) {
        Ok(config) => config,
        Err(error) => panic!("Unable to parse json: {}", error),
    }
}

fn get_package_manager(config: &Config) -> Vec<PackageManager> {
    let operating_systems: &Vec<OperatingSystem> = &config.operating_systems;
    let mut package_managers: Vec<PackageManager> = vec![];

    operating_systems.iter().for_each(|operating_system| {
        if OS == operating_system.name {
            let release = match linux_os_release() {
                Ok(value) => value.id,
                Err(error) => panic!("Error {}", error),
            };

            operating_system.versions.iter().for_each(|version| {
                version
                    .types
                    .iter()
                    .for_each(|name| match release.as_deref() {
                        Some(value) => {
                            if value == name {
                                package_managers = version.package_managers.to_vec();
                            }
                        }
                        None => panic!("ERROR: not sure what distribution this is"),
                    });
            });
        }
        if OS == "macos" {}
    });

    package_managers
}

fn run() -> i32 {
    let args = App::new("biosphere")
	.version("0.1.0")
	.about("Bootstrap your environment with your preferred apps")
	.author("Ricky Nelson")
	.args(&[
	    Arg::new("config")
		.short('c')
		.long("config")
		.takes_value(true),
	]).get_matches();
    let config_file: String = args.value_of_t("config").unwrap_or("".to_string());

    if args.is_present("config") {
        let config = read_config_from_file(config_file);
        let package_manager: Vec<PackageManager> = get_package_manager(&config);

        for package in package_manager.iter() {
            for app in package.meta.apps.iter() {
                let mut args = package.meta.args.to_owned();

                match which(app) {
                    Ok(value) => {
                        println!("{} skipping, found here: {}", app, value.display());
                    }
                    Err(_error) => {
                        args.push(app.to_string());

                        // install the app
                        let installer = scuttle::App {
                            command: package.meta.command.to_owned(),
                            // this is not my code, found this magic here
                            // https://stackoverflow.com/questions/33216514/how-do-i-convert-a-vecstring-to-vecstr
                            args: args.iter().map(|arg| &**arg).collect(),
                        };

                        scuttle::run_app(&installer).unwrap();
                    }
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
