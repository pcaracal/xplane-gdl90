use std::{
    cell::{Ref, RefCell, RefMut},
    path::PathBuf,
    rc::Rc,
};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{PLUGIN_FOLDER, socket::SocketState};

const STATE_FILE: &str = "state.ron";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State(Rc<RefCell<StateInner>>);

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateInner {
    pub socket: SocketState,

    pub heartbeat: bool,
    pub ownship: bool,
    pub ahrs: bool,
}

impl Default for StateInner {
    fn default() -> Self {
        Self {
            socket: SocketState::default(),
            heartbeat: true,
            ownship: true,
            ahrs: true,
        }
    }
}

impl State {
    pub fn borrow(&self) -> Ref<'_, StateInner> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, StateInner> {
        self.0.borrow_mut()
    }

    #[must_use]
    pub fn load() -> Self {
        match Self::load_file() {
            Ok(state) => Self(Rc::new(state.into())),
            Err(why) => {
                error!("Failed to load state: {why}");
                Self::default()
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

    fn load_file() -> anyhow::Result<StateInner> {
        let s = std::fs::read_to_string(Self::state_path()?)?;
        ron::from_str(&s).map_err(Into::into)
    }

    fn save_file(&self) -> anyhow::Result<()> {
        std::fs::write(
            Self::state_path()?,
            ron::ser::to_string_pretty(&*self.0.borrow(), PrettyConfig::default())?,
        )
        .map_err(Into::into)
    }
}
