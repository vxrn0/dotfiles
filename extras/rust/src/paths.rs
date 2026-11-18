use std::env;
use std::path::PathBuf;

pub const QUICKSHELL_CONFIG_NAME: &str = "dotfiles";

pub struct Paths {
    pub home: PathBuf,
    pub config_home: PathBuf,
    pub state_home: PathBuf,
    pub cache_home: PathBuf,
}

impl Paths {
    pub fn from_env() -> Result<Self, String> {
        let home = non_empty("HOME").ok_or("HOME is not set")?;
        let home = PathBuf::from(home);
        let config_home = non_empty("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let state_home = non_empty("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));
        let cache_home = non_empty("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"));
        Ok(Self {
            home,
            config_home,
            state_home,
            cache_home,
        })
    }

    /// Same location as switchwall.sh's SCRIPT_DIR on an installed system.
    pub fn colors_dir(&self) -> PathBuf {
        self.config_home
            .join("quickshell")
            .join(QUICKSHELL_CONFIG_NAME)
            .join("scripts/colors")
    }

    pub fn shell_config(&self) -> PathBuf {
        self.config_home.join("dotfiles/config.json")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.state_home.join("quickshell")
    }

    pub fn generated_dir(&self) -> PathBuf {
        self.state_dir().join("user/generated")
    }

    pub fn cache_generated_dir(&self) -> PathBuf {
        self.cache_home.join("quickshell/user/generated")
    }

    pub fn custom_scripts_dir(&self) -> PathBuf {
        self.config_home.join("hypr/custom/scripts")
    }
}

fn non_empty(name: &str) -> Option<String> {
    env::var_os(name)
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
}
