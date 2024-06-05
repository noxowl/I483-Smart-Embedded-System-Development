/*
    This is the worker module. It contains the worker struct and its implementation.
    The worker struct is a base actor struct that contains the id and age of the worker.

    The worker struct is used to create a worker instance that can be used to perform
    * Calculate the rolling average of a given age.
    * Calculate the threshold of a given value.
    * Returns a message to the caller when the task given to the worker is complete.
*/
use std::{fmt, thread};
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::{channel, Sender, Receiver};
use std::time::Duration;
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;
use crate::cli::ProcessType;


#[derive(Debug, Clone)]
pub enum ProcessData {
    RollingAverage(f32),
    Threshold(f32),
}

impl Display for ProcessData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProcessData::RollingAverage(value) => write!(f, "Rolling average: {}", value),
            ProcessData::Threshold(value) => write!(f, "Threshold: {}", value),
        }
    }
}

pub enum ActorMessage {
    FeedData(Uuid, ProcessData),
    Updated(Uuid, ProcessData),
    Finished(Uuid, ProcessData),
    AddActor(Uuid, Sender<ActorMessage>),
    RemoveActor(Uuid),
    GetActors(),
    Actors(HashMap<Uuid, Sender<ActorMessage>>),
}

trait Actor {
    fn send_message(&self, message: ActorMessage);
    fn receive_message(&mut self);
    fn compute(&mut self, data: ProcessData);
    fn fetch(&self) -> f32;
    fn send_last_will(&mut self);
}

pub(crate) struct ManagerActor {
    id: Uuid,
    sender: Sender<ActorMessage>,
    receiver: Receiver<ActorMessage>,
    actors: HashMap<Uuid, Sender<ActorMessage>>,
}

impl ManagerActor {
    pub fn new(id: Uuid, sender: Sender<ActorMessage>, receiver: Receiver<ActorMessage>) -> ManagerActor {
        ManagerActor {
            id,
            sender,
            receiver,
            actors: HashMap::new(),
        }
    }

    pub fn add_actor(&mut self, id: Uuid, sender: Sender<ActorMessage>) {
        self.actors.insert(id, sender);
    }

    pub fn remove_actor(&mut self, id: Uuid) {
        self.actors.remove(&id);
    }

    pub fn get_actors(&self) -> HashMap<Uuid, Sender<ActorMessage>> {
        self.actors.clone()
    }
}

impl Actor for ManagerActor {
    fn send_message(&self, message: ActorMessage) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            match self.sender.send(message).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Error sending message: {:?}", e);
                }
            }
        });
    }

    fn receive_message(&mut self) {
        println!("Manager actor {} is alive", self.id);
        loop {
            match self.receiver.blocking_recv() {
                Some(message) => {
                    match message {
                        ActorMessage::AddActor(id, sender) => {
                            self.add_actor(id, sender);
                        },
                        ActorMessage::RemoveActor(id) => {
                            self.remove_actor(id);
                        },
                        ActorMessage::GetActors() => {
                            let actors = self.get_actors();
                            println!("Actors: {:?}", &actors);
                            self.send_message(ActorMessage::Actors(actors))
                        },
                        _ => {},
                    }
                },
                None => {}
            }
        }
    }

    fn compute(&mut self, data: ProcessData) {
        println!("Manager actor {} received data: {}", self.id, &data);
    }

    fn fetch(&self) -> f32 {
        0.0
    }

    fn send_last_will(&mut self) {
        println!("Manager actor {} is dead", self.id);
    }
}


pub(crate) struct ComputeActor {
    id: Uuid,
    dead_at: DateTime<Utc>,
    lifespan: u64,
    is_immortal: bool,
    process_type: ProcessType,
    sender: Sender<ActorMessage>,
    receiver: Receiver<ActorMessage>,
    counter: u64,
    result: ProcessData,
    rolling_stack: VecDeque<f32>,
    rolling_counter: VecDeque<u64> // I know it's too ugly, but I'm running out of time
}

