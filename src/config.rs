use anyhow::Context;
use directories::ProjectDirs;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RemoteDatabaseSettings {
    pub postgres_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IntervalSettings {
    #[serde(default = "default_processing_interval")]
    pub processing: u64,
    #[serde(default = "default_saving_interval")]
    pub saving: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(default)]
    pub database: RemoteDatabaseSettings,
    #[serde(default)]
    pub intervals_ms: IntervalSettings,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

// Default functions for serde
fn default_processing_interval() -> u64 {
    250
}
fn default_saving_interval() -> u64 {
    60000
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Default for IntervalSettings {
    fn default() -> Self {
        Self {
            processing: default_processing_interval(),
            saving: default_saving_interval(),
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            // local_database: LocalDatabaseSettings { path: default_local_db_path() }, // If configurable
            database: RemoteDatabaseSettings { postgres_url: None },
            intervals_ms: IntervalSettings {
                processing: default_processing_interval(), // Ensure these defaults exist
                saving: default_saving_interval(),
            },
            log_level: default_log_level(),
        }
    }
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "seatedro", "etsu")
            .context("Failed to get project directories")?;
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        let config_file = config_dir.join("config.toml");

        let builder = config::Config::builder()
            .set_default("database.postgres_url", None::<String>)?
            .set_default("intervals_ms.processing", default_processing_interval())?
            .set_default("intervals_ms.saving", default_saving_interval())?
            .set_default("log_level", default_log_level())?
            .add_source(config::File::from(config_file).required(false))
            .add_source(config::Environment::with_prefix("ETSU").separator("__"));

        let settings = builder.build()?.try_deserialize()?;

        Ok(settings)
    }

    pub fn get_local_sqlite_path(&self) -> anyhow::Result<PathBuf> {
        let db_filename = "etsu.db";

        let path = PathBuf::from(db_filename);
        if path.is_absolute() {
            Ok(path)
        } else {
            let proj_dirs = ProjectDirs::from("com", "seatedro", "etsu")
                .context("Failed to get project directories for local DB path")?;
            let data_dir = proj_dirs.data_local_dir();
            std::fs::create_dir_all(data_dir).context("Failed to create local data directory")?;
            Ok(data_dir.join(path))
        }
    }

    pub fn processing_interval(&self) -> Duration {
        Duration::from_millis(self.intervals_ms.processing)
    }
    pub fn saving_interval(&self) -> Duration {
        Duration::from_millis(self.intervals_ms.saving)
    }
}
