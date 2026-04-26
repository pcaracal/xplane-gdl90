use std::rc::Rc;

use xplm_egui::flight_loop::FlightLoopCallback;

use crate::{data::Datarefs, socket::Socket};

#[derive(Builder)]
pub struct FlightLoopHandler {
    datarefs: Rc<Datarefs>,
    socket: Socket,
}

impl FlightLoopCallback for FlightLoopHandler {
    fn flight_loop(&mut self, _: &mut xplm_egui::flight_loop::LoopState) {
        let data = self.datarefs.data();
        if let Err(why) = self.socket.send_data(&data) {
            error!("Failed to send GDL90 data: {why}");
        }
    }
}
