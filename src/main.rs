#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate log;

use clap::{App, Arg, ArgMatches, SubCommand};
use prettytable::Table;
use std::io::Write;
use std::process::Command;

mod profiles;

const VERSION: &str = git_version::git_version!();
const LIST_SUBCOMMAND: &str = "list";
const CREATE_SUBCOMMAND: &str = "create";
const DELETE_SUBCOMMAND: &str = "delete";
const SET_SUBCOMMAND: &str = "set";
const PROFILE_ARG: &str = "profile";
const GLOBAL_ARG: &str = "global";

fn exit_with_log(msg: &str) {
    error!("{}", msg);
    std::process::exit(1);
}

fn git_command(args: &[&str]) -> String {
    debug!("Running git {:?}", args);
    let output = Command::new("git")
        .args(args)
        .output()
        .expect("failed to run git command");

    if output.status.success() {
        let res = std::str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .to_string();
        debug!("Output: {}", res);
        res
    } else {
        let msg = format!("Error running git command: git {:?}", args);
        exit_with_log(&msg);
        unreachable!()
    }
}

fn get_current_email() -> String {
    git_command(&["config", "--get", "user.email"])
}

fn list() {
    let all_profiles = profiles::ProfileRepository::get_all();
    let mut table = Table::new();
    table.add_row(row!["Name", "User", "Email", "Sign", "Key", "SSH Key"]);
    let current_email = get_current_email();
    for profile in all_profiles {
        let signing = if profile.signing { "✓" } else { "X" };
        let key = match profile.key {
            Some(k) => k,
            None => "".to_string(),
        };
        let name = if profile.email == current_email {
            format!("* {}", profile.name)
        } else {
            profile.name
        };
        let ssh_key = match profile.ssh_key {
            Some(k) => k,
            None => "".to_string(),
        };
        table.add_row(row![
            name,
            profile.user,
            profile.email,
            signing,
            key,
            ssh_key
        ]);
    }
    table.printstd();
}

fn ask_for_variable(var_name: &str) -> String {
    loop {
        let mut buff = String::new();
        print!("{}: ", var_name);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut buff).unwrap();
        if buff.is_empty() {
            warn!("The value cannot be empty");
        } else {
            return buff.trim().to_string();
        }
    }
}

#[allow(clippy::match_like_matches_macro)]
fn ask_for_bool(prompt: &str) -> bool {
    let mut buff = String::new();
    print!("{} [y/N]: ", prompt);
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut buff).unwrap();
    match buff.as_str().trim() {
        "y" | "Y" | "yes" | "YES" | "Yes" => true,
        _ => false,
    }
}

fn create() {
    let name = ask_for_variable("Profile name");
    let user = ask_for_variable("Git user");
    let email = ask_for_variable("Git email");
    let signing = ask_for_bool("Sign commits?");
    let key = if signing {
        Some(ask_for_variable("Key fingerprint"))
    } else {
        None
    };
    let custom_ssh_key = ask_for_bool("Use custom ssh key?");
    let ssh_key = if custom_ssh_key {
        Some(ask_for_variable("Path to the SSH key"))
    } else {
        None
    };

    let profile = profiles::Profile {
        name,
        user,
        email,
        signing,
        key,
        ssh_key,
    };
    profiles::ProfileRepository::create(profile);
}

fn delete(matches: &ArgMatches) {
    let profile = matches
        .value_of(PROFILE_ARG)
        .expect("you must specify a profile");
    profiles::ProfileRepository::remove(profile);
}

fn git_set(var: &str, value: &str, global: bool) {
    let scope = if global { "--global" } else { "--local" }.to_string();
    git_command(&["config", &scope, var, value]);
}

fn git_unset(var: &str, global: bool) {
    let scope = if global { "--global" } else { "--local" }.to_string();
    git_command(&["config", "--unset", &scope, var]);
}

fn set(matches: &ArgMatches) {
    let profile = matches
        .value_of(PROFILE_ARG)
        .expect("you must specify a profile");
    let global = matches.is_present(GLOBAL_ARG);
    let p = profiles::ProfileRepository::find_by_name(profile);
    let p = match p {
        Some(p) => p,
        None => {
            let msg = format!("Could not find a profile with the name {}", profile);
            exit_with_log(&msg);
            unreachable!()
        }
    };
    git_set("user.name", &p.user, global);
    git_set("user.email", &p.email, global);

    if p.signing {
        let key = match p.key {
            Some(k) => k,
            None => {
                error!(
                    "Profile {} has signing set to true, but has no associated PGP key",
                    profile
                );
                std::process::exit(1);
            }
        };
        git_set("commit.gpgsign", "true", global);
        git_set("tag.gpgsign", "true", global);
        git_set("user.signingkey", &key, global);
    } else {
        git_unset("commit.gpgsign", global);
        git_unset("tag.gpgsign", global);
        git_unset("user.signingkey", global);
    }

    match p.ssh_key {
        Some(k) => {
            git_set("core.sshCommand", &format!("ssh -i {}", k), global);
        }
        None => {
            git_unset("core.sshCommand", global);
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("INFO")).init();

    let app = App::new("git-switch-user")
        .version(VERSION)
        .subcommand(
            SubCommand::with_name(LIST_SUBCOMMAND)
                .alias("l")
                .help("List the available profiles")
                .about("List the available profiles"),
        )
        .subcommand(
            SubCommand::with_name(CREATE_SUBCOMMAND)
                .alias("c")
                .help("Create a new profile")
                .about("Create a new profile"),
        )
        .subcommand(
            SubCommand::with_name(SET_SUBCOMMAND)
                .alias("s")
                .help("Set the current profile")
                .about("Set the current profile")
                .arg(
                    Arg::with_name(PROFILE_ARG)
                        .help("Name of the profile to be used")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name(GLOBAL_ARG)
                        .short("g")
                        .long("global")
                        .help("Set the profile globally")
                        .takes_value(false)
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name(DELETE_SUBCOMMAND)
                .alias("d")
                .help("Delete a profile")
                .about("Delete a profile")
                .arg(
                    Arg::with_name(PROFILE_ARG)
                        .help("Name of the profile to be deleted")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    match app.subcommand() {
        (LIST_SUBCOMMAND, _) => list(),
        (CREATE_SUBCOMMAND, _) => create(),
        (DELETE_SUBCOMMAND, Some(m)) => delete(m),
        (SET_SUBCOMMAND, Some(m)) => set(m),
        _ => list(),
    }
}
