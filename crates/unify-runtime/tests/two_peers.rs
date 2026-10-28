use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use unify_core::config::SettingsPatch;
use unify_platform::fake::FakePlatform;
use unify_platform::{CaptureEvent, InjectEvent, Platform};
use unify_proto::keymap::usage;
use unify_proto::{InputEvent, Message, OsKind, Side};
use unify_runtime::{ConnectionState, ControlState, PairingStatus, RuntimeHandle, RuntimeOptions, StatusSnapshot};
use unify_security::{MemorySecretStore, SecretStore};

const WAIT: Duration = Duration::from_secs(15);

struct Node {
    handle: RuntimeHandle,
    platform: FakePlatform,
    dir: PathBuf,
    secrets: Arc<dyn SecretStore>,
}

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("unify-it-{name}-{}-{}", std::process::id(), rand_suffix()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rand_suffix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64 % 1_000_000_007
}

fn start_node(dir: PathBuf, platform: FakePlatform, secrets: Arc<dyn SecretStore>) -> Node {
    let handle = unify_runtime::start(RuntimeOptions {
        config_dir: dir.clone(),
        profile: String::from("test"),
        platform: Arc::new(platform.clone()) as Arc<dyn Platform>,
        secrets: secrets.clone(),
        bind_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        listen_port_override: Some(0),
        discovery_enabled: false,
    })
    .expect("runtime starts");
    Node { handle, platform, dir, secrets }
}

fn new_node(name: &str, os: OsKind) -> Node {
    let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecretStore::default());
    start_node(test_dir(name), FakePlatform::new(os, name, 1920, 1080), secrets)
}

async fn wait_for(node: &Node, what: &str, predicate: impl Fn(&StatusSnapshot) -> bool) -> StatusSnapshot {
    let mut rx = node.handle.subscribe();
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let snapshot = rx.borrow_and_update().clone();
        if predicate(&snapshot) {
            return snapshot;
        }
        match tokio::time::timeout_at(deadline, rx.changed()).await {
            Ok(Ok(())) => {}
            _ => panic!("timed out waiting for {what}; last status: {:?}", node.handle.status()),
        }
    }
}

