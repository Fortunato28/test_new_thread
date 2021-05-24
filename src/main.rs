use actix_web::{dev::Server, get, rt, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::{sync::Arc, thread, time};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<()> {
    let panel = ControlPanel::new("127.0.0.1:8080");
    panel.clone().start()?;

    println!("WAITING 10 SECONDS");
    thread::sleep(time::Duration::from_secs(10));

    let mut rx = panel.graceful_shutdown().ok_or(anyhow!("No receiver"))?;
    let received = rx.try_recv();
    dbg!(&received);

    Ok(())
}

struct ControlPanel {
    address: String,
    work_finished_receiver: Mutex<Option<oneshot::Receiver<Result<()>>>>,
    server: Arc<Mutex<Option<Server>>>,
}

impl ControlPanel {
    fn new(address: &str) -> Arc<Self> {
        Arc::new(Self {
            address: address.to_owned(),
            work_finished_receiver: Mutex::new(None),
            server: Arc::new(Mutex::new(None)),
        })
    }

    fn start(self: Arc<Self>) -> std::io::Result<()> {
        thread::spawn(move || {
            let _ = self.start_server();
        });

        Ok(())
    }

    fn stop(self: Arc<Self>) {
        let (tx, rx) = oneshot::channel();
        (*self.work_finished_receiver.lock()) = Some(rx);

        let cloned_self = self.clone();
        let runtime_handler = tokio::runtime::Handle::current();
        // FIXME can it die before main thread?
        let thread_result = thread::spawn(move || {
            let maybe_server = cloned_self.server.lock();
            if let Some(server) = &(*maybe_server) {
                runtime_handler.block_on(async {
                    server.stop(true).await;

                    let _ = tx.send(Ok(()));
                })
            }
        });

        //dbg!(&thread_result.join());
    }

    fn start_server(self: Arc<Self>) -> std::io::Result<()> {
        let address = self.address.clone();

        let system = Arc::new(rt::System::new());
        let server = HttpServer::new(|| App::new().service(health))
            .bind(&address)?
            .shutdown_timeout(3)
            .workers(1);

        system.block_on(async {
            *self.server.lock() = Some(server.run());
        });

        Ok(())
    }
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("Server is working")
}

pub trait Service: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn graceful_shutdown(self: Arc<Self>) -> Option<oneshot::Receiver<Result<()>>>;
}

impl Service for ControlPanel {
    fn name(&self) -> &str {
        "ControlPanel"
    }

    fn graceful_shutdown(self: Arc<Self>) -> Option<oneshot::Receiver<Result<()>>> {
        self.clone().stop();

        let work_finished_receiver = self.work_finished_receiver.lock().take();
        if work_finished_receiver.is_none() {
            dbg!(
                "'work_finished_receiver' wasn't created when started graceful shutdown in {}",
                self.name()
            );
        }

        work_finished_receiver
    }
}
