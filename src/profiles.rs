use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Profile {
    pub name: String,
    pub user: String,
    pub email: String,
    pub signing: bool,
    pub key: Option<String>,
    pub ssh_key: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProfilesContent {
    profiles: Vec<Profile>,
}

pub struct ProfileRepository;

impl ProfileRepository {
    pub fn get_all() -> Result<Vec<Profile>> {
        let config_file = Self::get_config_file().context("Error getting config file")?;
        let contents = std::fs::read(&config_file).context("error reading contents from the config file")?;
        if contents.is_empty() {
            return Ok(vec![]);
        }
        let contents_as_str = std::str::from_utf8(&contents).context("error converting config file contents to string")?;
        let profiles: ProfilesContent = toml::from_str(contents_as_str).context("error reading profile list")?;
        Ok(profiles.profiles)
    }

    pub fn find_by_name(profile_name: &str) -> Result<Option<Profile>> {
        let all = Self::get_all().context("Error getting profiles")?;
        for p in all {
            if p.name == profile_name {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }

    pub fn create(p: Profile) -> Result<()> {
        let mut all = Self::get_all().context("Error getting profiles")?;
        for profile in all.iter() {
            if profile.name == p.name {
                warn!("You are trying to create a profile with a name that already exists! Please remove it first");
                return Ok(());
            }
        }
        all.push(p);
        Self::store_profiles(&ProfilesContent { profiles: all }).context("Error storing profiles")?;
        Ok(())
    }

    pub fn remove(profile_name: &str) -> Result<()> {
        let all = Self::get_all().context("Error retrieving profiles")?;
        let mut new = Vec::new();
        let mut found = false;
        for profile in all {
            if profile_name == profile.name {
                found = true;
            } else {
                new.push(profile);
            }
        }

        if !found {
            warn!("Could not find a profile with the name {}", profile_name);
        } else {
            Self::store_profiles(&ProfilesContent { profiles: new }).context("Error storing profiles")?;
            info!("Profile {} has been removed", profile_name);
        }
        Ok(())
    }

    fn store_profiles(profiles: &ProfilesContent) -> Result<()> {
        let file = Self::get_config_file().context("Error getting config file")?;
        let content = toml::ser::to_string(profiles).context("error converting profile list to string")?;
        std::fs::write(file, content).context("error writing profile list to file")?;
        Ok(())
    }

    fn get_config_file() -> Result<PathBuf> {
        let config_dir = Self::get_config_dir().context("Error getting config dir")?;
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).context("Error creating directories")?;
        }
        let f = config_dir.join("users.toml");
        if !f.exists() {
            std::fs::File::create(&f).context("Error creating config file")?;
        }
        Ok(f)
    }

    fn get_config_dir() -> Result<PathBuf> {
        let directories = xdg::BaseDirectories::new().context("could not get the base directory")?;
        Ok(directories.get_config_home().join("git-switch-user"))
    }
}
