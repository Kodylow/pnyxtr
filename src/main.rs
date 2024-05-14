#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use keys::Nip47Keys;
use log::{debug, error, info};
use multimint::MultiMint;
use nostr::nips::nip04;
use nostr::nips::nip47::*;
use nostr::{Event, EventBuilder, EventId, Filter, JsonUtil, Kind, Timestamp};
use nostr_sdk::{Client, RelayPoolNotification};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{oneshot, Mutex};
use tokio::{select, spawn};

use crate::config::Config;

mod config;
mod keys;
mod nwc;
mod payments;

struct AppState {
    keys: Nip47Keys,
    multimint_client: MultiMint,
    nostr_client: Client,
    active_requests: HashSet<EventId>,
    config: Config,
}

const METHODS: [Method; 8] = [
    Method::GetInfo,
    Method::MakeInvoice,
    Method::GetBalance,
    Method::LookupInvoice,
    Method::PayInvoice,
    Method::MultiPayInvoice,
    Method::PayKeysend,
    Method::MultiPayKeysend,
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::try_init()?;
    let config: Config = Config::parse();
    let keys = keys::Nip47Keys::load_or_generate_keys(PathBuf::from(&config.keys_file))?;

    let multimint_client = MultiMint::new(PathBuf::from(&config.data_dir)).await?;
    let nostr_client = Client::new(&keys.server_keys());

    let state = Arc::new(Mutex::new(AppState {
        keys,
        multimint_client,
        nostr_client,
        active_requests: HashSet::new(),
        config,
    }));

    // Set up a oneshot channel to handle shutdown signal
    let (tx, rx) = oneshot::channel();

    // Clone the Arc to pass into the shutdown listener and event loop
    let state_for_signals = state.clone();
    let state_for_event_loop = state.clone();

    spawn(async move {
        let mut term_signal = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(e) => {
                error!("failed to install TERM signal handler: {e}");
                return;
            }
        };
        let mut int_signal = match signal(SignalKind::interrupt()) {
            Ok(signal) => signal,
            Err(e) => {
                error!("failed to install INT signal handler: {e}");
                return;
            }
        };

        select! {
            _ = term_signal.recv() => {
                debug!("Received SIGTERM");
            },
            _ = int_signal.recv() => {
                debug!("Received SIGINT");
            },
        }

        let _ = tx.send(());
    });

    spawn(async move {
        if let Err(e) = event_loop(state_for_event_loop).await {
            error!("Error: {e}");
        }
    });

    rx.await?;

    info!("Shutting down...");
    // Ensure all active requests are completed
    let mut shared = state_for_signals.lock().await;
    while !shared.active_requests.is_empty() {
        drop(shared);
        debug!("Waiting for active requests to complete...");
        tokio::time::sleep(Duration::from_secs(1)).await;
        shared = state_for_signals.lock().await;
    }

    Ok(())
}

async fn event_loop(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    // loop in case we get disconnected
    loop {
        let state_clone = state.clone();
        let keys = &state_clone.lock().await.keys;
        let client = Client::new(&keys.server_keys());
        client
            .add_relay(state.lock().await.config.relay.as_str())
            .await?;

        client.connect().await;

        // broadcast info event
        if !keys.sent_info {
            let content: String = METHODS
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let info = EventBuilder::new(Kind::WalletConnectInfo, content, [])
                .to_event(&keys.server_keys())?;
            client.send_event(info).await?;

            state.lock().await.keys.sent_info = true;
            keys.write_keys(&PathBuf::from(&state.lock().await.config.keys_file));
        }

        let subscription = Filter::new()
            .kinds(vec![Kind::WalletConnectRequest])
            .author(keys.user_keys().public_key())
            .pubkey(keys.server_keys().public_key())
            .since(Timestamp::now());

        client.subscribe(vec![subscription], None).await;

        info!("Listening for nip 47 requests...");

        let (tx, mut rx) = tokio::sync::watch::channel(());
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(60 * 15)).await;
            tx.send_modify(|_| ())
        });

        let mut notifications = client.notifications();
        loop {
            select! {
                Ok(notification) = notifications.recv() => {
                    match notification {
                        RelayPoolNotification::Event { event, .. } => {
                            if event.kind == Kind::WalletConnectRequest
                                && event.pubkey == keys.user_keys().public_key()
                                && event.verify().is_ok()
                            {
                                debug!("Received event!");
                                spawn(async move {
                                    let event_id = event.id;
                                    state.lock().await.active_requests.insert(event_id);

                                    match tokio::time::timeout(
                                        Duration::from_secs(60),
                                        handle_nwc_request(*event, state.clone()),
                                    )
                                    .await
                                    {
                                        Ok(Ok(_)) => {},
                                        Ok(Err(e)) => error!("Error processing request: {e}"),
                                        Err(e) => error!("Timeout error: {e}"),
                                    }

                                    // remove request from active requests
                                    state.lock().await.active_requests.remove(&event_id);
                                });
                            } else {
                                error!("Invalid event: {}", event.as_json());
                            }
                        }
                        RelayPoolNotification::Shutdown => {
                            info!("Relay pool shutdown");
                            break;
                        }
                        _ => {}
                    }
                }
                _ = rx.changed() => {
                    break;
                }
            }
        }

        client.disconnect().await?;
    }
}

async fn handle_nwc_request(event: Event, state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    let keys = state.lock().await.keys.clone();
    let server_keys = keys.server_keys();
    let secret_key = server_keys.secret_key()?;
    let decrypted = nip04::decrypt(&secret_key, &keys.user_keys().public_key(), &event.content)?;
    let req: Request = Request::from_json(&decrypted)?;

    debug!("Request params: {:?}", req.params);

    // split up the multis into their parts
    match req.params {
        RequestParams::MultiPayInvoice(params) => {
            for inv in params.invoices {
                let params = RequestParams::PayInvoice(inv);
                let event = event.clone();
                let state = state.clone();
                spawn(async move { nwc::handle_nwc(params, req.method, &event, state).await })
                    .await??;
            }

            Ok(())
        }
        RequestParams::MultiPayKeysend(params) => {
            for inv in params.keysends {
                let params = RequestParams::PayKeysend(inv);
                let event = event.clone();
                let state = state.clone();
                spawn(async move { nwc::handle_nwc(params, req.method, &event, state).await })
                    .await??;
            }

            Ok(())
        }
        params => nwc::handle_nwc(params, req.method, &event, state).await,
    }
}
