//! Deferred-launch contract: zero network dials before `launch_holochain`,
//! traffic + setup-completed after, `AlreadyLaunched` on a second call.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::Listener;
use tauri_plugin_holochain::{
    init_deferred, vec_to_locked, Error, HolochainExt, HolochainPluginConfig, NetworkConfig,
};

struct LocalEndpoints {
    network_config: NetworkConfig,
    accept_counts: [Arc<AtomicUsize>; 3],
}

impl LocalEndpoints {
    fn total_accepts(&self) -> usize {
        self.accept_counts
            .iter()
            .map(|count| count.load(Ordering::SeqCst))
            .sum()
    }
}

async fn bind_local_endpoints() -> LocalEndpoints {
    let (bootstrap_port, bootstrap_accepts) = bind_counted_listener().await;
    let (signal_port, signal_accepts) = bind_counted_listener().await;
    let (relay_port, relay_accepts) = bind_counted_listener().await;

    let network_config = NetworkConfig {
        bootstrap_url: url2::url2!("http://127.0.0.1:{}", bootstrap_port),
        signal_url: url2::url2!("ws://127.0.0.1:{}", signal_port),
        relay_url: url2::url2!("http://127.0.0.1:{}", relay_port),
        ..NetworkConfig::default()
    };

    LocalEndpoints {
        network_config,
        accept_counts: [bootstrap_accepts, signal_accepts, relay_accepts],
    }
}

async fn bind_counted_listener() -> (u16, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local listener");
    let port = listener.local_addr().expect("local addr").port();
    let accept_count = Arc::new(AtomicUsize::new(0));
    let accept_count_in_loop = accept_count.clone();
    tokio::spawn(async move {
        loop {
            if listener.accept().await.is_ok() {
                accept_count_in_loop.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    (port, accept_count)
}

async fn wait_until(predicate: impl Fn() -> bool, timeout: Duration, what: &str) {
    let deadline = tokio::time::Instant::now() + timeout;
    while !predicate() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn launch_gates_network_and_rejects_double_launch() {
    let endpoints = bind_local_endpoints().await;
    let holochain_dir = tempfile::tempdir().expect("tempdir");
    let app = tauri::test::mock_builder()
        .plugin(init_deferred(HolochainPluginConfig::new(
            holochain_dir.path().into(),
            endpoints.network_config.clone(),
        )))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("mock app must build");

    let setup_completed = Arc::new(AtomicBool::new(false));
    let setup_completed_flag = setup_completed.clone();
    app.listen("holochain://setup-completed", move |_| {
        setup_completed_flag.store(true, Ordering::SeqCst);
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(
        endpoints.total_accepts(),
        0,
        "no dial may happen before launch_holochain"
    );
    assert!(
        app.holochain().is_err(),
        "runtime must be absent before launch"
    );

    app.launch_holochain(vec_to_locked(b"test-passphrase".to_vec()))
        .await
        .expect("first launch must succeed");
    assert!(
        app.holochain().is_ok(),
        "runtime must be present after launch"
    );
    wait_until(
        || setup_completed.load(Ordering::SeqCst),
        Duration::from_secs(10),
        "setup-completed event",
    )
    .await;
    wait_until(
        || endpoints.total_accepts() >= 1,
        Duration::from_secs(30),
        "post-launch network dial",
    )
    .await;

    let second = app
        .launch_holochain(vec_to_locked(b"test-passphrase".to_vec()))
        .await;
    assert!(
        matches!(&second, Err(Error::AlreadyLaunched)),
        "second launch must return AlreadyLaunched, got {second:?}"
    );
}
