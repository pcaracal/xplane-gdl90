#![deny(clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::missing_errors_doc)]

mod app;
mod data;
mod flight_loop;
mod socket;
mod state;
mod util;

#[macro_use]
extern crate log;
#[macro_use]
extern crate utilities_derive;
extern crate xplm_egui;

use std::{cell::RefCell, rc::Rc, time::Duration};

use xplm_egui::{
    debugln,
    display::get_screen_bounds_global,
    egui_window::EguiWindow,
    flight_loop::FlightLoop,
    geometry::ScreenRect,
    menu::{ActionItem, Menu, MenuClickHandler},
    plugin::{Plugin, PluginInfo, reload_plugins},
    xplane_plugin,
};

use crate::{
    app::EguiApp, data::Datarefs, flight_loop::FlightLoopHandler, socket::Socket, state::State,
    util::init_log,
};

pub const PLUGIN_FOLDER: &str = "xplane-gdl90";

#[allow(unused)]
struct GDL90Plugin {
    menu: Menu,
    state: State,
    flight_loop: Rc<RefCell<FlightLoop>>,
}

impl Plugin for GDL90Plugin {
    type Error = anyhow::Error;

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: String::from("Rust GDL90 Plugin"),
            signature: String::from("ch.pcaracal.xplane-gdl90"),
            description: String::from(
                "A plugin written in rust sending position data to an EFB using the GDL90 protocol",
            ),
        }
    }

    fn start() -> Result<Self, Self::Error> {
        if let Err(why) = init_log() {
            error!("Failed to initialize log: {why}");
            debugln!("Failed to initialize log: {why}");
        }

        let state = State::load();
        let socket = Socket::new(state.clone())?;

        let datarefs = Rc::new(Datarefs::new()?);
        let flight_loop = Rc::new(RefCell::new(FlightLoop::new(FlightLoopHandler::new(
            datarefs.clone(),
            socket.clone(),
        ))));

        let menu = Menu::new("GDL90")?;
        menu.add_child(ActionItem::new(
            "Toggle window",
            ToggleWindow::Uninitialized {
                state: state.clone(),
                datarefs,
                flight_loop: flight_loop.clone(),
                socket,
            },
        )?);
        menu.add_child(ActionItem::new("Reload Plugins", |_: &ActionItem| {
            warn!("Reloading plugins...");
            reload_plugins();
        })?);
        menu.add_to_plugins_menu();

        Ok(GDL90Plugin {
            menu,
            state,
            flight_loop,
        })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        info!("Enabling plugin");
        self.flight_loop
            .borrow_mut()
            .schedule_after(Duration::from_millis(10));
        Ok(())
    }

    fn disable(&mut self) {
        info!("Disabling plugin");
        self.state.save();
        self.flight_loop.borrow_mut().deactivate();
    }
}

xplane_plugin!(GDL90Plugin);

impl Drop for GDL90Plugin {
    fn drop(&mut self) {
        info!("Dropping plugin");
    }
}

enum ToggleWindow {
    Initialized(EguiWindow),
    Uninitialized {
        state: State,
        datarefs: Rc<Datarefs>,
        flight_loop: Rc<RefCell<FlightLoop>>,
        socket: Socket,
    },
}

impl MenuClickHandler for ToggleWindow {
    fn item_clicked(&mut self, _: &ActionItem) {
        if let Err(why) = self.toggle() {
            error!("Failed to create window: {why}");
        }
    }
}

impl ToggleWindow {
    fn toggle(&mut self) -> anyhow::Result<()> {
        match self {
            ToggleWindow::Initialized(w) => {
                w.set_visible(!w.visible());
                Ok(())
            }
            ToggleWindow::Uninitialized {
                state,
                datarefs,
                flight_loop,
                socket,
            } => {
                let screen = get_screen_bounds_global();
                let window_geometry = ScreenRect::zero()
                    .translate(screen.center().to_vector())
                    .inflate(200, 200);

                let window = EguiWindow::new(
                    EguiApp::new(
                        state.clone(),
                        datarefs.clone(),
                        flight_loop.clone(),
                        socket.clone(),
                    ),
                    window_geometry,
                )?;

                window.set_title("GDL90")?;
                window.set_resizing_limits(100, 100, None, None);
                window.set_visible(true);
                *self = ToggleWindow::Initialized(window);

                Ok(())
            }
        }
    }
}
