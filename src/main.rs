use std::env::consts::OS;
use which::which;

extern crate scuttle;
extern crate sys_info;

use sys_info::*;

fn main() {
    let packaged_apps: &[String] = &[
        String::from("tmux"),
        String::from("weechat"),
        String::from("shellcheck"),
        String::from("gimp"),
        String::from("krita"),
        String::from("openshot-qt"),
        String::from("magnus"),
        String::from("obs"),
        String::from("vlc"),
        String::from("xcolor"),
        String::from("flameshot"),
        String::from("peek"),
    ];
    let rust_apps: &[String] = &[
        String::from("exa"),
        String::from("bat"),
        String::from("gitui"),
        String::from("pastel"),
        String::from("dtool"),
        String::from("watchexec"),
        String::from("t-rec"),
    ];
    let fzf_clone = scuttle::App {
        command: String::from("git"),
        args: vec!["clone", "--depth", "1", "https://github.com/junegunn/fzf.git", "~/.fzf"]
    };
    let fzf_install = scuttle::App {
        command: String::from("~/.fzf/install"),
        args: vec![]
    };

    for packaged_app in packaged_apps.iter() {
        match which(packaged_app) {
            Ok(value) => {
                println!("{} skipping, found here: {}", packaged_app, value.display());
            },
            Err(_error) => {
                // install the app
                if OS == "linux" {
                    let release = match linux_os_release() {
                        Ok(value) => value.id,
                        Err(error) => panic!("Error {}", error)
                    };

                    match release.as_deref() {
                        Some("pop") | Some("ubuntu") => {
                            let package_manager = scuttle::App {
                                command: String::from("sudo"),
                                args: vec!["apt-get", "install", "-y", packaged_app]
                            };

                            scuttle::run_app(&package_manager).unwrap();
                        },
                        Some("arch") => {
                        },
                        Some(&_) => panic!("ERROR: not sure what distribution this is"),
                        None => panic!("ERROR: not sure what distribution this is")
                    }
                }

                if OS == "macos" {
                }
            }
        }
    }

    for rust_app in rust_apps.iter() {
        match which(rust_app) {
            Ok(value) => {
                println!("{} skipping, found here: {}", rust_app, value.display());
            },
            Err(_error) => {
                let cargo_install = scuttle::App {
                    command: String::from("cargo"),
                    args: vec!["install", rust_app]
                };

                scuttle::run_app(&cargo_install).unwrap();
            }
        }
    }

    match which(String::from("fzf")) {
        Ok(value) => {
            println!("{} skipping, found here: {}", String::from("fzf"), value.display());
        },
        Err(_error) => {
            let install_fzf: &[scuttle::App] = &[fzf_clone, fzf_install];

            scuttle::run_apps(&install_fzf);
        }
    }
}
