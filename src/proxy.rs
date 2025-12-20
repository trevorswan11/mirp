use std::{
    collections::HashSet,
    env,
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::{Result, anyhow};
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::{error, info, warn};
use notify::{Event, RecursiveMode, Watcher};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use valence::protocol::packets::{DecodePacket, c2s::handshake::Handshake};
use valence::protocol::{Decode, VarInt};

const KB: usize = 1024;
const MAX_STRIKES: u8 = 5;

lazy_static! {
    static ref STRIKES: DashMap<IpAddr, u8> = DashMap::new();
}

static BLACKLIST_FILE: LazyLock<String> =
    LazyLock::new(|| env::var("BLACKLIST_PATH").unwrap_or_else(|_| "mirp.blacklist".to_string()));

fn load_blacklist() -> HashSet<IpAddr> {
    let path = &*BLACKLIST_FILE;
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse::<IpAddr>().ok())
        .collect()
}

pub struct ProxyConfig {
    pub target_domain: String,
    pub public_address: String,
    pub local_address: String,
}

pub async fn serve(config: ProxyConfig) -> Result<()> {
    let (blacklist, _watcher) = watch()?;

    info!("Identified target domain: {}", config.target_domain);
    info!("Identified public address: {}", config.public_address);
    let listener = TcpListener::bind(&config.public_address)
        .await
        .inspect_err(|e| error!("Failed to bind to public address: {e}"))?;
    info!("Listening on {}", config.public_address);

    info!("Identified local address: {}", config.local_address);
    println!("Successfully launched Mirp");

    loop {
        let (mut client_stream, peer_addr) = listener.accept().await?;

        // Check blacklist before dispatch
        {
            let ip = peer_addr.ip();
            let lock = blacklist.read().await;
            if lock.contains(&ip) {
                warn!("🚫 Dropped blacklisted IP: {}", ip);
                continue;
            }
        }

        let domain = config.target_domain.clone();
        let local = config.local_address.clone();

        tokio::spawn(async move {
            if let Err(e) = handle(&mut client_stream, peer_addr, domain, local).await {
                error!("Connection from {} closed: {}", peer_addr, e);
            }
        });
    }
}

fn watch() -> Result<(Arc<RwLock<HashSet<IpAddr>>>, notify::RecommendedWatcher)> {
    let initial_set = load_blacklist();
    let blacklist = Arc::new(RwLock::new(initial_set));

    let path = &*BLACKLIST_FILE;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .inspect_err(|e| error!("Could not create/open blacklist file: {e}"))?;

    let blacklist_clone = Arc::clone(&blacklist);
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res {
            // Blocking write here because watcher callbacks are usually synchronous
            if event.kind.is_modify() {
                let new_set = load_blacklist();
                if let Ok(mut lock) = blacklist_clone.try_write() {
                    *lock = new_set;
                    info!("Reloaded blacklist from disk");
                }
            }
        }
    })?;

    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
    Ok((blacklist, watcher))
}

async fn handle(
    client: &mut TcpStream,
    peer_addr: SocketAddr,
    target_domain: String,
    local_addr: String,
) -> Result<()> {
    // Client & server no delay to play nice with minecraft's packet frequency
    client.set_nodelay(true)?;
    let peer_ip = peer_addr.ip();

    let mut raw = [0u8; KB];
    let n = client.peek(&mut raw).await?;
    if n == 0 {
        return Ok(());
    }

    // If the packet header is unreadable, it's a strike
    let mut cursor = Cursor::new(raw);
    if let Err(_) = VarInt::decode(&mut cursor) {
        strike(peer_addr.ip()).await?;
        return Err(anyhow!("Invalid packet header from {}", peer_addr));
    }

    match Handshake::decode_packet(&mut cursor) {
        Ok(handshake) => {
            let used_addr = handshake
                .server_adddress
                .0
                .trim_end_matches('.')
                .to_string();

            if used_addr != target_domain {
                warn!("Dropped connection from {} to {}", peer_addr, used_addr);
                strike(peer_ip).await?;
                return Ok(());
            }

            // If they made it here we just assume its ok
            info!("Allowed connection from {} to {}", peer_addr, used_addr);

            if let Some(_) = STRIKES.remove(&peer_ip) {
                info!("Cleared strikes for {}", peer_ip);
            }

            let mut server = TcpStream::connect(local_addr).await?;
            server.set_nodelay(true)?;
            let transfer_buf_size = 128 * KB;
            io::copy_bidirectional_with_sizes(
                client,
                &mut server,
                transfer_buf_size,
                transfer_buf_size,
            )
            .await?;
        }
        Err(e) => warn!("Failed to decode packet: {e}"),
    }

    Ok(())
}

async fn strike(ip: IpAddr) -> Result<()> {
    if ip.is_loopback() || ip.to_string().starts_with("192.168.") {
        return Ok(());
    }

    let mut count = STRIKES.entry(ip).or_insert(0);
    *count += 1;
    info!(
        "Recorded strike for {} ({}/{})",
        ip.to_string(),
        *count,
        MAX_STRIKES
    );

    let path = &*BLACKLIST_FILE;
    if *count >= MAX_STRIKES {
        match OpenOptions::new().append(true).open(path) {
            Ok(mut file) => match writeln!(file, "{}", ip) {
                Ok(_) => {
                    info!("Banned {} for repeated failed connections", ip);
                    STRIKES.remove(&ip);
                }
                Err(e) => error!("Failed to write to blacklist file: {}", e),
            },
            Err(e) => error!("Failed to open blacklist file for appending: {}", e),
        }
    }

    Ok(())
}
