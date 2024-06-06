use std::collections::HashMap;
use std::str::FromStr;
use chrono::{DateTime, Utc, Duration, TimeDelta, NaiveDateTime};
use futures::{StreamExt, TryStreamExt};
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message, OwnedMessage};
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use rdkafka::util::Timeout;
use uuid::Uuid;
use crate::cli::ProcessType;

use crate::worker::{ActorMessage, ComputeActor, create_actor, ProcessData};


fn create_consumer_config(group_id: &str, client_id: &str) -> ClientConfig {
    let mut client_config = ClientConfig::new();
    client_config
        .set("group.id", group_id)
        .set("client.id", client_id)
        .set("enable.auto.commit", "false")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .set_log_level(RDKafkaLogLevel::Debug);
    client_config
}

fn parse_kafka_payload(payload: Option<&[u8]>) -> f32 {
    return match payload {
        None => 0.0,
        Some(payload) => f32::from_str(&String::from_utf8_lossy(payload).to_string()).unwrap_or(0.0),
    }
}

fn debug_kafka_message(message: &OwnedMessage) {
    println!("Received message from topic: {}, partition: {}, offset: {}, timestamp: {:?}, key: {:?}, payload: {:?}, headers: {:?}",
        message.topic(),
        message.partition(),
        message.offset(),
        message.timestamp(),
        message.key(),
        message.payload(),
        message.headers(),
    );
}

pub async fn listen(host: &str, topics: &Vec<String>, debug: &bool) -> Result<(), rdkafka::error::KafkaError> {
    let mut config = create_consumer_config("kafka-cli", "kafka-cli");
    let consumer: StreamConsumer = config.set("bootstrap.servers", host).create().unwrap();
    let topics_for_consume: Vec<&str> = topics.iter().map(AsRef::as_ref).collect();
    consumer.subscribe(&topics_for_consume).unwrap();
    loop {
        match consumer.stream().try_for_each(|borrowed_message| {
            let debug = debug.clone();
            async move {
                borrowed_message.offset();
                let owned_message = borrowed_message.detach();
                tokio::spawn(async move {
                    let data_timestamp = DateTime::from_timestamp_millis(owned_message.timestamp().to_millis().unwrap()).unwrap();
                    let acceptable_timestamp = Utc::now() - Duration::seconds(3);
                    if data_timestamp < acceptable_timestamp {
                        println!("Message is too old, skipping");
                        return;
                    }
                    if debug {
                        debug_kafka_message(&owned_message);
                    }
                    println!("Received message from topic: {}, value: {}", owned_message.topic(), parse_kafka_payload(owned_message.payload()));
                });
                Ok(())
            }
        }).await {
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
            Ok(_) => {
                break;
            }
        }
    }
    Ok(())
}

fn pairing_topics_and_processes(topics: &Vec<String>, processes: &Vec<ProcessType>) -> HashMap<String, ProcessType> {
    let mut map = HashMap::new();
    for (i, topic) in topics.iter().enumerate() {
        map.insert(topic.clone(), processes.get(i).unwrap().clone());
    }
    return map;
}

