use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod server;
mod tools;

#[tokio::main]
async fn main() {
    server::run().await;
}
