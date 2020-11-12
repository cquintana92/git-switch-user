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
    pub fn get_all() -> Vec<Profile> {
        let config_file = Self::get_config_file();
        let contents =
            std::fs::read(&config_file).expect("error reading contents from the config file");
        if contents.is_empty() {
            return vec![];
        }
        let contents_as_str = std::str::from_utf8(&contents)
            .expect("error converting config file contents to string");
        let profiles: ProfilesContent =
            toml::from_str(&contents_as_str).expect("error reading profile list");
        profiles.profiles
    }

    pub fn find_by_name(profile_name: &str) -> Option<Profile> {
        let all = Self::get_all();
        for p in all {
            if p.name == profile_name {
                return Some(p);
            }
        }
        None
    }

    pub fn create(p: Profile) {
        let mut all = Self::get_all();
        for profile in all.iter() {
            if profile.name == p.name {
                warn!("You are trying to create a profile with a name that already exists! Please remove it first");
                return;
            }
        }
        all.push(p);
        Self::store_profiles(&ProfilesContent { profiles: all });
    }

    pub fn remove(profile_name: &str) {
        let all = Self::get_all();
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
            Self::store_profiles(&ProfilesContent { profiles: new });
            info!("Profile {} has been removed", profile_name);
        }
    }

    fn store_profiles(profiles: &ProfilesContent) {
        let file = Self::get_config_file();
        let content =
            toml::ser::to_string(profiles).expect("error converting profile list to string");
        std::fs::write(file, content).expect("error writing profile list to file");
    }

    fn get_config_file() -> PathBuf {
        let config_dir = Self::get_config_dir();
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).unwrap();
        }
        let f = config_dir.join("users.toml");
        if !f.exists() {
            std::fs::File::create(&f).unwrap();
        }
        f
    }

    fn get_config_dir() -> PathBuf {
        let directories = xdg::BaseDirectories::new().expect("could not get the base directory");
        directories.get_config_home().join("git-switch-user")
    }
}
