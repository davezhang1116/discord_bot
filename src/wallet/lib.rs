
use serde_json::json;
use serde_json::Value;
use std::time::Duration;
use crate::xml::reader::get_data;

fn get_url() -> String{
    let data = get_data();
    format!("http://{}:{}@{}:{}/", data.username, data.password, data.url, data.port)
}

pub async fn get_new_address() -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let map = json!({
        "method": "getnewaddress"
    });
    let req =client
        .post(get_url())
        .timeout(Duration::from_secs(2))
        .json(&map)
        .send()
        .await?
        .text()
        .await?;
    let data = serde_json::from_str::<Value>(&req).unwrap();
    let res = data["result"].clone();
    let addr = res.as_str().unwrap_or("address generation failed");
    Ok(addr.to_string())
}

pub async fn get_received_amount(address: String) -> Result<f64, reqwest::Error>{
    let client = reqwest::Client::new();
    let map = json!({
        "method": "listunspent",
        "params": [1, 10000, [address]]
    });
    let req =client
        .post(get_url())
        .timeout(Duration::from_secs(2))
        .json(&map)
        .send()
        .await?
        .text()
        .await?;
    let data = serde_json::from_str::<Value>(&req).unwrap();
    
    let r = data["result"][0]["amount"].clone();
    let amt = r.as_f64().unwrap_or(0.0);
    Ok(amt)
}

pub async fn send(address: String, amt: f64) -> Result<String, reqwest::Error>{
    let client = reqwest::Client::new();
    let map = json!({
        "method": "sendtoaddress",
        "params": [address, amt]
    });

    println!("{:?}",&map);
    let req =client
        .post(get_url())
        .timeout(Duration::from_secs(2))
        .json(&map)
        .send()
        .await?
        .text()
        .await?;
    let data = serde_json::from_str::<Value>(&req).unwrap();
    let res = data["result"].clone();
    let txid = res.as_str().unwrap_or("error! try again");
    Ok(txid.to_string())
}