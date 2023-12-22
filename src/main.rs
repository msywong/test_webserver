use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use reqwest::Client as HttpClient;
use async_trait::async_trait;
use tokio::time::{self, Duration};
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CryptoPair {
    name: String,
    price: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    crypto_pairs: HashMap<String, CryptoPair>,
}

impl Database {
    fn new() -> Self {
        Self {
            crypto_pairs: HashMap::new(),
        }
    }

    fn insert(&mut self, pair: CryptoPair) {
        self.crypto_pairs.insert(pair.name.clone(), pair);
    }

    fn get(&self, name: &str) -> Option<&CryptoPair> {
        self.crypto_pairs.get(name)
    }

    fn get_all(&self) -> Vec<&CryptoPair> {
        self.crypto_pairs.values().collect()
    }
}

struct AppState {
    db: Mutex<Database>,
}

#[async_trait]
trait FetchPrice {
    async fn fetch_price(&self, pair: &str) -> Result<f64, reqwest::Error>;
}

struct CryptoPriceFetcher {
    client: HttpClient,
}

#[async_trait]
impl FetchPrice for CryptoPriceFetcher {
    async fn fetch_price(&self, pair: &str) -> Result<f64, reqwest::Error> {
        let url = format!("https://api.binance.com/api/v3/ticker/price?symbol={}", pair);
        let resp = self.client.get(&url).send().await?.json::<HashMap<String, String>>().await?;
        dbg!(&resp);
        Ok(resp.get("price").unwrap().parse().unwrap())
    }
}

async fn fetch_and_update_prices(app_state: web::Data<AppState>, fetcher: web::Data<CryptoPriceFetcher>) {
    let pairs = vec!["BTCUSDT", "ETHUSDT", "LTCUSDT"];
    loop {
        for pair in &pairs {
            match fetcher.fetch_price(pair).await {
                Ok(price) => {
                    let mut db = app_state.db.lock().unwrap();
                    db.insert(CryptoPair {
                        name: pair.to_string(),
                        price,
                    });
                }
                Err(e) => eprintln!("Error fetching price for {}: {}", pair, e),
            }
        }
        time::sleep(Duration::from_secs(300)).await;
    }
}

async fn get_price(app_state: web::Data<AppState>, pair: web::Path<String>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    match db.get(&pair.into_inner()) {
        Some(crypto_pair) => HttpResponse::Ok().json(crypto_pair),
        None => HttpResponse::NotFound().finish(),
    }
}

async fn get_all_prices(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    let pairs: Vec<&CryptoPair> = db.get_all();
    HttpResponse::Ok().json(pairs)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Database::new();
    let fetcher = CryptoPriceFetcher {
        client: HttpClient::new(),
    };

    let data: web::Data<AppState> = web::Data::new(AppState {
        db: Mutex::new(db),
    });

    let fetcher_data: web::Data<CryptoPriceFetcher> = web::Data::new(fetcher);

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            .app_data(data.clone())
            .app_data(fetcher_data.clone())
            .route("/price/{pair}", web::get().to(get_price))
            .route("/price", web::get().to(get_all_prices))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}