async fn wait_until(what: &str, check: impl Fn() -> bool) {
    let deadline = tokio::time::Instant::now() + WAIT;
    while !check() {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for {what}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn address_of(node: &Node) -> String {
    format!("127.0.0.1:{}", node.handle.status().device.port)
}

async fn pair(host: &Node, joiner: &Node) {
    host.handle.start_pairing_host().await.unwrap();
    let status = wait_for(host, "pairing code", |s| matches!(s.pairing, Some(PairingStatus::Hosting { .. }))).await;
    let Some(PairingStatus::Hosting { code, .. }) = status.pairing else { unreachable!() };
    joiner.handle.join_pairing(address_of(host), code).await.unwrap();
    wait_for(host, "approval request", |s| matches!(s.pairing, Some(PairingStatus::AwaitingApproval { .. }))).await;
    host.handle.approve_pairing(true).unwrap();
    wait_for(host, "host connected", |s| s.connection.state == ConnectionState::Connected).await;
    wait_for(joiner, "joiner connected", |s| s.connection.state == ConnectionState::Connected).await;
}

async fn connected_pair(name: &str) -> (Node, Node) {
    let mac = new_node(&format!("{name}-mac"), OsKind::MacOs);
    let win = new_node(&format!("{name}-win"), OsKind::Windows);
    mac.handle.update_settings(SettingsPatch { peer_side: Some(Side::Left), ..Default::default() }).await.unwrap();
    mac.handle.update_settings(SettingsPatch { peer_side: Some(Side::Right), ..Default::default() }).await.unwrap();
    pair(&win, &mac).await;
    wait_for(&win, "layout sync", |s| s.settings.peer_side == Side::Left).await;
    (mac, win)
}

async fn cleanup(nodes: Vec<Node>) {
    for node in nodes {
        node.handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&node.dir);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pairing_requires_approval_and_records_both_peers() {
    let (mac, win) = connected_pair("pair").await;
    let mac_status = mac.handle.status();
    let win_status = win.handle.status();
    assert_eq!(mac_status.peer.as_ref().unwrap().id, win_status.device.id);
    assert_eq!(win_status.peer.as_ref().unwrap().id, mac_status.device.id);
    assert_eq!(mac_status.peer.as_ref().unwrap().fingerprint, win_status.device.fingerprint);
    assert_eq!(win_status.peer.as_ref().unwrap().os, "macos");
    assert!(matches!(win_status.pairing, Some(PairingStatus::Succeeded { .. })));
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wrong_pairing_code_is_rejected_without_pairing() {
    let host = new_node("wrong-host", OsKind::Windows);
    let joiner = new_node("wrong-join", OsKind::MacOs);
    host.handle.start_pairing_host().await.unwrap();
    let status = wait_for(&host, "pairing code", |s| matches!(s.pairing, Some(PairingStatus::Hosting { .. }))).await;
    let Some(PairingStatus::Hosting { code, .. }) = status.pairing else { unreachable!() };
    let wrong = if code == "000000" { "111111".to_owned() } else { "000000".to_owned() };
    joiner.handle.join_pairing(address_of(&host), wrong).await.unwrap();
    wait_for(&joiner, "joiner failure", |s| matches!(s.pairing, Some(PairingStatus::Failed { reason: "pairing_code_mismatch" }))).await;
    let host_status = host.handle.status();
    assert!(host_status.peer.is_none());
    assert!(matches!(host_status.pairing, Some(PairingStatus::Hosting { .. })), "host keeps the window open after one failure");
    assert!(joiner.handle.status().peer.is_none());
    cleanup(vec![host, joiner]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invalid_code_and_address_are_rejected_before_network() {
    let node = new_node("invalid", OsKind::MacOs);
    assert_eq!(node.handle.join_pairing("127.0.0.1:1".into(), "12ab56".into()).await.unwrap_err().code, "pairing_code_invalid");
    assert_eq!(node.handle.join_pairing("bad host".into(), "123456".into()).await.unwrap_err().code, "address_invalid");
    cleanup(vec![node]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_rejection_of_pairing_request_leaves_both_unpaired() {
    let host = new_node("reject-host", OsKind::Windows);
    let joiner = new_node("reject-join", OsKind::MacOs);
    host.handle.start_pairing_host().await.unwrap();
    let status = wait_for(&host, "pairing code", |s| matches!(s.pairing, Some(PairingStatus::Hosting { .. }))).await;
    let Some(PairingStatus::Hosting { code, .. }) = status.pairing else { unreachable!() };
    joiner.handle.join_pairing(address_of(&host), code).await.unwrap();
    wait_for(&host, "approval request", |s| matches!(s.pairing, Some(PairingStatus::AwaitingApproval { .. }))).await;
    host.handle.approve_pairing(false).unwrap();
    wait_for(&joiner, "remote rejection", |s| matches!(s.pairing, Some(PairingStatus::Failed { reason: "pairing_rejected_remote" }))).await;
    assert!(host.handle.status().peer.is_none());
    assert!(joiner.handle.status().peer.is_none());
    cleanup(vec![host, joiner]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn clipboard_syncs_both_directions_preserving_turkish_text_and_line_endings() {
    let (mac, win) = connected_pair("clip").await;
    let from_mac = "Merhaba Dünya — İĞÜŞÖÇ ığüşöç\r\nikinci satır\n\tüçüncü 🙂";
    mac.platform.user_copies(from_mac);
    wait_until("windows clipboard", || win.platform.clipboard_text().as_deref() == Some(from_mac)).await;
    let from_win = "Windows'tan geri: çğı\r\n".repeat(20_000);
    win.platform.user_copies(&from_win);
    wait_until("mac clipboard", || mac.platform.clipboard_text().as_deref() == Some(from_win.as_str())).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mac_stats = mac.handle.status().stats;
    let win_stats = win.handle.status().stats;
    assert_eq!(mac_stats.clipboard_sent, 1, "no echo loop from mac");
    assert_eq!(win_stats.clipboard_sent, 1, "no echo loop from windows");
    assert_eq!(mac_stats.clipboard_received, 1);
    assert_eq!(win_stats.clipboard_received, 1);
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concealed_clipboard_content_is_never_sent() {
    let (mac, win) = connected_pair("conceal").await;
    win.platform.user_copies("önceki");
    wait_until("baseline", || mac.platform.clipboard_text().as_deref() == Some("önceki")).await;
    mac.platform.user_copies_concealed("gizli-parola");
    wait_for(&mac, "skip reason", |s| s.clipboard.last_skip == Some("concealed")).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(win.platform.clipboard_text().as_deref(), Some("önceki"));
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disabling_clipboard_sync_stops_transfers() {
    let (mac, win) = connected_pair("clipoff").await;
    mac.handle.update_settings(SettingsPatch { clipboard_sync: Some(false), ..Default::default() }).await.unwrap();
    mac.platform.user_copies("gönderilmemeli");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_ne!(win.platform.clipboard_text().as_deref(), Some("gönderilmemeli"));
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn edge_crossing_transfers_control_and_translates_command_to_control() {
    let (mac, win) = connected_pair("input").await;
    mac.platform.emit_capture(CaptureEvent::LocalMotion { x: 1000, y: 540 });
    mac.platform.emit_capture(CaptureEvent::LocalMotion { x: 1919, y: 540 });
    wait_for(&mac, "mac controlling", |s| s.control == ControlState::Controlling).await;
    wait_for(&win, "windows controlled", |s| s.control == ControlState::Controlled).await;
    assert!(mac.platform.capture_shared().unwrap().is_remote());
    wait_until("entry move", || win.platform.injected().iter().any(|e| matches!(e, InjectEvent::MoveTo { x: 2, y: 540, .. }))).await;

    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: usage::LEFT_GUI, down: true }));
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: 0x06, down: true }));
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: 0x06, down: false }));
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: usage::LEFT_GUI, down: false }));
    wait_until("ctrl+c injected", || {
        let keys: Vec<_> = win.platform.injected().into_iter().filter(|e| matches!(e, InjectEvent::Key { .. })).collect();
        keys == vec![
            InjectEvent::Key { usage: usage::LEFT_CTRL, down: true },
            InjectEvent::Key { usage: 0x06, down: true },
            InjectEvent::Key { usage: 0x06, down: false },
            InjectEvent::Key { usage: usage::LEFT_CTRL, down: false },
        ]
    })
    .await;

    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::MouseMove { dx: 100, dy: 0 }));
    wait_until("relative move", || win.platform.injected().iter().any(|e| matches!(e, InjectEvent::MoveTo { x: 102, y: 540, .. }))).await;
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::MouseMove { dx: -500, dy: 0 }));
    wait_for(&mac, "control returned", |s| s.control == ControlState::Local).await;
    wait_for(&win, "windows local", |s| s.control == ControlState::Local).await;
    assert!(!mac.platform.capture_shared().unwrap().is_remote());
    let warps = mac.platform.warps();
    let last = *warps.last().unwrap();
    assert_eq!(last.0, 1919 - 2, "cursor restored just inside the right edge");
    assert!((last.1 - 540).abs() <= 1);
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn input_from_peer_is_ignored_unless_control_was_granted() {
    let (mac, win) = connected_pair("deny").await;
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: 0x04, down: true }));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(win.platform.injected().is_empty());
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn connection_loss_releases_keys_held_on_controlled_machine() {
    let (mac, win) = connected_pair("release").await;
    mac.platform.emit_capture(CaptureEvent::Hotkey);
    wait_for(&win, "controlled", |s| s.control == ControlState::Controlled).await;
    wait_until("center entry", || win.platform.injected().iter().any(|e| matches!(e, InjectEvent::MoveTo { x: 960, y: 540, .. }))).await;
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: usage::LEFT_SHIFT, down: true }));
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::MouseButton { button: unify_proto::MouseButton::Left, down: true }));
    wait_until("held inputs", || win.platform.injected().len() >= 3).await;
    win.platform.clear_injected();
    let mac_dir = mac.dir.clone();
    mac.handle.shutdown().await;
    wait_for(&win, "local after loss", |s| s.control == ControlState::Local).await;
    let injected = win.platform.injected();
    assert!(injected.contains(&InjectEvent::Key { usage: usage::LEFT_SHIFT, down: false }), "{injected:?}");
    assert!(injected.contains(&InjectEvent::Button { button: unify_proto::MouseButton::Left, down: false }), "{injected:?}");
    let _ = std::fs::remove_dir_all(mac_dir);
    cleanup(vec![win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hotkey_on_controller_returns_control_and_releases_remote_keys() {
    let (mac, win) = connected_pair("hotkey").await;
    mac.platform.emit_capture(CaptureEvent::Hotkey);
    wait_for(&win, "controlled", |s| s.control == ControlState::Controlled).await;
    mac.platform.emit_capture(CaptureEvent::Input(InputEvent::Key { usage: usage::LEFT_ALT, down: true }));
    wait_until("alt injected", || win.platform.injected().contains(&InjectEvent::Key { usage: usage::LEFT_ALT, down: true })).await;
    mac.platform.emit_capture(CaptureEvent::Hotkey);
    wait_for(&mac, "mac local", |s| s.control == ControlState::Local).await;
    wait_for(&win, "win local", |s| s.control == ControlState::Local).await;
    wait_until("alt released", || win.platform.injected().contains(&InjectEvent::Key { usage: usage::LEFT_ALT, down: false })).await;
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn peer_with_input_sharing_disabled_refuses_control() {
    let (mac, win) = connected_pair("refuse").await;
    win.handle.update_settings(SettingsPatch { input_sharing: Some(false), ..Default::default() }).await.unwrap();
    mac.platform.emit_capture(CaptureEvent::Hotkey);
    wait_for(&mac, "refusal", |s| s.last_error.as_ref().is_some_and(|e| e.code == "peer_released_control")).await;
    assert_eq!(mac.handle.status().control, ControlState::Local);
    assert!(win.platform.injected().is_empty());
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reconnects_after_peer_restart_with_same_identity() {
    let (mac, win) = connected_pair("reconnect").await;
    let win_dir = win.dir.clone();
    let win_secrets = win.secrets.clone();
    let win_platform = win.platform.clone();
    let old_port = win.handle.status().device.port;
    win.handle.shutdown().await;
    wait_for(&mac, "mac notices loss", |s| s.connection.state != ConnectionState::Connected).await;
    let restarted = start_node(win_dir, win_platform, win_secrets);
    let status = wait_for(&restarted, "restarted node listening", |s| s.device.port != 0).await;
    assert_ne!(status.peer, None, "pairing survives restart");
    let _ = old_port;
    wait_for(&restarted, "reconnected", |s| s.connection.state == ConnectionState::Connected).await;
    wait_for(&mac, "mac reconnected", |s| s.connection.state == ConnectionState::Connected).await;
    mac.platform.user_copies("yeniden bağlandı");
    wait_until("clipboard after reconnect", || restarted.platform.clipboard_text().as_deref() == Some("yeniden bağlandı")).await;
    cleanup(vec![mac, restarted]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn removing_peer_disconnects_and_rejects_further_sessions() {
    let (mac, win) = connected_pair("remove").await;
    win.handle.remove_peer().await.unwrap();
    wait_for(&win, "not paired", |s| s.connection.state == ConnectionState::NotPaired && s.peer.is_none()).await;
    wait_for(&mac, "mac disconnected", |s| s.connection.state != ConnectionState::Connected).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_ne!(mac.handle.status().connection.state, ConnectionState::Connected);
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unpaired_certificate_is_rejected_after_hello() {
    let (mac, win) = connected_pair("intruder").await;
    let intruder = unify_security::DeviceIdentity::generate().unwrap();
    let config = unify_security::tls::client_config(&intruder, unify_security::tls::ServerExpectation::AnyForPairing).unwrap();
    let (stream, _) = unify_net::connect_tls(&address_of(&win), config, Duration::from_secs(3), Duration::from_secs(3)).await.unwrap();
    let (mut reader, writer) = tokio::io::split(stream);
    let (outbound, _writer) = unify_net::spawn_writer(writer);
    let mac_id = mac.handle.status().device.id;
    outbound
        .send_high(Message::Hello(unify_proto::Hello {
            protocol_version: unify_proto::PROTOCOL_VERSION,
            device_id: mac_id,
            device_name: "Sahte".into(),
            os: OsKind::MacOs,
            app_version: "0.1.0".into(),
            listen_port: 1,
        }))
        .unwrap();
    let reply = tokio::time::timeout(WAIT, unify_net::read_message(&mut reader)).await.unwrap();
    assert!(matches!(reply, Ok(Message::Goodbye { reason: unify_proto::GoodbyeReason::Unpaired })), "{reply:?}");
    outbound.send_high(Message::Input(InputEvent::Key { usage: 0x04, down: true })).ok();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(win.platform.injected().is_empty());
    assert_eq!(win.handle.status().connection.state, ConnectionState::Connected, "legitimate session unaffected");
    assert!(win.handle.status().stats.rejected_connections >= 1);
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unauthenticated_input_before_hello_closes_connection() {
    let (mac, win) = connected_pair("prehello").await;
    let intruder = unify_security::DeviceIdentity::generate().unwrap();
    let config = unify_security::tls::client_config(&intruder, unify_security::tls::ServerExpectation::AnyForPairing).unwrap();
    let (stream, _) = unify_net::connect_tls(&address_of(&win), config, Duration::from_secs(3), Duration::from_secs(3)).await.unwrap();
    let (mut reader, writer) = tokio::io::split(stream);
    let (outbound, _writer) = unify_net::spawn_writer(writer);
    outbound.send_high(Message::Input(InputEvent::Key { usage: 0x04, down: true })).unwrap();
    let reply = tokio::time::timeout(WAIT, unify_net::read_message(&mut reader)).await.unwrap();
    assert!(matches!(reply, Ok(Message::Goodbye { reason: unify_proto::GoodbyeReason::ProtocolViolation })), "{reply:?}");
    assert!(win.platform.injected().is_empty());
    cleanup(vec![mac, win]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn measures_round_trip_time_over_loopback() {
    let (mac, win) = connected_pair("rtt").await;
    let status = wait_for(&mac, "rtt sample", |s| s.connection.rtt_ms.is_some()).await;
    let rtt = status.connection.rtt_ms.unwrap();
    println!("loopback TLS ping/pong RTT: {rtt} ms");
    assert!(rtt < 200);
    cleanup(vec![mac, win]).await;
}