pub async fn process(host: &str, topics: &Vec<String>, processes: &Vec<ProcessType>, debug: &bool, dry_run: &bool) {
    let mut config = create_consumer_config("kafka-cli", "kafka-cli");
    let consumer: StreamConsumer = config.set("bootstrap.servers", host).create().unwrap();
    let producer: FutureProducer = ClientConfig::new().set("bootstrap.servers", host).create().unwrap();
    let topics_for_consume: Vec<&str> = topics.iter().map(AsRef::as_ref).collect();
    let topics_and_processes = pairing_topics_and_processes(topics, processes);
    let (tx, mut rx) = mpsc::channel::<ActorMessage>(100);
    let mut actors = Arc::new(Mutex::new(HashMap::new()));
    let mut last_rolling_average_actor_crated = Utc::now();
    consumer.subscribe(&topics_for_consume).unwrap();
    for (t, p) in &topics_and_processes {
        match p {
            ProcessType::Threshold(value) => {
                let (actor_id, sender) = create_actor(0, ProcessType::Threshold(value.clone()), &tx);
                let actors = actors.clone();
                tokio::spawn(async move {
                    let mut lock = actors.lock().await;
                    lock.insert(actor_id, sender);
                });
            }
            ProcessType::RollingAverage(value) => {
                let (actor_id, sender) = create_actor(30, ProcessType::RollingAverage(value.clone()), &tx);
                let actors = actors.clone();
                tokio::spawn(async move {
                    let mut lock = actors.lock().await;
                    lock.insert(actor_id, sender);
                });
            }
        }
    }
    let receiver_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let copied_actors = actors.clone();
    let copied_tap = topics_and_processes.clone();
    let debug = debug.clone();
    let dry_run = dry_run.clone();
    let future_producer = producer.clone();
    receiver_runtime.spawn(async move {
        // https://stackoverflow.com/questions/77494743/tokio-mpsc-closes-channel-when-sender-assigned-to-static
        loop {
            if let Some(actor_message) = rx.recv().await {
            match actor_message {
                ActorMessage::Finished(uuid, data) => {
                    println!("Actor finished processing data: {:?}, from: {}", data, &uuid);
                    let mut lock = copied_actors.lock().await;
                    // lock.remove(&uuid);
                    for (t, p) in &copied_tap {
                        let (topic, payload) = generate_payload(p.clone(), data.clone(), t, debug);
                        if payload.is_empty() {
                            continue;
                        }
                        if !dry_run {
                            produce(future_producer.clone(), &topic, &payload).await.unwrap();
                        }
                    }
                },
                ActorMessage::Updated(uuid, data) => {
                    println!("Actor updated data: {:?}, from: {}", data, uuid);
                    for (t, p) in &copied_tap {
                        let (topic, payload) = generate_payload(p.clone(), data.clone(), t, debug);
                        if payload.is_empty() {
                            continue;
                        }
                        if !dry_run {
                            produce(future_producer.clone(), &topic, &payload).await.unwrap();
                        }
                    }
                },
                _ => {}
                }
            }
        }
    });

    // let copied_actors = actors.clone();
    // let copied_tap = topics_and_processes.clone();
    let copied_tap2 = topics_and_processes.clone();
    loop {
        // tokio::spawn(async move {
        //     let tap = copied_tap.clone();
        //     let mut lock = copied_actors.lock().await;
        //     if lock.len() == 1 || Utc::now() - last_rolling_average_actor_crated > Duration::seconds(30) {
        //         println!("Creating new rolling average actor");
        //         for (t, p) in &tap {
        //             match p {
        //                 ProcessType::RollingAverage(value) => {
        //                     let (actor_id, sender) = create_actor(value.clone(), ProcessType::RollingAverage(value.clone()), &tx);
        //                     last_rolling_average_actor_crated = Utc::now();
        //                     lock.insert(actor_id, sender);
        //                 }
        //                 _ => {}
        //             }
        //         }
        //     }
        // });

        match consumer.stream().try_for_each(|borrowed_message| {
            let copied_actors = actors.clone();
            let debug = debug.clone();
            let tap = copied_tap2.clone();
            async move {
                borrowed_message.offset();
                let owned_message = borrowed_message.detach();
                tokio::spawn(async move {
                    let data_timestamp = DateTime::from_timestamp_millis(owned_message.timestamp().to_millis().unwrap()).unwrap();
                    let acceptable_timestamp = Utc::now() - Duration::seconds(3);
                    if data_timestamp < acceptable_timestamp {
                        println!("Message is too old, skipping");
                        return;
                    }
                    if debug {
                        debug_kafka_message(&owned_message);
                    }
                    let payload = parse_kafka_payload(owned_message.payload());
                    let process = tap.get(owned_message.topic()).unwrap();
                    let mut actors_lock = copied_actors.lock().await;
                    for (actor_id, sender) in actors_lock.iter() {
                        match process {
                            ProcessType::RollingAverage(_) => {
                                sender.send(ActorMessage::FeedData(Uuid::default(), ProcessData::RollingAverage(payload))).await.unwrap();
                            }
                            ProcessType::Threshold(_) => {
                                sender.send(ActorMessage::FeedData(Uuid::default(), ProcessData::Threshold(payload))).await.unwrap();
                            }
                        }
                    }
                });
                Ok(())
            }
        }).await {
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
            Ok(_) => {
                break;
            }
        }
    }
}


fn generate_payload(process: ProcessType, data: ProcessData, t: &str, debug: bool) -> (String, String) {
    let (topic, payload) = match process {
        ProcessType::RollingAverage(_) => {
            match data {
                ProcessData::RollingAverage(value) => {
                    if debug {
                        (t.replace("-temperature", "_avg-temperature-debug"), value.to_string())
                    } else {
                        (t.replace("-temperature", "_avg-temperature"), value.to_string())
                    }
                },
                _ => {
                    (t.replace("-temperature", "_avg-temperature"), "".to_string())
                },
            }
        }
        ProcessType::Threshold(_) => {
            match data {
                ProcessData::Threshold(value) => {
                    let mut  threshold_result = "no";
                    if value > 0.0 {
                        threshold_result = "yes";
                    }
                    if debug {
                        (format!("{}_threshold-crossed-debug", t), threshold_result.to_string())
                    } else {
                        (format!("{}_threshold-crossed", t), threshold_result.to_string())
                    }
                },
                _ => {
                    (format!("{}_threshold-crossed", t), "".to_string())
                },
            }
        }
    };
    (topic, payload)
}


async fn produce(future_producer: FutureProducer, topic: &str, payload: &String) -> Result<(), rdkafka::error::KafkaError> {
    println!("Producing message to topic: {}, payload: {}", &topic, &payload);
    let record: FutureRecord<String, String> = FutureRecord::to(topic).payload(payload);
    let produce_future = future_producer.send(
        record,
        Timeout::Never
    );
    match produce_future.await {
        Ok(delivered) => {
            println!("Produced message to topic: {}, {:?}", topic, delivered);
            Ok(())
        }
        Err((e, msg)) => {
            println!("Error producing message to topic: {:?}", e);
            Err(e)
        }
    }
}
