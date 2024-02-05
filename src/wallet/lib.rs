
use serde_json::json;
use serde_json::Value;

const URL : &str = "http://dave:password@localhost:44555/";

pub async fn get_new_address() -> String{
    let client = reqwest::Client::new();
    let map = json!({
        "method": "getnewaddress"
    });
    let req =client
        .post(URL)
        .json(&map)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let data = serde_json::from_str::<Value>(&req).unwrap();
    let res = data["result"].clone();
    let addr = res.as_str().unwrap_or("address generation failed");
    addr.to_string()
}

pub async fn get_received_amount(address: String) -> f64{
    let client = reqwest::Client::new();
    let map = json!({
        "method": "listunspent",
        "params": [1, 10000, [address]]
    });
    let req =client
        .post(URL)
        .json(&map)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let data = serde_json::from_str::<Value>(&req).unwrap();
    
    let r = data["result"][0]["amount"].clone();
    let amt = r.as_f64().unwrap_or(0.0);
    amt
}

pub async fn send(address: String, amt: f64) -> String{
    let client = reqwest::Client::new();
    let map = json!({
        "method": "sendtoaddress",
        "params": [address, amt]
    });

    println!("{:?}",&map);
    let req =client
        .post(URL)
        .json(&map)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let data = serde_json::from_str::<Value>(&req).unwrap();
    let res = data["result"].clone();
    let txid = res.as_str().unwrap_or("error! try again");
    txid.to_string()
}