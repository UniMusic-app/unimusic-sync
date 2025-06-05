use std::sync::OnceLock;

use anyhow::anyhow;
use neon::{
    prelude::*,
    types::extract::{self, Error, TryIntoJs},
};
use tokio::{fs, runtime::Runtime};
use unimusic_sync::{IrohFactory, IrohManager};

type Result<T> = std::result::Result<T, Error>;

static UNIMUSIC: OnceLock<IrohManager> = OnceLock::new();

#[neon::export]
async fn initialize(path: String) -> Result<()> {
    let factory = IrohFactory::new();
    let iroh_manager = factory.iroh_manager(&path).await?;
    UNIMUSIC
        .set(iroh_manager)
        .map_err(|_| anyhow!("You can only create one UniMusicSync instance!"))?;
    Ok(())
}

#[neon::export]
async fn shutdown() -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;
    unimusic.shutdown().await?;
    Ok(())
}

#[neon::export]
async fn create_namespace() -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = unimusic.create_namespace().await?;

    Ok(namespace.into())
}

#[neon::export]
async fn delete_namespace(namespace: String) -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    unimusic.delete_namespace(namespace).await?;

    Ok(())
}

#[neon::export]
async fn get_author() -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let author = unimusic.get_author().await?;

    Ok(author.into())
}

#[neon::export]
async fn get_node_id() -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let node_id = unimusic.get_node_id().await;

    Ok(node_id.into())
}

#[neon::export]
async fn get_files(namespace: String) -> Result<impl for<'cx> TryIntoJs<'cx>> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    let files = unimusic.get_files(namespace).await?;

    Ok(extract::with(move |cx| {
        let result = cx.empty_array();

        for (i, entry) in files.iter().enumerate() {
            let obj = cx.empty_object();

            let key = entry.key().try_into_js(cx);
            obj.prop(cx, "key").set(key)?;

            let author = entry.author().to_string().try_into_js(cx);
            obj.prop(cx, "author").set(author)?;

            let timestamp = entry.timestamp();
            obj.prop(cx, "timestamp").set(timestamp as f64)?;

            let content_hash = entry.content_hash().to_string().try_into_js(cx);
            obj.prop(cx, "contentHash").set(content_hash)?;

            let content_len = entry.content_len();
            obj.prop(cx, "contentLen").set(content_len as f64)?;

            result.prop(cx, i as u32).set(obj)?;
        }

        Ok(result)
    }))
}

#[neon::export]
async fn write_file(namespace: String, sync_path: String, source_path: String) -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    let data = fs::read(source_path).await?;

    let file_hash = unimusic.write_file(namespace, sync_path, data).await?;

    Ok(file_hash.into())
}

#[neon::export]
async fn delete_file(namespace: String, sync_path: String) -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    unimusic.delete_file(namespace, sync_path).await?;

    Ok(())
}

#[neon::export]
async fn read_file(namespace: String, sync_path: String) -> Result<Vec<u8>> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    let data = unimusic.read_file(namespace, &sync_path).await?;

    Ok(data)
}

#[neon::export]
async fn read_file_hash(file_hash: String) -> Result<Vec<u8>> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let file_hash = file_hash.parse()?;
    let data = unimusic.read_file_hash(file_hash).await?;

    Ok(data)
}

#[neon::export]
async fn export_file(namespace: String, sync_path: String, destination_path: String) -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    unimusic
        .export(namespace, &sync_path, &destination_path)
        .await?;

    Ok(())
}

#[neon::export]
async fn export_hash(file_hash: String, destination_path: String) -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let file_hash = file_hash.parse()?;
    unimusic.export_hash(file_hash, &destination_path).await?;

    Ok(())
}

#[neon::export]
async fn share(namespace: String) -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    let ticket = unimusic.share(namespace).await?;

    Ok(ticket.into())
}

#[neon::export]
async fn import_file(ticket: String) -> Result<String> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let ticket = ticket.parse()?;
    let namespace = unimusic.import(ticket).await?;

    Ok(namespace.into())
}

#[neon::export]
async fn sync(namespace: String) -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    let namespace = namespace.parse()?;
    unimusic.sync(namespace).await?;

    Ok(())
}

#[neon::export]
async fn reconnect() -> Result<()> {
    let unimusic = UNIMUSIC
        .get()
        .ok_or_else(|| anyhow!("UniMusicSync is not initialized!"))?;

    unimusic.reconnect().await;

    Ok(())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    let runtime = match RUNTIME.get() {
        Some(runtime) => runtime,
        _ => {
            let runtime = Runtime::new().or_else(|err| cx.throw_error(err.to_string()))?;
            RUNTIME.set(runtime).unwrap();
            RUNTIME.get().unwrap()
        }
    };

    neon::set_global_executor(&mut cx, runtime)
        .or_else(|_| cx.throw_error("executor already set"))?;
    neon::registered().export(&mut cx)?;

    Ok(())
}
