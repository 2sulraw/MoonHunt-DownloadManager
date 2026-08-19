//! Typed aria2 JSON-RPC client. All RPC traffic should go through this type.

use crate::state::{current_rpc_port, ARIA2_RPC_TOKEN};
use crate::util::no_proxy_client;
use futures_util::StreamExt;
use serde_json::{json, Map, Value};

/// Thin client over aria2's JSON-RPC HTTP endpoint.
pub struct Aria2Client {
    port: u32,
    token: String,
    http: reqwest::Client,
}

impl Aria2Client {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            port: current_rpc_port(),
            token: format!("token:{}", ARIA2_RPC_TOKEN),
            http: no_proxy_client()?,
        })
    }

    fn rpc_url(&self) -> String {
        format!("http://localhost:{}/jsonrpc", self.port)
    }

    /// Low-level JSON-RPC call. `params` should NOT include the token — it is prepended.
    pub async fn call(&self, method: &str, params: Vec<Value>) -> Result<Value, String> {
        let mut full_params = Vec::with_capacity(params.len() + 1);
        full_params.push(json!(self.token));
        full_params.extend(params);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": method,
            "params": full_params
        });

        let response = self
            .http
            .post(self.rpc_url())
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to aria2: {}", e))?;

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to get response text: {}", e))?;

        let result: Value =
            serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {} — {}", e, text))?;

        if let Some(err) = result.get("error") {
            return Err(format!("Aria2 error: {}", err));
        }
        Ok(result)
    }

    /// Call and extract the `result` field (any JSON value).
    pub async fn call_result(&self, method: &str, params: Vec<Value>) -> Result<Value, String> {
        let resp = self.call(method, params).await?;
        resp.get("result")
            .cloned()
            .ok_or_else(|| format!("Unexpected aria2 response (no result): {}", resp))
    }

    /// Call and extract a string result (e.g. GID).
    pub async fn call_string(&self, method: &str, params: Vec<Value>) -> Result<String, String> {
        let result = self.call_result(method, params).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Expected string result, got: {}", result))
    }

    pub async fn get_version(&self) -> Result<Value, String> {
        self.call_result("aria2.getVersion", vec![]).await
    }

    pub async fn tell_active(&self) -> Result<Vec<Value>, String> {
        let result = self.call_result("aria2.tellActive", vec![]).await?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    pub async fn tell_waiting(&self, offset: i64, num: i64) -> Result<Vec<Value>, String> {
        let result = self
            .call_result("aria2.tellWaiting", vec![json!(offset), json!(num)])
            .await?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    /// Minimal `tellActive` that only fetches GIDs, keeping the payload small
    /// when applying an option to every running download.
    pub async fn tell_active_gids(&self) -> Result<Vec<String>, String> {
        let result = self
            .call_result("aria2.tellActive", vec![json!(["gid"])])
            .await?;
        Ok(result
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d.get("gid").and_then(|g| g.as_str()).map(String::from))
            .collect())
    }

    /// Minimal `tellWaiting` that only fetches GIDs for a queue page.
    pub async fn tell_waiting_gids(&self, offset: i64, num: i64) -> Result<Vec<String>, String> {
        let result = self
            .call_result(
                "aria2.tellWaiting",
                vec![json!(offset), json!(num), json!(["gid"])],
            )
            .await?;
        Ok(result
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d.get("gid").and_then(|g| g.as_str()).map(String::from))
            .collect())
    }

    pub async fn tell_stopped(&self, offset: i64, num: i64) -> Result<Vec<Value>, String> {
        let result = self
            .call_result("aria2.tellStopped", vec![json!(offset), json!(num)])
            .await?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    pub async fn tell_status(&self, gid: &str) -> Result<Value, String> {
        self.call_result("aria2.tellStatus", vec![json!(gid)]).await
    }

    pub async fn add_uri(&self, uris: &[String], options: Map<String, Value>) -> Result<String, String> {
        self.call_string(
            "aria2.addUri",
            vec![json!(uris), Value::Object(options)],
        )
        .await
    }

    /// Add a BitTorrent download from base64-encoded .torrent content.
    /// `uris` may contain additional tracker URLs (typically empty).
    pub async fn add_torrent(&self, torrent_b64: &str, uris: &[String], options: Map<String, Value>) -> Result<String, String> {
        self.call_string(
            "aria2.addTorrent",
            vec![json!(torrent_b64), json!(uris), Value::Object(options)],
        )
        .await
    }

    pub async fn pause(&self, gid: &str) -> Result<String, String> {
        self.call_string("aria2.pause", vec![json!(gid)]).await
    }

    pub async fn unpause(&self, gid: &str) -> Result<String, String> {
        self.call_string("aria2.unpause", vec![json!(gid)]).await
    }

    pub async fn remove(&self, gid: &str) -> Result<Value, String> {
        self.call_result("aria2.remove", vec![json!(gid)]).await
    }

    pub async fn remove_download_result(&self, gid: &str) -> Result<Value, String> {
        self.call_result("aria2.removeDownloadResult", vec![json!(gid)])
            .await
    }

    pub async fn change_option(&self, gid: &str, options: Map<String, Value>) -> Result<(), String> {
        let _ = self
            .call_result(
                "aria2.changeOption",
                vec![json!(gid), Value::Object(options)],
            )
            .await?;
        Ok(())
    }

    pub async fn change_global_option(&self, options: Map<String, Value>) -> Result<(), String> {
        let _ = self
            .call_result("aria2.changeGlobalOption", vec![Value::Object(options)])
            .await?;
        Ok(())
    }

    /// Apply the same options to active and queued downloads. Waiting entries
    /// (including paused ones) are included so a limit change takes effect
    /// before those downloads start.
    pub async fn change_option_on_all(
        &self,
        options: Map<String, Value>,
    ) -> Result<(usize, usize), String> {
        let (active, waiting) = futures_util::future::join(
            self.tell_active_gids(),
            self.tell_waiting_gids(0, 1000),
        )
        .await;
        let mut gids = active?;
        gids.extend(waiting?);

        let results: Vec<Result<(), String>> = futures_util::stream::iter(
            gids.into_iter().map(|gid| {
                let options = options.clone();
                async move { self.change_option(&gid, options).await }
            }),
        )
        .buffer_unordered(8)
        .collect()
        .await;

        let applied = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.len() - applied;
        Ok((applied, failed))
    }
}
