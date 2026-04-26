use std::{cell::Cell, net::SocketAddr, path::PathBuf, time::Duration};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::PLUGIN_FOLDER;

const STATE_FILE: &str = "state.ron";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Builder)]
pub struct State {
    #[builder(skip(ctor))]
    pub target: Cell<Option<SocketAddr>>,
    #[builder(skip(ctor), default = Self::DEFAULT_INTERVAL.into())]
    pub interval: Cell<Duration>,
}

impl State {
    pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);
    pub const MINIMUM_INTERVAL: Duration = Duration::from_millis(10);

    #[must_use]
    pub fn load() -> Self {
        match Self::load_file() {
            Ok(state) => state,
            Err(why) => {
                error!("Failed to load state: {why}");
                Self::new()
            }
        }
    }

    pub fn save(&self) {
        if let Err(why) = self.save_file() {
            error!("Failed to save state: {why}");
        }
    }

    fn state_path() -> anyhow::Result<PathBuf> {
        Ok(std::env::current_dir()?
            .join("Resources/plugins/")
            .join(PLUGIN_FOLDER)
            .join(STATE_FILE))
    }

    fn load_file() -> anyhow::Result<Self> {
        let s = std::fs::read_to_string(Self::state_path()?)?;
        ron::from_str(&s).map_err(Into::into)
    }

    fn save_file(&self) -> anyhow::Result<()> {
        std::fs::write(
            Self::state_path()?,
            ron::ser::to_string_pretty(self, PrettyConfig::default())?,
        )
        .map_err(Into::into)
    }
}
