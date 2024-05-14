use std::sync::Arc;

use anyhow::anyhow;
use nostr::nips::nip04;
use nostr::nips::nip47::{Method, RequestParams};
use nostr_sdk::{Event, EventBuilder, Kind, Tag};
use tokio::sync::Mutex;

use crate::AppState;

mod get_balance;
mod get_info;
mod lookup_invoice;
mod make_invoice;
mod pay_invoice;
mod pay_keysend;

pub async fn handle_nwc(
    params: RequestParams,
    method: Method,
    event: &Event,
    state: Arc<Mutex<AppState>>,
) -> anyhow::Result<()> {
    let mut d_tag: Option<Tag> = None;
    let content = match params {
        RequestParams::PayInvoice(params) => {
            pay_invoice::handle_nwc_pay_invoice(params, state, tracker, multimint_client, method)
                .await
        }
        RequestParams::PayKeysend(params) => {
            pay_keysend::handle_nwc_pay_keysend(params, state, tracker, multimint_client, method)
                .await
        }
        RequestParams::MakeInvoice(params) => {
            make_invoice::handle_nwc_make_invoice(params, state, method).await
        }
        RequestParams::LookupInvoice(params) => {
            lookup_invoice::handle_nwc_lookup_invoice(params, state, method).await
        }
        RequestParams::GetBalance => get_balance::handle_nwc_get_balance(state, method).await,
        RequestParams::GetInfo => get_info::handle_nwc_get_info(state, method).await,
        _ => {
            return Err(anyhow!("Command not supported"));
        }
    };

    let encrypted = nip04::encrypt(
        &keys.server_key.into(),
        &keys.user_keys().public_key(),
        content.as_json(),
    )?;
    let p_tag = Tag::public_key(event.pubkey);
    let e_tag = Tag::event(event.id);
    let tags = match d_tag {
        None => vec![p_tag, e_tag],
        Some(d_tag) => vec![p_tag, e_tag, d_tag],
    };
    let response = EventBuilder::new(Kind::WalletConnectResponse, encrypted, tags)
        .to_event(&keys.server_keys())?;

    client.send_event(response).await?;

    Ok(())
}
