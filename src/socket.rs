use std::{
    cell::Cell,
    collections::BTreeMap,
    net::{SocketAddr, UdpSocket},
    rc::Rc,
    time::{Duration, Instant},
};

use gdl90::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{data::Data, state::State};

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct SocketState {
    pub interval: Duration,
    pub target: Option<SocketAddr>,

    #[serde(skip)]
    pub auto_targets: BTreeMap<SocketAddr, FFBroadcast>,
}

impl PartialEq for SocketState {
    fn eq(&self, other: &Self) -> bool {
        self.interval == other.interval && self.target == other.target
    }
}

impl SocketState {
    pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(1);
    pub const MINIMUM_INTERVAL: Duration = Duration::from_millis(10);
}

impl Default for SocketState {
    fn default() -> Self {
        Self {
            interval: Self::DEFAULT_INTERVAL,
            target: None,
            auto_targets: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Socket {
    #[allow(clippy::struct_field_names)]
    socket: Rc<UdpSocket>,
    state: State,
    last_send: Cell<Option<Instant>>,
}

impl Socket {
    pub fn new(state: State) -> anyhow::Result<Self> {
        let socket = Rc::new(UdpSocket::bind("0.0.0.0:63093")?);
        socket.set_nonblocking(true)?;
        socket.set_broadcast(true)?;
        Ok(Self {
            socket,
            state,
            last_send: Cell::new(None),
        })
    }

    fn recv(&self) -> anyhow::Result<()> {
        let mut buf = [0; 255];

        if let Ok((read, mut addr)) = self.socket.recv_from(&mut buf) {
            let data = &buf[..read];
            let msg = serde_json::from_slice::<FFBroadcast>(data)?;
            addr.set_port(msg.gdl90.port);

            let targets = &mut self.state.borrow_mut().socket.auto_targets;
            if !targets.contains_key(&addr) {
                info!("Adding {addr} to auto targets from {msg:?}");
            }
            targets.insert(addr, msg);
        }
        Ok(())
    }

    pub fn send_data(&self, data: &Data) -> anyhow::Result<()> {
        if let Err(why) = self.recv() {
            error!("recv failed: {why}");
        }

        self.state
            .borrow_mut()
            .socket
            .auto_targets
            .retain(|addr, ff| {
                if ff.time.elapsed() < Duration::from_secs(30) {
                    true
                } else {
                    info!("No broadcast from {addr} ({}) in 30s, removing", ff.app);
                    false
                }
            });

        if self
            .last_send
            .get()
            .is_some_and(|i| i.elapsed() < self.state.borrow().socket.interval)
        {
            return Ok(());
        }

        let targets = self
            .state
            .borrow()
            .socket
            .auto_targets
            .keys()
            .copied()
            .chain(self.state.borrow().socket.target)
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return Ok(());
        }

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

        let precise = CustomPreciseOwnship::new(data.lat, data.lon, data.alt, data.gs);

        let mut bytes = Vec::new();
        if self.state.borrow().heartbeat {
            bytes.extend_from_slice(&heartbeat.into_gdl90_bytes()?);
        }
        if self.state.borrow().ownship {
            bytes.extend_from_slice(&ownship.into_gdl90_bytes()?);
        }
        if self.state.borrow().ahrs {
            bytes.extend_from_slice(&ahrs.into_gdl90_bytes()?);
        }
        if self.state.borrow().precise {
            bytes.extend_from_slice(&precise.into_gdl90_bytes()?);
        }

        if !self.socket.broadcast()? {
            self.socket.set_broadcast(true)?;
            info!("Enabled broadcast on socket");
        }

        for target in targets {
            if let Err(why) = self.socket.send_to(&bytes, target) {
                error!("Failed to send data to {target}: {why}");
            }
        }
        self.last_send.replace(Some(Instant::now()));

        match self.socket.take_error() {
            Ok(Some(error)) => error!("UdpSocket error: {error:?}"),
            Ok(None) => (),
            Err(error) => error!("UdpSocket.take_error failed: {error:?}"),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct FFBroadcast {
    #[serde(skip, default = "Instant::now")]
    pub time: Instant,

    #[serde(rename = "App")]
    pub app: String,

    #[serde(rename = "GDL90")]
    pub gdl90: FFPort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub struct FFPort {
    pub port: u16,
}
