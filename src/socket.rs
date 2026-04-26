use std::{
    cell::Cell,
    net::{SocketAddr, UdpSocket},
    rc::Rc,
};

use gdl90::prelude::*;

use crate::data::Data;

#[derive(Debug, Clone)]
pub struct Socket {
    socket: Rc<UdpSocket>,
    target: Rc<Cell<Option<SocketAddr>>>,
}

impl Socket {
    pub fn new() -> anyhow::Result<Self> {
        let socket = Rc::new(UdpSocket::bind("0.0.0.0:0")?);
        // socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            target: Rc::default(),
        })
    }

    pub fn set_target(&self, target: Option<SocketAddr>) {
        self.target.set(target);
    }

    pub fn send_data(&self, data: &Data) -> anyhow::Result<()> {
        let Some(target) = self.target.get() else {
            return Ok(());
        };

        let heartbeat = Heartbeat::default()
            .with_gps_pos_valid()
            .with_uat_initialized()
            .with_utc_ok()
            .with_timestamp_now();

        let ownship = TrafficReport::default()
            .with_latitude(data.lat)
            .with_longitude(data.lon)
            .with_altitude(data.alt)
            .with_horizontal_velocity(data.gs)
            .with_vertical_velocity(data.vs)
            .with_track_heading(data.true_hdg)
            .with_miscellaneous_indicators(MiscellaneousIndicators::new(
                AirGroundState::Airborne,
                ReportType::Updated,
                TrackHeadingType::HeadingTrue,
            ))
            .ownship();

        let ahrs = ForeFlightAHRS::default()
            .with_heading(data.true_hdg)
            .with_heading_type(AHRSHeadingType::True);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&heartbeat.into_gdl90_bytes()?);
        bytes.extend_from_slice(&ownship.into_gdl90_bytes()?);
        bytes.extend_from_slice(&ahrs.into_gdl90_bytes()?);

        if !self.socket.broadcast()? {
            self.socket.set_broadcast(true)?;
            debug!("Enabled broadcast on socket");
        }
        self.socket.send_to(&bytes, target)?;

        match self.socket.take_error() {
            Ok(Some(error)) => error!("UdpSocket error: {error:?}"),
            Ok(None) => (),
            Err(error) => error!("UdpSocket.take_error failed: {error:?}"),
        }

        Ok(())
    }
}