impl ComputeActor {
    fn new(id: Uuid, lifespan: u64, process_type: ProcessType, sender: Sender<ActorMessage>, receiver: Receiver<ActorMessage>) -> ComputeActor {
        let mut dead_at = Utc.with_ymd_and_hms(9999, 12, 31, 23, 59, 59).unwrap();
        match process_type {
            ProcessType::RollingAverage(value) => {
                dead_at = Utc::now() + Duration::from_secs(value);
                if lifespan == 0 {
                    panic!("Rolling average process must have a lifespan");
                }
            },
            _ => {},
        }
        let immortal = lifespan == 0;
        let result = match process_type {
            ProcessType::RollingAverage(value) => ProcessData::RollingAverage(0.0),
            ProcessType::Threshold(value) => ProcessData::Threshold(0.0),
        };
        let mut rolling_stack = VecDeque::new();
        rolling_stack.push_back(0.0);
        let mut rolling_counter = VecDeque::new();
        rolling_counter.push_back(0);
        ComputeActor {
            id,
            dead_at,
            lifespan,
            is_immortal: true, // multithreading complex issue (lack of time!!)
            process_type,
            sender,
            receiver,
            counter: 0,
            result,
            rolling_stack,
            rolling_counter
        }
    }
}

impl Actor for ComputeActor {
    fn send_message(&self, message: ActorMessage) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            match self.sender.send(message).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Error sending message: {:?}", e);
                }
            }
        });
    }

    fn receive_message(&mut self) {
        println!("Actor {} is alive. process {:?}", self.id, self.process_type);
        let mut next_rolling_average_start = Utc::now() + Duration::from_secs(self.lifespan);
        while self.is_immortal {
            match self.receiver.blocking_recv() {
                Some(message) => {
                    match message {
                        ActorMessage::FeedData(_, data) => {
                            self.compute(data);
                        },
                        _ => {},
                    }
                },
                None => {}
            }
            if next_rolling_average_start < Utc::now() {
                println!("Actor {} is resetting rolling average", self.id);
                next_rolling_average_start = Utc::now() + Duration::from_secs(self.lifespan);
                self.rolling_stack.push_back(0.0);
                self.rolling_counter.push_back(0);
            }
            if self.dead_at < Utc::now() {
                if self.rolling_stack.len() > 1 {
                    self.send_last_will();
                }
            }
        }
    }

    fn compute(&mut self, data: ProcessData) {
        println!("Actor {} received data: {}", self.id, &data);
        match data {
            ProcessData::RollingAverage(value) => {
                match self.process_type {
                    ProcessType::RollingAverage(_) => {
                        println!("Actor {} is computing rolling average", self.id);
                        for i in 0..self.rolling_stack.len() {
                            self.rolling_counter[i] += 1;
                            let previous = self.rolling_stack[i];
                            let count = self.rolling_counter[i];
                            if previous == 0.0 {
                                self.rolling_stack[i] = value;
                            } else {
                                let new_avg = previous * (count - 1) as f32 / count as f32 + value / count as f32;
                                self.rolling_stack[i] = new_avg;
                            }
                            println!("Actor {} rolling average {}: {}/{}", self.id, i, self.rolling_stack[i], count);
                        }
                    },
                    _ => {},
                }
            },
            ProcessData::Threshold(value) => {
                match self.process_type {
                    ProcessType::Threshold(baseline) => {
                        println!("Actor {} is computing threshold", self.id);
                        if value >= baseline as f32 {
                            if self.counter == 0 {
                                self.counter = 1;
                                self.result = ProcessData::Threshold(value);
                                self.send_message(ActorMessage::Updated(self.id, ProcessData::Threshold(value)));
                            }
                        } else {
                            if self.counter == 1 {
                                self.counter = 0;
                                self.result = ProcessData::Threshold(0.0);
                                self.send_message(ActorMessage::Updated(self.id, ProcessData::Threshold(0.0)));
                            }
                        }
                    },
                    _ => {},
                }
            },
        }
    }

    fn fetch(&self) -> f32 {
        match self.result {
            ProcessData::RollingAverage(value) => {
                value
            },
            ProcessData::Threshold(value) => {
                value
            },
        }
    }

    fn send_last_will(&mut self) {
        println!("Actor {} is send last will. process {:?}", self.id, self.process_type);
        match self.result {
            ProcessData::RollingAverage(_) => {
                let value = self.rolling_stack.pop_front().unwrap();
                let _ = self.rolling_counter.pop_front().unwrap();
                self.send_message(ActorMessage::Finished(self.id, ProcessData::RollingAverage(value)));
            },
            ProcessData::Threshold(value) => {
                self.send_message(ActorMessage::Finished(self.id, ProcessData::Threshold(value)));
            },
        }
    }
}

pub fn create_actor(lifespan: u64, process_type: ProcessType, main_sender: &Sender<ActorMessage>) -> (Uuid, Sender<ActorMessage>) {
    let (sender, receiver) = channel(100);
    let id = Uuid::new_v4();
    let mut actor = ComputeActor::new(id, lifespan, process_type, main_sender.clone(), receiver);
    thread::spawn(move || {
        actor.receive_message();
    });
    (id, sender)
}
