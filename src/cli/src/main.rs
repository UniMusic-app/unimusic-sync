use std::error::Error;

use sync::start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    start().await
}
