mod register;
mod proxy;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let serve_vars = register::env()?;
    register::logger()?;
    Ok(proxy::serve(serve_vars).await?)
}
