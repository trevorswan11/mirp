use std::io::Cursor;

use anyhow::Result;
use log::{error, info, warn};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};

use valence::protocol::packets::{DecodePacket, c2s::handshake::Handshake};
use valence::protocol::{Decode, VarInt};

const KB: usize = 1024;

pub async fn serve(
    (target_domain, public_addr, local_addr): (String, String, String),
) -> Result<()> {
    info!("Identified target domain: {}", target_domain);
    info!("Identified public address: {}", public_addr);
    let listener = TcpListener::bind(&public_addr)
        .await
        .inspect_err(|e| error!("Failed to bind to public address: {e}"))?;
    info!("Listening on {}", public_addr);

    info!("Identified local address: {}", local_addr);
    println!("Successfully launched Mirp");
    
    loop {
        let (mut client_stream, peer_addr) = listener.accept().await?;

        let domain = target_domain.clone();
        let local = local_addr.clone();

        tokio::spawn(async move {
            if let Err(e) = handle(&mut client_stream, domain, local).await {
                error!("Connection from {} closed: {}", peer_addr, e);
            }
        });
    }
}

async fn handle(
    client: &mut TcpStream,
    target_domain: String,
    local_addr: String,
) -> Result<()> {
    // Client & server no delay to play nice with minecraft's packet frequency
    client.set_nodelay(true)?;

    let mut raw = [0u8; KB];
    let n = client.peek(&mut raw).await?;
    if n == 0 {
        return Ok(());
    }

    let mut cursor = Cursor::new(raw);
    VarInt::decode(&mut cursor)?;
    
    match Handshake::decode_packet(&mut cursor) {
        Ok(handshake) => {
            let user_addr = client.local_addr()?.to_string();
            let used_addr = handshake.server_adddress.0.trim_end_matches('.').to_string();
    
            if used_addr != target_domain {
                warn!("Dropped connection from {} to {}", user_addr, used_addr);
                return Ok(());
            } else {
                info!("Allowed connection from {} to {}", user_addr, used_addr);
            }
    
            let mut server = TcpStream::connect(local_addr).await?;
            server.set_nodelay(true)?;
            let transfer_buf_size = 128 * KB;
            io::copy_bidirectional_with_sizes(client, &mut server, transfer_buf_size, transfer_buf_size).await?;
        }
        Err(e) => warn!("Failed to decode packet: {e}")
    }
    
    Ok(())
}
