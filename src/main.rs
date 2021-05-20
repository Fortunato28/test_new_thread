use actix_web::{get, rt, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use std::{sync::Arc, thread, time};

#[actix_web::main]
async fn main() -> Result<()> {
    let panel = ControlPanel::new("127.0.0.1:8080");
    panel.start()?;

    println!("WAITING 10 SECONDS");
    thread::sleep(time::Duration::from_secs(10));

    Ok(())
}

struct ControlPanel {
    address: String,
}

impl ControlPanel {
    fn new(address: &str) -> Arc<Self> {
        Arc::new(Self {
            address: address.to_owned(),
        })
    }

    fn start(self: Arc<Self>) -> std::io::Result<()> {
        thread::spawn(move || {
            let _ = self.start_server();
        });

        Ok(())
    }

    fn start_server(&self) -> std::io::Result<()> {
        let system = rt::System::new();

        let server = HttpServer::new(|| App::new().service(health))
            .bind(&self.address)
            .expect("fix it")
            .shutdown_timeout(3)
            .workers(1)
            .run();

        system.block_on(server)
    }
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("Server is working")
}
