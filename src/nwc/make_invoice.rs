use std::sync::Arc;

use hex::FromHex;
use log::info;
use nostr::nips::nip47::{Method, RequestParams, Response};
use tokio::sync::Mutex;

use crate::AppState;

async fn handle_nwc_make_invoice(
    params: RequestParams,
    state: Arc<Mutex<AppState>>,
    method: Method,
) -> anyhow::Result<Response> {
    let description_hash: Vec<u8> = match params.description_hash {
        None => vec![],
        Some(str) => FromHex::from_hex(&str)?,
    };
    let inv = Invoice {
        memo: params.description.unwrap_or_default(),
        description_hash,
        value_msat: params.amount as i64,
        expiry: params.expiry.unwrap_or(86_400) as i64,
        private: config.route_hints,
        ..Default::default()
    };
    let res = lnd.add_invoice(inv).await?.into_inner();

    info!("Created invoice: {}", res.payment_request);

    Response {
        result_type: Method::MakeInvoice,
        error: None,
        result: Some(ResponseResult::MakeInvoice(MakeInvoiceResponseResult {
            invoice: res.payment_request,
            payment_hash: ::hex::encode(res.r_hash),
        })),
    }
}
