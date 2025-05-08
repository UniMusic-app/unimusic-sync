use std::error::Error;
use sync::IrohManager;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

async fn receive(ticket: &str) -> Result<()> {
    let iroh_manager = IrohManager::new("./tests/receive").await?;
    let namespace = iroh_manager.import(ticket).await?;

    if let Ok(dog_bytes) = iroh_manager.read_file(namespace, "dog.txt").await {
        let dog_string = String::from_utf8_lossy(&dog_bytes);
        println!("Dog contents (imported): {}", dog_string);
    } else {
        println!("No dog found");
    }

    tokio::signal::ctrl_c().await?;
    iroh_manager.close().await?;

    Ok(())
}

async fn provide() -> Result<()> {
    let mut iroh_manager = IrohManager::new("./tests/provide").await?;
    let namespace = iroh_manager.get_or_create_namespace().await?;

    if let Ok(dog_bytes) = iroh_manager.read_file(namespace, "dog.txt").await {
        let dog_string = String::from_utf8_lossy(&dog_bytes);
        println!("Dog contents (before write): {}", dog_string);
    }

    let hash = iroh_manager
        .write_file(
            namespace,
            "dog.txt".to_string(),
            "WOOF!".as_bytes().to_vec(),
        )
        .await?;

    let dog_bytes = iroh_manager.read_file(namespace, "dog.txt").await?;
    let dog_string = String::from_utf8_lossy(&dog_bytes);
    println!("Dog contents: {}", dog_string);

    let dog_bytes = iroh_manager.read_file_hash(hash).await?;
    let dog_string = String::from_utf8_lossy(&dog_bytes);
    println!("Dog contents (from hash): {}", dog_string);

    let ticket = iroh_manager.share(namespace).await?;
    println!("Ticket: {0}", ticket);

    tokio::signal::ctrl_c().await?;
    iroh_manager.close().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().collect::<Box<[_]>>();
    match args.get(1).map(String::as_ref) {
        Some("receive") => {
            if let Some(ticket) = args.get(2) {
                receive(ticket).await?
            } else {
                Err("<cmd> receive <ticket>")?;
            }
        }
        Some("provide") => provide().await?,
        _ => Err("<cmd> receive/provide")?,
    }

    Ok(())
}
