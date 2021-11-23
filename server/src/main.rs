// src/main.rs
#[macro_use]
extern crate log;

use actix_web::{get, web, post, App, HttpResponse, HttpServer, Responder, Result};
use dotenv::dotenv;
use listenfd::ListenFd;
use std::env;
use serde_json::json;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
struct GasCalcObject {
    address: String,
    time_period: String
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[tokio::main]
async fn get_block_number_from_timestamp(timestamp: i64) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://api.etherscan.io/api?module=block&action=getblocknobytime&timestamp={}&closest=before&apikey={}", timestamp, env::var("APIKEY").expect("API Key not set"));
    let resp = reqwest::get(url)
        .await?
        .json::<HashMap<String,String>>()
        .await?;
    println!("{:?}", resp);
    let block_number = resp["result"].to_string();
    Ok(block_number)
}

#[tokio::main]
async fn get_list_of_transactions_by_address(address: String, startblock: u32, endblock: u32) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.etherscan.io/api?module=account&action=txlist&address={}&startblock={}&endblock={}&apikey={}",
        address,
        startblock,
        endblock,
        env::var("APIKEY").expect("API Key not set")
    );

    let response = reqwest::get(url)
        .await?
        .json::<HashMap<String,String>>()
        .await?;

    Ok(())

}

#[post("/calculate")]
async fn calculate_gas_fees(request_body: web::Json<GasCalcObject>) -> Result<String> {
    let dt = Utc::now();
    let timestamp = dt.timestamp();
    let time_range;

    if(request_body.time_period.as_str() != "AllTime") {
        match request_body.time_period.as_str() {
            "Last24Hours" => time_range = 86400,
            "Last7Days" => time_range = 604800,
            "Last30Days" => time_range = 2592000,
            "Last3Months" => time_range = 7776000,
            "Last6Months" => time_range = 15552000,
            "Last12Months" => time_range = 31536000,
            _ => time_range = 0
        }
        
        let start_timestamp = timestamp - time_range;
        println!("{:?}", start_timestamp);
        let end_block_number: u32 = get_block_number_from_timestamp(timestamp).unwrap().parse().unwrap();
        let start_block_number: u32 = get_block_number_from_timestamp(start_timestamp).unwrap().parse().unwrap();
        println!("End block: {:?}", end_block_number);
        println!("Start block: {:?}", start_block_number);
    } else {
        let start_block_number = 0;
    }
    


    Ok(format!("Address: {}, Time Period: {}, current_time: {}", request_body.address, request_body.time_period, timestamp))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
    cfg.service(calculate_gas_fees);
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(||
        App::new()
            .configure(init_routes)
    );

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("Host not set");
            let port = env::var("PORT").expect("Port not set");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await
}