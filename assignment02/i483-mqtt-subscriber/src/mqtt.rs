use std::time::Duration;
use rumqttc::v5::{AsyncClient, Event, EventLoop, MqttOptions};
use rumqttc::v5::mqttbytes::{QoS};

pub async fn connect(client_id: &str, host: &str) -> Result<(AsyncClient, EventLoop), rumqttc::ConnectionError> {
    let mut mqtt_options = MqttOptions::new(client_id, host, 1883);
    mqtt_options.set_keep_alive(Duration::from_secs(30));
    let (client, event_loop) = AsyncClient::new(mqtt_options, 100);
    Ok((client, event_loop))
}

pub async fn subscribe(client: &AsyncClient, topics: &Vec<String>) -> Result<(), rumqttc::v5::ClientError> {
    for topic in topics {
        client.subscribe(topic.clone(), QoS::AtMostOnce).await?;
    }
    Ok(())
}

pub async fn listen(event_loop: &mut EventLoop) -> Result<(), rumqttc::ConnectionError> {
    loop {
        let event = event_loop.poll().await;
        match &event {
            Ok(v) => {
                println!("Event = {v:?}");
            }
            Err(e) => {
                println!("Error = {e:?}");
                return Ok(());
            }
        }
    }
}
