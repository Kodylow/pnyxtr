use std::sync::Arc;

use log::info;
use nostr::nips::nip47::{Method, Response};
use tokio::sync::Mutex;

use crate::AppState;

pub async fn handle_nwc_get_balance(
    state: Arc<Mutex<AppState>>,
    method: Method,
) -> anyhow::Result<Response> {
    let tracker = tracker.lock().await.sum_payments();
    let remaining_msats = config.daily_limit * 1_000 - tracker;
    info!("Current balance: {remaining_msats}msats");
    Response {
        result_type: Method::GetBalance,
        error: None,
        result: Some(ResponseResult::GetBalance(GetBalanceResponseResult {
            balance: remaining_msats,
        })),
    }
}
