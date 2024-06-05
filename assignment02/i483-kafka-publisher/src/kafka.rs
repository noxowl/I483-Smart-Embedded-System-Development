use futures::{StreamExt, TryStreamExt};
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{BaseConsumer, CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message};


pub fn create_consumer_config(group_id: &str, client_id: &str) -> ClientConfig {
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

pub async fn listen(host: &str, topics: &Vec<String>,) -> Result<(), rdkafka::error::KafkaError> {
    let mut config = create_consumer_config("kafka-cli", "kafka-cli");
    let consumer: StreamConsumer = config.set("bootstrap.servers", host).create().unwrap();
    consumer.subscribe(topics.iter().map(|s| s.as_str()).collect()).unwrap();
    loop {
        match consumer.stream().try_for_each(|borrowed_message| {
            async move {
                borrowed_message.offset();
                let owned_message = borrowed_message.detach();
                tokio::spawn(async move {
                    println!("Received message from topic: {}, partition: {}, offset: {}, timestamp: {:?}, key: {:?}, payload: {:?}, headers: {:?}",
                        owned_message.topic(),
                        owned_message.partition(),
                        owned_message.offset(),
                        owned_message.timestamp(),
                        owned_message.key(),
                        owned_message.payload(),
                        owned_message.headers(),
                    );
                });
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

// pub async fn process(host: &str, topics: &Vec<String>, processes: &Vec<Process>) {
//     let mut client = connect("kafka-cli", host).await.unwrap();
//     subscribe(&client.0, topics).await.unwrap();
//     let mut event_loop = client.1;
//     listen(&mut event_loop).await.unwrap();
//     for process in processes {
//         process.run().await;
//     }
// }
