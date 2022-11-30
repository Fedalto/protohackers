use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock};

use tokio::sync::{mpsc, oneshot};

type RoadNumber = u16;
type Speed = u16;

pub struct Ticket {
    pub plate: Vec<u8>,
    pub road: u16,
    pub mile1: u16,
    pub timestamp1: u32,
    pub mile2: u16,
    pub timestamp2: u32,
    pub speed: u16,
}

impl Debug for Ticket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let formatted_plate = String::from_utf8_lossy(&self.plate);
        f.debug_struct("Ticket")
            .field("plate", &formatted_plate.as_ref())
            .field("road", &self.road)
            .field("mile1", &self.mile1)
            .field("mile2", &self.mile2)
            .field("timestamp1", &self.timestamp1)
            .field("timestamp2", &self.timestamp2)
            .field("speed", &self.speed)
            .finish()
    }
}

pub struct PlateObservation {
    plate: Vec<u8>,
    mile: u16,
    time: u32,
}

impl PlateObservation {
    pub fn new(plate: Vec<u8>, mile: u16, time: u32) -> Self {
        Self { plate, mile, time }
    }
}

impl Debug for PlateObservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let formatted_plate = String::from_utf8_lossy(&self.plate);
        f.debug_struct("Plate")
            .field("plate", &formatted_plate.as_ref())
            .field("mile", &self.mile)
            .field("time", &self.time)
            .finish()
    }
}

#[derive(Clone)]
pub struct IslandMap {
    roads: Arc<RwLock<HashMap<RoadNumber, mpsc::Sender<PlateObservation>>>>,
    pub ticket_processor: mpsc::Sender<ProcessorCommand>,
}

impl IslandMap {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(16);
        tokio::spawn(ticket_processor(rx));
        Self {
            roads: Arc::new(RwLock::new(HashMap::new())),
            ticket_processor: tx,
        }
    }

    #[instrument(skip(self))]
    pub fn get_or_create_road(
        &self,
        road_number: RoadNumber,
        limit: Speed,
    ) -> mpsc::Sender<PlateObservation> {
        {
            let roads = self.roads.read().unwrap();
            if roads.contains_key(&road_number) {
                return roads[&road_number].clone();
            }
        }
        let mut roads = self.roads.write().unwrap();
        let (tx, rx) = mpsc::channel(16);
        roads.insert(road_number, tx.clone());

        tokio::spawn(check_road_plates(
            road_number,
            limit,
            rx,
            self.ticket_processor.clone(),
        ));

        tx
    }
}

#[derive(Debug)]
pub enum ProcessorCommand {
    Ticket(Ticket),
    NewDispatcher {
        roads: Vec<RoadNumber>,
        ch: oneshot::Sender<Vec<async_channel::Receiver<Ticket>>>,
    },
}

/// Process tickets before sending to Dispatchers
///
/// Because tickets are emitted independently on each road, instead of sending directly to the
/// Dispatchers, all tickets are sent to this processor first so it can discard tickets
/// if a car plate has already received one in that day.
#[instrument(skip_all)]
async fn ticket_processor(mut rx: mpsc::Receiver<ProcessorCommand>) {
    let mut ticket_days = HashMap::new(); // Plate number -> days
    let mut tickets_by_road = HashMap::new(); // Road Number -> Tickets channel

    while let Some(message) = rx.recv().await {
        match message {
            ProcessorCommand::Ticket(ticket) => {
                debug!("New ticket to process. {:?}", ticket);
                let ticket_day1 = ticket.timestamp1 / 86400;
                let ticket_day2 = ticket.timestamp2 / 86400;
                let days = ticket_days
                    .entry(ticket.plate.clone())
                    .or_insert_with(HashSet::new);
                if !days.contains(&ticket_day1) && !days.contains(&ticket_day2) {
                    days.insert(ticket_day1);
                    days.insert(ticket_day2);
                    let channel = tickets_by_road
                        .entry(ticket.road)
                        .or_insert_with(|| async_channel::bounded(16));
                    // Channel is never closed because receiver is also stored in `channel`
                    debug!(
                        "Sending ticket to Dispatcher. ticket={:?}, days={},{}",
                        ticket, ticket_day1, ticket_day2
                    );
                    channel.0.send(ticket).await.unwrap();
                }
            }
            ProcessorCommand::NewDispatcher { roads, ch } => {
                let mut ticket_channels = Vec::new();
                for road in roads {
                    let channel = tickets_by_road
                        .entry(road)
                        .or_insert_with(|| async_channel::bounded(16));
                    ticket_channels.push(channel.1.clone());
                }
                ch.send(ticket_channels).unwrap();
            }
        }
    }
}

/// Check for plates for all cameras in a road and emit tickets if above the speed limit
// One tokio task is created per road, and all cameras on that road will send all seen plates to
// the channel in `rx`.
#[instrument(skip(rx, ticket_processor))]
async fn check_road_plates(
    road_number: RoadNumber,
    road_limit: Speed,
    mut rx: mpsc::Receiver<PlateObservation>,
    ticket_processor: mpsc::Sender<ProcessorCommand>,
) {
    // Map of "Plate number" -> Vec<(mile, timestamp)>
    let mut seen_plates = HashMap::<Vec<u8>, Vec<(u16, u32)>>::new();

    while let Some(plate) = rx.recv().await {
        debug!("Seen new plate. road={}, plate={:?}", road_number, plate);
        let other_observations = seen_plates.entry(plate.plate.clone()).or_insert(Vec::new());
        for (mile, timestamp) in other_observations.iter() {
            let mut obs = [(*mile, *timestamp), (plate.mile, plate.time)];
            obs.sort_by_key(|x| x.1);
            let (earlier_obs, later_obs) = (obs[0], obs[1]);

            let miles = mile.abs_diff(plate.mile);
            let time = later_obs.1 - earlier_obs.1;
            let speed = (miles as u32 * 3600 * 100 / time) as u16;

            if speed > road_limit * 100 {
                let ticket = Ticket {
                    plate: plate.plate.clone(),
                    road: road_number,
                    mile1: earlier_obs.0,
                    timestamp1: earlier_obs.1,
                    mile2: later_obs.0,
                    timestamp2: later_obs.1,
                    speed,
                };
                info!(
                    "Above road limit. Preparing new ticket. limit={}, ticket={:?}",
                    road_limit, ticket
                );
                ticket_processor
                    .send(ProcessorCommand::Ticket(ticket))
                    .await
                    .unwrap();
            }
        }
        other_observations.push((plate.mile, plate.time));
    }
}
