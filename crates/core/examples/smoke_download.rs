//! Backend M1 smoke test (no UI):
//!   cargo run -p moonhunt-core --example smoke_download -- https://.../file
//!
//! Boots the full backend (DB → bridge → poller → aria2c), adds a download,
//! and prints every backend→UI event for manual inspection.

fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://proof.ovh.net/files/10Mb.dat".to_string());

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async move {
        let (app, mut rx) = moonhunt_core::AppHandle::new();

        moonhunt_core::db::init_db(&app).expect("db init");
        moonhunt_core::backend::start_backend(&app).expect("backend");

        // Spawn aria2c from the sidecar dir + saved settings.
        let a2 = app.clone();
        tokio::spawn(async move {
            match moonhunt_core::aria2::start_aria2(a2).await {
                Ok(m) => println!("[aria2] {m}"),
                Err(e) => eprintln!("[aria2] {e}"),
            }
        });

        // Give the daemon a moment to bind its RPC port, then add the download.
        let h = app.clone();
        let url2 = url.clone();
        tokio::spawn(async move {
            for _ in 0..30 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Ok(client) = moonhunt_core::aria2::Aria2Client::new() {
                    if client.get_version().await.is_ok() {
                        match moonhunt_core::aria2::add_download(
                            h, url2, None, None, None, None, None, None, None,
                        )
                        .await
                        {
                            Ok(g) => println!("[add] gid={g}"),
                            Err(e) => eprintln!("[add] {e}"),
                        }
                        return;
                    }
                }
            }
            eprintln!("[add] aria2 never came up");
        });

        // Stream the backend→UI event traffic (what the egui loop would drain).
        let mut seen: Option<tokio::time::Instant> = None;
        loop {
            let ev = tokio::time::timeout(std::time::Duration::from_secs(8), rx.recv()).await;
            let item = match ev {
                Ok(item) => item,
                Err(_) => {
                    let _ = moonhunt_core::aria2::shutdown_aria2().await;
                    println!("[done] event stream idle — exiting");
                    std::process::exit(0);
                }
            };
            match item {
                None => {
                    println!("[done] channel closed");
                    std::process::exit(0);
                }
                Some((name, payload)) => {
                    if name == "downloads-changed" {
                        if let Some(arr) = payload.as_array() {
                            for item in arr {
                                println!(
                                    "[dl] gid={} status={} {} / {} speed={} conns={}",
                                    item.get("gid").and_then(|v| v.as_str()).unwrap_or("?"),
                                    item.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
                                    item.get("completedLength").and_then(|v| v.as_str()).unwrap_or("0"),
                                    item.get("totalLength").and_then(|v| v.as_str()).unwrap_or("0"),
                                    item.get("downloadSpeed").and_then(|v| v.as_str()).unwrap_or("0"),
                                    item.get("connections").and_then(|v| v.as_str()).unwrap_or("0"),
                                );
                            }
                        }
                        seen = Some(tokio::time::Instant::now());
                    } else if name != "live-stats-changed" {
                        println!("[event] {name}: {payload}");
                    }
                }
            }
        }
    });
}
