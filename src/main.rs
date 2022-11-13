#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate log;

use anyhow::{anyhow, Context, Result};
use clap::{App, Arg, ArgMatches, SubCommand};
use prettytable::Table;
use std::io::Write;
use std::process::Command;

mod profiles;

const VERSION: &str = git_version::git_version!(args = ["--tags", "--abbrev=1", "--dirty=-modified"]);
const LIST_SUBCOMMAND: &str = "list";
const CREATE_SUBCOMMAND: &str = "create";
const DELETE_SUBCOMMAND: &str = "delete";
const SET_SUBCOMMAND: &str = "set";
const PROFILE_ARG: &str = "profile";
const GLOBAL_ARG: &str = "global";

enum GitOutput {
    Value(String),
    NoValue,
}

fn git_command(args: &[&str]) -> Result<GitOutput> {
    debug!("Running git {:?}", args);
    let output = Command::new("git").args(args).output().context("Failed to run git command")?;

    match output.status.code() {
        Some(code) => match code {
            0 => {
                let res = std::str::from_utf8(&output.stdout)
                    .context("Error converting git output to string")?
                    .trim()
                    .to_string();
                debug!("Output: {}", res);
                Ok(GitOutput::Value(res))
            }
            1 => Ok(GitOutput::NoValue),
            _ => Err(anyhow!("Error running git command: git {:?}", args)),
        },
        None => Err(anyhow!("Error running git command: git {:?}", args)),
    }
}

fn get_current_email() -> Result<String> {
    match git_command(&["config", "--get", "user.email"]).context("Error getting current user email")? {
        GitOutput::NoValue => Ok("".to_string()),
        GitOutput::Value(v) => Ok(v),
    }
}

fn list() -> Result<()> {
    let all_profiles = profiles::ProfileRepository::get_all().context("Error retrieving profiles")?;
    let mut table = Table::new();
    table.add_row(row!["Name", "User", "Email", "Sign", "Key", "SSH Key"]);
    let current_email = get_current_email().context("Error getting user email")?;
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
        table.add_row(row![name, profile.user, profile.email, signing, key, ssh_key]);
    }
    table.printstd();
    Ok(())
}

fn ask_for_variable(var_name: &str) -> Result<String> {
    loop {
        let mut buff = String::new();
        print!("{}: ", var_name);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut buff).context("Error getting line from stdin")?;
        if buff.is_empty() {
            warn!("The value cannot be empty");
        } else {
            return Ok(buff.trim().to_string());
        }
    }
}

#[allow(clippy::match_like_matches_macro)]
fn ask_for_bool(prompt: &str) -> Result<bool> {
    let mut buff = String::new();
    print!("{} [y/N]: ", prompt);
    std::io::stdout().flush().context("Error flushing stdout")?;
    std::io::stdin().read_line(&mut buff).context("Error getting line from stdout")?;
    match buff.as_str().trim() {
        "y" | "Y" | "yes" | "YES" | "Yes" => Ok(true),
        _ => Ok(false),
    }
}

fn create() -> Result<()> {
    let name = ask_for_variable("Profile name").context("Error getting profile name")?;
    let user = ask_for_variable("Git user").context("Error getting git user")?;
    let email = ask_for_variable("Git email").context("Error getting git email")?;
    let signing = ask_for_bool("Sign commits?").context("Error getting sign commits var")?;
    let key = if signing {
        Some(ask_for_variable("Key fingerprint").context("Error getting key fingerprint")?)
    } else {
        None
    };
    let custom_ssh_key = ask_for_bool("Use custom ssh key?").context("Error getting custom ssh key var")?;
    let ssh_key = if custom_ssh_key {
        Some(ask_for_variable("Path to the SSH key").context("Error getting path to SSH key")?)
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
    profiles::ProfileRepository::create(profile).context("Error creating profile")?;
    Ok(())
}

fn delete(matches: &ArgMatches) -> Result<()> {
    let profile = matches.value_of(PROFILE_ARG).expect("you must specify a profile");
    profiles::ProfileRepository::remove(profile).context("Error removing profile")?;
    Ok(())
}

fn git_set(var: &str, value: &str, global: bool) -> Result<()> {
    let scope = if global { "--global" } else { "--local" }.to_string();
    git_command(&["config", &scope, var, value]).context("Error running git command")?;
    Ok(())
}

fn git_unset(var: &str, local_default: &str, global: bool) -> Result<()> {
    let scope = if global { "--global" } else { "--local" }.to_string();
    if !global {
        if let GitOutput::Value(_) = git_command(&["config", "--get", "--global", var]).context("Error running git command")? {
            // global value exists that might have unintended consquences
            // for our local settings. We set local value to a sensible
            // default to prevent global setting from having an effect
            debug!("Found global value for {}. Setting local value to '{}'", var, local_default);
            git_set(var, local_default, global).context("Error running git set")?;
            // early return - otherwise the setting we just made will get
            // unset again later in this routine
            return Ok(());
        } else {
            debug!("Global value for {} not found. Unsetting local value", var);
        }
    }

    let _ = git_command(&["config", "--unset", &scope, var]).context("Error running git command")?;
    Ok(())
}

fn set(matches: &ArgMatches) -> Result<()> {
    let profile = matches.value_of(PROFILE_ARG).context("you must specify a profile")?;
    let global = matches.is_present(GLOBAL_ARG);
    let p = profiles::ProfileRepository::find_by_name(profile).context("Error finding profile")?;
    let p = match p {
        Some(p) => p,
        None => {
            return Err(anyhow!("Could not find a profile with the name {}", profile));
        }
    };
    git_set("user.name", &p.user, global).context("Error running git set user.name")?;
    git_set("user.email", &p.email, global).context("Error running git set user.email")?;

    if p.signing {
        let key = match p.key {
            Some(k) => k,
            None => {
                return Err(anyhow!(
                    "Profile {} has signing set to true, but has no associated PGP key",
                    profile
                ));
            }
        };
        git_set("commit.gpgsign", "true", global).context("Error running commit.gpgsign")?;
        git_set("tag.gpgsign", "true", global).context("Error running tag.gpgsign")?;
        git_set("user.signingkey", &key, global).context("Error running user.signingkey")?;
    } else {
        git_unset("commit.gpgsign", "false", global).context("Error running commit.gpgsign")?;
        git_unset("tag.gpgsign", "false", global).context("Error running tag.gpgsign")?;
        git_unset("user.signingkey", "", global).context("Error running user.signingkey")?;
    }

    match p.ssh_key {
        Some(k) => {
            git_set("core.sshCommand", &format!("ssh -i {}", k), global).context("Error setting core.sshCommand")?;
        }
        None => {
            git_unset("core.sshCommand", "ssh", global).context("Error setting core.sshCommand")?;
        }
    }
    Ok(())
}

fn run() -> Result<()> {
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

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("INFO")).init();
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}
