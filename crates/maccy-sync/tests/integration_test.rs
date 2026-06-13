use std::time::Duration;
use std::sync::{Arc, Mutex};

use maccy_sync::state::{SharedState, SyncCommand, SyncState};
use maccy_sync::types::PeerInfo;

/// Build a NetworkManager and run it on a dedicated thread (same as FFI does).
/// Returns a command sender for sending commands.
fn spawn_node(
    device_name: &str,
    device_id: &str,
    port: u16,
    events: Arc<Mutex<Vec<String>>>,
) -> tokio::sync::mpsc::UnboundedSender<SyncCommand> {
    let state = SyncState::new(device_name, device_id).unwrap();
    let shared: SharedState = Arc::new(std::sync::Mutex::new(state));

    // Register callbacks that capture events
    {
        let mut guard = shared.lock().unwrap();
        let ev = events.clone();
        *guard.on_peer_discovered.lock() = Some(Box::new(move |info: PeerInfo| {
            ev.lock().unwrap().push(
                format!("peer_discovered: {} ({}) connected={}", info.display_name, info.peer_id, info.is_connected)
            );
        }));
        let ev = events.clone();
        *guard.on_peer_lost.lock() = Some(Box::new(move |peer_id: String| {
            ev.lock().unwrap().push(format!("peer_lost: {}", peer_id));
        }));
        let ev = events.clone();
        *guard.on_error.lock() = Some(Box::new(move |(code, msg): (i32, String)| {
            ev.lock().unwrap().push(format!("error({}): {}", code, msg));
        }));
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let key = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = key.public().to_peer_id();
    let shared_clone = shared.clone();

    let rt = shared.lock().unwrap().runtime.handle().clone();

    std::thread::spawn(move || {
        let mut nm = if port == 31774 {
            maccy_sync::network::NetworkManager::new(cmd_rx, shared_clone.clone(), key).unwrap()
        } else {
            maccy_sync::network::NetworkManager::new_with_port(cmd_rx, shared_clone.clone(), key, port).unwrap()
        };
        rt.block_on(async move { nm.run().await });
    });

    println!("Spawned node '{}' on port {} with peer_id {}", device_name, port, peer_id);
    cmd_tx
}

fn wait_and_report(events: Arc<Mutex<Vec<String>>>, label: &str, secs: u64) {
    println!("\n--- Waiting {}s for {} ---", secs, label);
    std::thread::sleep(Duration::from_secs(secs));

    let evts = events.lock().unwrap();
    println!("\n=== Captured events ({}) ===", label);
    for e in evts.iter() {
        println!("  {}", e);
    }
    println!("=== Total: {} events ===\n", evts.len());
}

/// Test: manually dial from NodeA to NodeB
#[test]
fn test_manual_dial_two_nodes() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Node A on port 31774
    let cmd_tx_a = spawn_node("NodeA", "device-a", 31774, events.clone());
    // Node B on port 31775
    let cmd_tx_b = spawn_node("NodeB", "device-b", 31775, events.clone());

    // Wait for both to start listening
    std::thread::sleep(Duration::from_secs(2));

    // Dial from A to B
    println!("\n--- Dialing NodeB at /ip4/127.0.0.1/tcp/31775 ---");
    let _ = cmd_tx_a.send(SyncCommand::AddPeerAddress {
        peer_id: String::new(),
        address: "/ip4/127.0.0.1/tcp/31775".to_string(),
    });

    wait_and_report(events.clone(), "manual dial", 10);

    let evts = events.lock().unwrap();
    let discovered = evts.iter().any(|e| e.contains("peer_discovered"));
    let errors: Vec<_> = evts.iter().filter(|e| e.contains("error")).cloned().collect();

    println!("has_discovered={}, errors={}", discovered, errors.len());
    for e in &errors {
        eprintln!("  ERROR: {}", e);
    }

    assert!(discovered, "Should discover peer after manual dial. Errors: {:?}", errors);

    // Cleanup
    let _ = cmd_tx_a.send(SyncCommand::Shutdown);
    let _ = cmd_tx_b.send(SyncCommand::Shutdown);
}

/// Test: mDNS auto-discovery between two nodes on localhost
#[test]
fn test_mdns_discovery_two_nodes() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Node A on port 31774
    let cmd_tx_a = spawn_node("NodeA", "device-a", 31774, events.clone());
    // Node B on port 31775
    let cmd_tx_b = spawn_node("NodeB", "device-b", 31775, events.clone());

    wait_and_report(events.clone(), "mDNS discovery", 15);

    let evts = events.lock().unwrap();
    let discovered = evts.iter().filter(|e| e.contains("peer_discovered")).count();
    println!("mDNS discovery count: {}", discovered);

    // Cleanup
    let _ = cmd_tx_a.send(SyncCommand::Shutdown);
    let _ = cmd_tx_b.send(SyncCommand::Shutdown);
}
