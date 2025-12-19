use std::io::Cursor;

use anyhow::Result;
use log::{error, info, warn};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};

use valence::protocol::packets::{DecodePacket, c2s::handshake::Handshake};
use valence::protocol::{Decode, VarInt};

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
    loop {
        let (mut client_stream, peer_addr) = listener.accept().await?;

        // Clone variables for the async task
        let domain = target_domain.clone();
        let local = local_addr.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_proxy(&mut client_stream, domain, local).await {
                error!("Connection from {} closed: {}", peer_addr, e);
            }
        });
    }
}

async fn handle_proxy(
    client: &mut TcpStream,
    target_domain: String,
    local_addr: String,
) -> Result<()> {
    // Peek at the first 1KB to find the Minecraft Handshake
    let mut raw = [0u8; 1024];
    let n = client.peek(&mut raw).await?;
    if n == 0 {
        return Ok(());
    }

    let mut cursor = Cursor::new(raw);
    let _length = VarInt::decode(&mut cursor)?;
    
    // The head of a packet looks like this
    // (0x15)(0x00)(0xfb05)(0x11mc.yourdomain.com)(0x63dd)(0x01)
    // <Packet Length><Packet Id><Something><Domain len + Domain><Port><Status>
    match Handshake::decode_packet(&mut cursor) {
        Ok(handshake) => {
            let used_addr = handshake.server_adddress.0.trim_end_matches('.').to_string();
    
            if used_addr != target_domain {
                warn!("Dropping incorrect connection (Player used: {})", used_addr);
                return Ok(());
            }
    
            info!("Trusted connection for domain: {}", used_addr);
    
            let mut server = TcpStream::connect(local_addr).await?;
            io::copy_bidirectional(client, &mut server).await?;
        }
        Err(e) => warn!("Failed to decode packet: {e}")
    }
    
    Ok(())
}
