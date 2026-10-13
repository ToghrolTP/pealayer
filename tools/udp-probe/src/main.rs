#[allow(dead_code)]
#[path="../../../src/four_d/protocol.rs"]
pub mod protocol;
mod four_d {pub use crate::protocol;}
#[allow(dead_code)]
#[path="../../../src/server/udp_codec.rs"]
mod udp_codec;
#[allow(dead_code)]
#[path="../../../src/server/udp_client.rs"]
mod udp_client;
use serde_json::json;
use std::time::Duration;
fn main()->Result<(),String>{
    if std::env::args().nth(1).as_deref()==Some("--codec") {let value=json!({"signed":i64::MIN,"unsigned":u64::MAX,"float":0.125,"unicode":"همتا","array":[null,false,true]});println!("{}",udp_codec::encode(&value)?.iter().map(|b|format!("{b:02x}")).collect::<String>());return Ok(());}
    // Fixed loopback fixture, not production discovery or a physical controller.
    let mut c=udp_client::Client::connect("127.0.0.1:18877",Duration::from_secs(3))?;
    let ping=c.call(&json!({"jsonrpc":"2.0","id":1,"method":"controller.ping","params":{}}),None)?;
    if ping.get("error").is_some(){return Err(ping.to_string());}
    let snapshot=c.call(&json!({"jsonrpc":"2.0","id":2,"method":"controller.snapshot","params":{}}),None)?;
    if snapshot["result"]["connected"]!=false{return Err("Probe must not connect any board".into());}
    let config=c.call(&json!({"jsonrpc":"2.0","id":3,"method":"controller.ui.config.get","params":{}}),None)?;
    if config.get("error").is_some(){return Err(config.to_string());}
    c.subscribe()?;
    let event=c.event(Duration::from_secs(3))?.ok_or("No pushed event")?;
    if event["method"]!="controller.event"{return Err(event.to_string());}
    let numbers=json!({"signed":i64::MIN,"unsigned":u64::MAX,"float":0.125,"unicode":"همتا","array":[null,false,true]});
    let bytes=udp_codec::encode(&numbers)?;if udp_codec::decode(&bytes)?!=numbers{return Err("Lossy binary codec".into());}
    println!("{}",json!({"query":"ok","catalog_binary_bytes":udp_codec::encode(&config)?.len(),"pushed_event":event["params"]["kind"],"exact_integers_unicode":"ok","physical_board":false}));Ok(())
}
