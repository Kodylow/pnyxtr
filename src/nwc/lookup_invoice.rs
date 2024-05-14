use std::sync::Arc;

use log::info;
use nostr::nips::nip47::{Method, RequestParams, Response};
use tokio::sync::Mutex;

use crate::AppState;

async fn handle_nwc_lookup_invoice(
    params: RequestParams,
    state: Arc<Mutex<AppState>>,
    method: Method,
) -> anyhow::Result<Response> {
    let mut invoice: Option<Bolt11Invoice> = None;
    let payment_hash: Vec<u8> = match params.payment_hash {
        None => match params.invoice {
            None => return Err(anyhow!("Missing payment_hash or invoice")),
            Some(bolt11) => {
                let inv = Bolt11Invoice::from_str(&bolt11)
                    .map_err(|_| anyhow!("Failed to parse invoice"))?;
                invoice = Some(inv.clone());
                inv.payment_hash().into_32().to_vec()
            }
        },
        Some(str) => FromHex::from_hex(&str)?,
    };

    let res = lnd
        .lookup_invoice(PaymentHash {
            r_hash: payment_hash.clone(),
            ..Default::default()
        })
        .await?
        .into_inner();

    info!("Looked up invoice: {}", res.payment_request);

    let (description, description_hash) = match invoice {
        Some(inv) => match inv.description() {
            Bolt11InvoiceDescription::Direct(desc) => (Some(desc.to_string()), None),
            Bolt11InvoiceDescription::Hash(hash) => (None, Some(hash.0.to_string())),
        },
        None => (None, None),
    };

    let preimage = if res.r_preimage.is_empty() {
        None
    } else {
        Some(hex::encode(res.r_preimage))
    };

    let settled_at = if res.settle_date == 0 {
        None
    } else {
        Some(res.settle_date as u64)
    };

    Response {
        result_type: Method::LookupInvoice,
        error: None,
        result: Some(ResponseResult::LookupInvoice(LookupInvoiceResponseResult {
            transaction_type: None,
            invoice: Some(res.payment_request),
            description,
            description_hash,
            preimage,
            payment_hash: hex::encode(payment_hash),
            amount: res.value_msat as u64,
            fees_paid: 0,
            created_at: res.creation_date as u64,
            expires_at: (res.creation_date + res.expiry) as u64,
            settled_at,
            metadata: Default::default(),
        })),
    }
}
