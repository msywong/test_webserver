use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FitnessProgress {
    id: u64,
    name: String,
    progress: String,
    timezone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    password: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    fitness_progress: HashMap<u64, FitnessProgress>,
    users: HashMap<u64, User>
}

impl Database {
    fn new () -> Self {
        Self {
            fitness_progress: HashMap::new(),
            users: HashMap::new()
        }
    }

    fn insert (&mut self, progress: FitnessProgress) {
        self.fitness_progress.insert(progress.id, progress);
    }

    fn get (&self, id: &u64)-> Option<&FitnessProgress> {
        self.fitness_progress.get (id)
    }

    fn get_all (&self)-> Vec<&FitnessProgress> {
        self.fitness_progress.values().collect()
    }

    fn delete (&mut self, id: &u64) {
        self.fitness_progress.remove(id);
    }

    fn update (&mut self, progress: FitnessProgress) {
        self.fitness_progress.insert (progress.id, progress);
    }

    fn insert_user (&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_user_by_name (&self, username: &str) -> Option<&User> {
        self.users.values().find(|u| u.username == username)
    }

    fn save_to_file(&self) -> std::io::Result<()>{
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create ("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file () -> std::io::Result<Self>{
        let file_content = fs::read_to_string("database.json")?;
        let db: Database = serde_json::from_str(&file_content)?;
        Ok(db)
    }
}

struct AppState {
    db: Mutex<Database>
}

async fn create_progress(app_state: web::Data<AppState>, progress: web::Json<FitnessProgress>) -> impl Responder{
    let mut db = app_state.db.lock().unwrap();
    db.insert(progress.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_progress(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let db = app_state.db.lock().unwrap();
    match db.get(&id.into_inner()) {
        Some(progress) => HttpResponse::Ok().json(progress),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_all_progress(app_state: web::Data<AppState>) -> impl Responder {
    let db = app_state.db.lock().unwrap();    
    let progress: Vec<&FitnessProgress> = db.get_all();
    HttpResponse::Ok().json(progress)
}

async fn delete_progress(app_state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.delete(&id.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()    
}

async fn update_progress(app_state: web::Data<AppState>, progress: web::Json<FitnessProgress>) -> impl Responder {
    let mut db = app_state.db.lock().unwrap();
    db.update(progress.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()    
}

async fn register (app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut db: std::sync::MutexGuard<Database> = app_state.db.lock().unwrap();
    db.insert_user(user.into_inner());
    let _ = db.save_to_file();
    HttpResponse::Ok().finish()
}

async fn login(app_state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let db: std::sync::MutexGuard<Database> = app_state.db.lock().unwrap();
    match db.get_user_by_name(&user.username) {
        Some(stored_user) if stored_user.password == user.password => {
            HttpResponse::Ok().body("Logged in!")         
        },
        _ => HttpResponse::BadRequest().body("Invalid username or password")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new()        
    };

    let data: web::Data<AppState> = web::Data::new(AppState {
        db: Mutex::new(db)
    });

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
            .max_age(3600)            
        )
        .app_data(data.clone())
        .route("/progress", web::post().to(create_progress))
        .route("/progress", web::get().to(read_all_progress))
        .route("/progress/{id}", web::get().to(read_progress))
        .route("/progress", web::put().to(update_progress))
        .route("/progress/{id}", web::delete().to(delete_progress))
        .route("/register", web::post().to(register))
        .route("/login", web::post().to(login))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}