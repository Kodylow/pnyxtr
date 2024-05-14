use std::sync::Arc;

use log::info;
use nostr::nips::nip47::{Method, Response};
use tokio::sync::Mutex;

use crate::AppState;

pub async fn handle_nwc_get_info(
    state: Arc<Mutex<AppState>>,
    method: Method,
) -> anyhow::Result<Response> {
    let lnd_info: GetInfoResponse = lnd.get_info(GetInfoRequest {}).await?.into_inner();
    info!("Getting info");
    Response {
        result_type: Method::GetBalance,
        error: None,
        result: Some(ResponseResult::GetInfo(GetInfoResponseResult {
            alias: lnd_info.alias,
            color: lnd_info.color,
            pubkey: lnd_info.identity_pubkey,
            network: "".to_string(),
            block_height: lnd_info.block_height,
            block_hash: lnd_info.block_hash,
            methods: METHODS.iter().map(|i| i.to_string()).collect(),
        })),
    }
}
