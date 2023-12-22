use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use reqwest::Client as HttpClient;
use std::sync::Mutex;
use std::collections::HashMap;
use tokio::time::{Duration, sleep};
use reqwest::Response;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CryptoPair {
    id: String,
    price: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Database {
    crypto_pairs: Mutex<HashMap<String, CryptoPair>>,
}

impl Database {
    fn new() -> Self {
        Self {
            crypto_pairs: Mutex::new(HashMap::new()),
        }
    }

    async fn insert(&self, pair: CryptoPair) {
        let mut db = self.crypto_pairs.lock().unwrap();
        db.insert(pair.id.clone(), pair);
    }

    async fn get(&self, id: &str) -> Option<CryptoPair> {
        let db = self.crypto_pairs.lock().unwrap();
        db.get(id).cloned()
    }

    async fn get_all(&self) -> Vec<CryptoPair> {
        let db = self.crypto_pairs.lock().unwrap();
        db.values().cloned().collect()
    }
}

struct AppState {
    db: Database,
    http_client: HttpClient,
}

async fn get_crypto_price(app_state: web::Data<AppState>, id: web::Path<String>) -> impl Responder {
    match app_state.db.get(&id.into_inner()).await {
        Some(pair) => HttpResponse::Ok().json(pair),
        None => HttpResponse::NotFound().finish(),
    }
}

async fn get_all_crypto_prices(app_state: web::Data<AppState>) -> impl Responder {
    let pairs: Vec<CryptoPair> = app_state.db.get_all().await;
    HttpResponse::Ok().json(pairs)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Database::new();
    let http_client = HttpClient::new();
    let data: web::Data<AppState> = web::Data::new(AppState {
        db,
        http_client,
    });

    let data_clone = data.clone();
    tokio::spawn(async move {
        loop {
            let data_clonetwo = data_clone.clone();
            let tempres = data_clonetwo.http_client.get("https://api.exchangeratesapi.io/latest?base=USD").send().await;
            dbg!(&tempres);
            
            let res = data_clone.http_client.get("https://api.binance.com/api/v3/exchangeInfo").send().await;
            dbg!(&data_clone.http_client);
            //let rere: & Response = res.as_ref().unwrap();
            
            dbg!(res.unwrap().json().await);
 /*           if let Ok(res) = res {
                let body = res.json::<HashMap<String, Vec<CryptoPair>>>().await;
                dbg!(&body);
              if let Ok(body) = res.json::<HashMap<String, Vec<CryptoPair>>>().await {
                    for pair in body["symbols"].clone() {
                        dbg!(pair.clone());
                        data_clone.db.insert(pair).await;
                    }
                }

            } else {
               println!("ERROR OCCURED!");
               dbg!(res);
            }
*/
            sleep(Duration::from_secs(60)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/crypto/{id}", web::get().to(get_crypto_price))
            .route("/crypto", web::get().to(get_all_crypto_prices))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}