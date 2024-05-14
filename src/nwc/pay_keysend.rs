use std::sync::Arc;

use log::info;
use multimint::MultiMint;
use nostr::nips::nip47::{Method, RequestParams, Response};
use nostr_sdk::Tag;
use tokio::sync::Mutex;

use crate::payments::PaymentTracker;
use crate::AppState;

async fn handle_nwc_pay_keysend(
    params: RequestParams,
    state: Arc<Mutex<AppState>>,
    tracker: Arc<Mutex<PaymentTracker>>,
    multimint_client: MultiMint,
    method: Method,
) -> anyhow::Result<Response> {
    d_tag = params.id.map(Tag::Identifier);

    let msats = params.amount;
    let error_msg = if config.max_amount > 0 && msats > config.max_amount * 1_000 {
        Some("Invoice amount too high.")
    } else if config.daily_limit > 0
        && tracker.lock().await.sum_payments() + msats > config.daily_limit * 1_000
    {
        Some("Daily limit exceeded.")
    } else {
        None
    };

    // verify amount, convert to msats
    match error_msg {
        None => {
            let pubkey = bitcoin::secp256k1::PublicKey::from_str(&params.pubkey)?;
            match pay_keysend(
                pubkey,
                params.preimage,
                params.tlv_records,
                msats,
                multimint_client,
                method,
            )
            .await
            {
                Ok(content) => {
                    // add payment to tracker
                    tracker.lock().await.add_payment(msats);
                    content
                }
                Err(e) => {
                    error!("Error paying keysend: {e}");

                    Response {
                        result_type: method,
                        error: Some(NIP47Error {
                            code: ErrorCode::PaymentFailed,
                            message: format!("Failed to pay keysend: {e}"),
                        }),
                        result: None,
                    }
                }
            }
        }
        Some(err_msg) => Response {
            result_type: method,
            error: Some(NIP47Error {
                code: ErrorCode::QuotaExceeded,
                message: err_msg.to_string(),
            }),
            result: None,
        },
    }
}
