use affinidi_tdk::{
    TDK,
    common::config::TDKConfig,
    didcomm::Message,
    messaging::{
        ATM, config::ATMConfig, profiles::ATMProfile, transports::websockets::WebSocketResponses,
    },
};
/// Robotic auto-responders for maintainers
/// You will need to create a TDK Environments file to hold the identity information for the
/// robotic maintainers
use std::{collections::HashMap, env};

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use clap::Parser;
use lkmv::{
    MessageType,
    relationships::{
        RelationshipRequestBody, create_send_message_accepted, create_send_message_rejected,
    },
};
use tokio::select;
use tracing::{info, warn};
use tracing_subscriber::filter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Environment to use
    #[arg(short, long)]
    environment: Option<String>,

    /// Path to the environments file (defaults to environments.json)
    #[arg(short, long)]
    path_environments: Option<String>,
}

struct Relationship {
    created: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = Args::parse();

    let environment_name = if let Some(environment_name) = &args.environment {
        environment_name.to_string()
    } else if let Ok(environment_name) = env::var("TDK_ENVIRONMENT") {
        environment_name
    } else {
        "default".to_string()
    };

    println!("Using Environment: {}", environment_name);

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .with_env_filter(filter::EnvFilter::from_default_env())
        .finish();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).expect("Logging failed, exiting...");

    // Instantiate TDK
    let tdk = TDK::new(
        TDKConfig::builder()
            .with_environment_name(environment_name.clone())
            .with_use_atm(false)
            .build()?,
        None,
    )
    .await?;

    // Custom Trusted Messaging interface, where all messsages for all profiles will come in on a
    // single channel

    let atm = ATM::new(
        ATMConfig::builder()
            .with_inbound_message_channel(10)
            .build()
            .unwrap(),
        tdk.get_shared_state(),
    )
    .await?;

    let environment = &tdk.get_shared_state().environment;
    let Some(mut inbound_channel) = atm.get_inbound_channel() else {
        bail!("Couldn't get ATM aggregated inbound channel");
    };

    let Some(mediator_did) = &environment.default_mediator else {
        println!("There is no default mediator set in the TDK environment configuration!");
        bail!("No default mediator!");
    };

    // Activate Ada Profile
    let tdk_ada = if let Some(ada) = environment.profiles.get("Ada") {
        tdk.add_profile(ada).await;
        ada
    } else {
        bail!("Ada not found in Environment: {}", environment_name);
    };

    let atm_ada = atm
        .profile_add(&ATMProfile::from_tdk_profile(&atm, tdk_ada).await?, true)
        .await?;
    info!("{} profile loaded", atm_ada.inner.alias);

    // Create an in-memory cache of relationships for incoming requests
    let mut relationships: HashMap<String, Relationship> = HashMap::new();

    info!("Main loop running...");
    loop {
        select! {
            // Listen for inbound messages for all profiles
            Ok(WebSocketResponses::MessageReceived(inbound_message, _)) = inbound_channel.recv() => {
                handle_message( &atm, mediator_did, &inbound_message, &mut relationships).await;
            }

        }
    }
}

// Handles an inbound message for all profiles
async fn handle_message(
    atm: &ATM,
    mediator: &str,
    message: &Message,
    relationships: &mut HashMap<String, Relationship>,
) {
    if message.type_ == "https://didcomm.org/messagepickup/3.0/status" {
        // Status message, ignore
        return;
    }

    let to_profile = if let Some(to) = &message.to
        && let Some(first) = to.first()
        && let Some(profile) = atm.find_profile(first).await
    {
        profile
    } else {
        warn!("Invalid message to: address received: {:#?}", message.to);
        return;
    };

    let from_did = if let Some(from) = &message.from {
        from.to_string()
    } else {
        warn!(
            "{}: Message receieved had no from: address! Ignoring...",
            to_profile.inner.alias
        );
        return;
    };

    if let Ok(msg_type) = MessageType::try_from(message) {
        match msg_type {
            MessageType::RelationshipRequest => {
                // Inbound relationship request
                let body: RelationshipRequestBody =
                    match serde_json::from_value(message.body.clone()) {
                        Ok(b) => b,
                        Err(e) => {
                            warn!(
                                "{}: Couldn't serialize relationship request body: {e}",
                                to_profile.inner.alias
                            );
                            return;
                        }
                    };

                if body.did != from_did {
                    // Requestor is asking for a relationship-did wrapped channel which we don't
                    // support

                    match create_send_message_rejected(atm, &to_profile, &from_did, mediator, Some(&format!("Sorry, {} doesn't accept r-did based relationships. Only Persona-DID level relationships are allowed!", &to_profile.inner.alias)), &message.id).await {
                        Ok(_) => info!("{}: Rejected a relationship due to using r-dids. Remote: {}", to_profile.inner.alias, &from_did),
                        Err(e) => warn!("{}: Couldn't send a relationship rejection message: {}", to_profile.inner.alias, e),
                    }
                } else {
                    // Accept and send a relationship request accept message
                    match create_send_message_accepted(
                        atm,
                        &to_profile,
                        &from_did,
                        mediator,
                        &to_profile.inner.did,
                        &message.id,
                    )
                    .await
                    {
                        Ok(_) => info!(
                            "{}: Accepted a relationship from: {}",
                            to_profile.inner.alias, &from_did
                        ),
                        Err(e) => warn!(
                            "{}: Couldn't send a relationship accept message: {}",
                            to_profile.inner.alias, e
                        ),
                    }

                    relationships.insert(
                        from_did,
                        Relationship {
                            created: Utc::now(),
                        },
                    );
                }
            }
            MessageType::RelationshipRequestFinalize => {
                info!(
                    "{}: Relationship setup fully completed with: {}",
                    &to_profile.inner.alias, &from_did
                );
            }
            _ => {
                // Is a message type that we are not interested in. Can safely ignore
                warn!(
                    "{}: Unknown Message: {:#?}",
                    to_profile.inner.alias, message
                );
            }
        }
    }
}
