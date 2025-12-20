mod register;
mod proxy;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let serve_vars = register::env()?;
    register::logger()?;
    Ok(proxy::serve(serve_vars).await?)
}
