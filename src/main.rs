mod proxy;
mod register;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let proxy_info = register::env()?;
    register::logger()?;
    Ok(proxy::serve(proxy_info).await?)
}
