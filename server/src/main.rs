// src/main.rs
#[macro_use]
extern crate log;

use actix_web::{get, web, post, App, HttpResponse, HttpServer, Responder, Result};
use dotenv::dotenv;
use listenfd::ListenFd;
use std::env;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
struct GasCalcObject {
    address: String,
    time_period: String
}

#[derive(Deserialize, Serialize, Debug)]
struct GasUsed {
    gas_used: u32
}

#[derive(Deserialize, Debug)]
struct TransactionList {
    status: String,
    message: String,
    result: Vec<HashMap<String, String>>
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
async fn get_list_of_transactions_by_address(address: String, startblock: u32, endblock: u32) -> Result<u32, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.etherscan.io/api?module=account&action=txlist&address={}&startblock={}&endblock={}&apikey={}",
        address,
        startblock,
        endblock,
        env::var("APIKEY").expect("API Key not set")
    );

    let response: TransactionList = reqwest::get(url)
        .await?
        .json()
        .await?;

    println!("Response: {:?}", response.result);

    let mut gas_count = 0;
    for txn in response.result {
        if txn["from"].to_uppercase() == address.to_uppercase() {
            let gas_used = txn["gasUsed"].to_string().parse::<u32>().unwrap();
            gas_count += gas_used;
        }
    }
    
    println!("Total Gas Used: {:?}", gas_count);
    Ok(gas_count)

}

#[post("/calculate")]
async fn calculate_gas_fees(request_body: web::Json<GasCalcObject>) -> HttpResponse {
    let dt = Utc::now();
    let timestamp = dt.timestamp();
    let time_range;
    let mut total_gas_used: u32 = 0;
    let end_block_number: u32 = get_block_number_from_timestamp(timestamp).unwrap().parse().unwrap();
    let start_block_number: u32;
    if request_body.time_period.as_str() != "AllTime" {
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
        
        start_block_number = get_block_number_from_timestamp(start_timestamp).unwrap().parse().unwrap();
        
    } else {
        start_block_number = 0;
    }
    total_gas_used = get_list_of_transactions_by_address(request_body.address.to_string(), start_block_number, end_block_number).unwrap();

    // I need to convert the gas_used to a number in Gwei or ethereum

    HttpResponse::Ok().json(GasUsed {
        gas_used: total_gas_used
    })
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