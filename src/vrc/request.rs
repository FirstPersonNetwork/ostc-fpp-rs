use std::{rc::Rc, sync::Mutex};

use crate::{
    CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_PURPLE, CLI_WHITE,
    config::Config,
    log::LogFamily,
    relationships::Relationship,
    tasks::Task,
    vrc::{CredentialSubject, FromSubject, ToSubject, VRCRequestReject, Vrc, VrcRequest},
};
use affinidi_data_integrity::DataIntegrityProof;
use affinidi_tdk::{TDK, didcomm::PackEncryptedOptions};
use anyhow::Result;
use chrono::{Local, prelude::*};
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

/// Handles the menu for an interactive Inbound VRC Request
pub async fn interact_vrc_inbound_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    request: &VrcRequest,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    // Show details of the VRC Request
    println!();
    let (from, from_c_did, to) = {
        let lock = relationship.lock().unwrap();
        (
            lock.remote_did.clone(),
            lock.remote_c_did.clone(),
            lock.our_did.clone(),
        )
    };

    let task_id = { task.lock().unwrap().id.clone() };

    let alias = if let Some(contact) = config.private.contacts.find_contact(&from_c_did)
        && let Some(alias) = &contact.alias
    {
        style(alias.to_string()).color256(CLI_GREEN)
    } else {
        style("NO ALIAS".to_string()).color256(CLI_ORANGE)
    };

    println!(
        "{}{} {}{}",
        style("From: alias: ").color256(CLI_BLUE),
        alias,
        style(" C-DID: ").color256(CLI_BLUE),
        style(&from_c_did).color256(CLI_PURPLE)
    );
    println!(
        "{}{}",
        style("To: ").color256(CLI_BLUE),
        style(&to).color256(CLI_PURPLE)
    );

    request.print();
    println!();

    match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Task Action?")
        .item("Accept this VRC request")
        .item("Reject this VRC request")
        .item("Delete this VRC request (Does not notify the other party)")
        .item("Return to previous menu?")
        .interact()?
    {
        0 => {
            // Accept the VRC Request
            Ok(handle_accept_vrcs_request(tdk, config, task, request, relationship).await?)
        }
        1 => {
            // Reject the VRC Request
            let reason: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Would you like to provide a reason for this rejection (Leave BLANK for None)?",
                )
                .allow_empty(true)
                .interact_text()
                .unwrap();

            let reason = if reason.trim().is_empty() {
                None
            } else {
                Some(reason.trim().to_string())
            };

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to reject this VRC request?")
                .default(true)
                .interact()?
            {
                let msg = VRCRequestReject::create_message(&to, &from, &task_id, reason.clone())?;

                // Pack the message
                let (msg, _) = msg
                    .pack_encrypted(
                        &to,
                        Some(&from),
                        Some(&from),
                        tdk.did_resolver(),
                        &tdk.get_shared_state().secrets_resolver,
                        &PackEncryptedOptions {
                            forward: false,
                            ..Default::default()
                        },
                    )
                    .await?;

                let atm = tdk.atm.clone().unwrap();
                atm.forward_and_send_message(
                    &config.community_did.profile,
                    false,
                    &msg,
                    None,
                    &config.public.mediator_did,
                    to.as_str(),
                    None,
                    None,
                    false,
                )
                .await?;

                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Rejected VRC request from remote DID({}) Task ID({}) Reason: {}",
                        from,
                        task_id,
                        reason.as_deref().unwrap_or("NO REASON PROVIDED")
                    ),
                );

                println!();
                println!(
                    "{}{}",
                    style("✅ Succesfully sent VRC Request Rejection to ").color256(CLI_GREEN),
                    style(to).color256(CLI_PURPLE)
                );

                Ok(true)
            } else {
                // Cancel rejection
                Ok(false)
            }
        }
        2 => {
            // Delete the VRC Request
            println!("{}", style("When you delete a VRC request, no response is sent back to the initiator of the request. Deleting acts as a silent ignore...").color256(CLI_BLUE));
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to DELETE this VRC request?")
                .default(false)
                .interact()?
            {
                config.private.tasks.remove(&task_id);
                config.public.logs.insert(
                    LogFamily::Task,
                    format!(
                        "Deleted VRC request from remote DID({}) Task ID({})",
                        from_c_did, task_id
                    ),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        3 => Ok(false),

        _ => Ok(false),
    }
}

/// Interactive menu for generating a VRC Response
pub async fn handle_accept_vrcs_request(
    tdk: &TDK,
    config: &mut Config,
    task: &Rc<Mutex<Task>>,
    request: &VrcRequest,
    relationship: &Rc<Mutex<Relationship>>,
) -> Result<bool> {
    // Start collecting data for VRC Response
    let (our_r_did, their_c_did, their_r_did, r_created) = {
        let lock = relationship.lock().unwrap();
        (
            lock.our_did.clone(),
            lock.remote_c_did.clone(),
            lock.remote_did.clone(),
            lock.created,
        )
    };
    let task_id = { task.lock().unwrap().id.clone() };

    println!();
    println!(
        "{}",
        style("Our information")
            .color256(CLI_BLUE)
            .underlined()
            .bold()
    );
    println!();

    println!(
        "{} {}",
        style("Your human readable name is:").color256(CLI_BLUE),
        style(&config.public.friendly_name).color256(CLI_GREEN)
    );
    let from_name = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to change the human-readable name for yourself in this VRC?")
        .default(false)
        .interact()?
    {
        let from_name:String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter a human-readable name for yourself of the VRC (leave blank for no issuer name)")
        .interact_text()
        .unwrap();

        if from_name.trim().is_empty() {
            None
        } else {
            Some(from_name.trim().to_string())
        }
    } else {
        Some(config.public.friendly_name.to_string())
    };

    let our_also_known_as = if our_r_did != config.public.community_did {
        println!(
            "{}{}",
            style("This relationship is using private Relationship DIDs (R-DID). R-DID: ")
                .color256(CLI_BLUE),
            style(&our_r_did).color256(CLI_PURPLE)
        );
        println!(
            "{}{}{}",
            style("Typically it is ")
                .color256(CLI_BLUE),
            style("NOT RECOMMENDED")
                .color256(CLI_ORANGE).bold(),
            style(" to include the R-DID in alsoKnownAs as this is your private communication channel!")
                .color256(CLI_BLUE),
        );
        let ask_default = if request.include_r_did {
            println!(
                "{} {}",
                style("Has the requestor requested to include their R-DID?").color256(CLI_BLUE),
                style("YES").color256(CLI_GREEN)
            );
            true
        } else {
            println!(
                "{} {}",
                style("Has the requestor requested to include their R-DID?").color256(CLI_BLUE),
                style("NO").color256(CLI_ORANGE)
            );
            false
        };

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include your R-DID in your alsoKnownAs?")
            .default(ask_default)
            .interact()?
        {
            println!(
                "{}{}",
                style("You are including your R-DID in alsoKnownAs: ").color256(CLI_BLUE),
                style(&our_r_did).color256(CLI_PURPLE)
            );
            vec![our_r_did.to_string()]
        } else {
            println!(
                "{}",
                style("You are NOT including your R-DID in alsoKnownAs").color256(CLI_BLUE)
            );
            vec![]
        }
    } else {
        vec![]
    };

    println!();
    println!(
        "{}",
        style("Their information")
            .color256(CLI_BLUE)
            .underlined()
            .bold()
    );
    println!();

    let their_name = if let Some(name) = &request.name {
        println!("{}{}", style("The requestor has suggested a name to use for themselves, do you want to use this? Name: ").color256(CLI_BLUE), style(name).color256(CLI_ORANGE));
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Use the requestor's suggested name?")
            .default(true)
            .interact()?
        {
            println!(
                "{}{}",
                style("Using the requestor's suggested name: ").color256(CLI_BLUE),
                style(name).color256(CLI_ORANGE)
            );
            Some(name.to_string())
        } else {
            let name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Enter a human-readable name for the requestor (leave blank for no name)",
                )
                .allow_empty(true)
                .interact_text()
                .unwrap();
            if name.trim().is_empty() {
                println!(
                    "{}",
                    style("No name will be included for the requestor").color256(CLI_BLUE)
                );
                None
            } else {
                Some(name.trim().to_string())
            }
        }
    } else {
        let name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter a human-readable name for the requestor (leave blank for no name)")
            .allow_empty(true)
            .interact_text()
            .unwrap();
        if name.trim().is_empty() {
            println!(
                "{}",
                style("No name will be included for the requestor").color256(CLI_BLUE)
            );
            None
        } else {
            Some(name.trim().to_string())
        }
    };

    let to_also_known_as = if their_r_did != their_c_did {
        if request.include_r_did {
            println!(
                "{}{}",
                style("The requestor has requested to include their R-DID in alsoKnownAs: ")
                    .color256(CLI_BLUE),
                style(&their_r_did).color256(CLI_PURPLE)
            );
        } else {
            println!(
                "{}",
                style("The requestor has NOT requested to include their R-DID in alsoKnownAs")
                    .color256(CLI_BLUE)
            );
        }
        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include the requestor's R-DID in their alsoKnownAs?")
            .default(request.include_r_did)
            .interact()?
        {
            println!(
                "{}{}",
                style("Including the requestor's R-DID in alsoKnownAs: ").color256(CLI_BLUE),
                style(&their_r_did).color256(CLI_PURPLE)
            );
            Some(vec![their_r_did.to_string()])
        } else {
            println!(
                "{}",
                style("Not including the requestor's R-DID in alsoKnownAs").color256(CLI_BLUE)
            );
            None
        }
    } else {
        // No aliasing needed
        None
    };

    println!();
    println!(
        "{}",
        style("VRC Configuration")
            .color256(CLI_BLUE)
            .underlined()
            .bold()
    );
    println!();

    if let Some(reason) = &request.reason {
        println!(
            "{} {}",
            style("The VRC Request provided the following reason:").color256(CLI_BLUE),
            style(reason).color256(CLI_PURPLE)
        );
    }

    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to include a description for this VRC (leave blank for no)?")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let description = if description.trim().is_empty() {
        println!(
            "{}",
            style("No description will be included in the VRC").color256(CLI_BLUE)
        );
        None
    } else {
        Some(description.trim().to_string())
    };

    println!("{}", style("It can be useful to have a human readable name for the VRC to help others understand the purpose or reason for this VRC").color256(CLI_BLUE));
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(
            "Would you like to include a human-readable name for this VRC (leave blank for no)?",
        )
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let name = if name.trim().is_empty() {
        println!(
            "{}",
            style("No name will be included in the VRC").color256(CLI_BLUE)
        );
        None
    } else {
        Some(name.trim().to_string())
    };

    // Set the relationshipType attribute
    let mut items = vec![
        "Do not include the relationshipType attribute".to_string(),
        "Set relationshipType to a custom value".to_string(),
    ];

    if let Some(type_) = &request.type_ {
        items.push(["Use the requestor's suggested relationshipType: ", type_].concat());
    }
    let relationship_type = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("How would you like to set the relationshipType attribute for this VRC?")
        .items(items)
        .interact()?
    {
        0 => None,
        1 => {
            let custom_relationship_type: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom relationshipType value")
                .interact_text()
                .unwrap();
            Some(custom_relationship_type.trim().to_string())
        }
        2 => {
            println!(
                "{}{}",
                style("Using the requestor's suggested relationshipType: ").color256(CLI_BLUE),
                style(request.type_.as_deref().unwrap()).color256(CLI_PURPLE)
            );
            Some(request.type_.as_deref().unwrap().to_string())
        }
        _ => None,
    };

    let valid_from = if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Should the issued VRC be valid from now?")
        .default(true)
        .interact()?
    {
        Local::now()
    } else {
        let now = Local::now();
        println!(
            "{}",
            style("Timestamp format must be in ISO 8601 Format!").color256(CLI_BLUE)
        );
        let custom_valid_from: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter custom valid from date-time (ISO 8601)")
            .default(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .validate_with(|input: &String| -> Result<(), &str> {
                if DateTime::parse_from_rfc3339(input).is_ok() {
                    Ok(())
                } else {
                    Err("Invalid date-time format. Please use ISO 8601 format.")
                }
            })
            .interact_text()
            .unwrap();

        custom_valid_from.parse().unwrap()
    };

    println!(
        "{} {}",
        style("Did the requestor want to include a relationship start date?").color256(CLI_BLUE),
        if request.start_date {
            style("YES").color256(CLI_GREEN)
        } else {
            style("NO").color256(CLI_ORANGE)
        }
    );
    let r_start_date_str = r_created
        .with_timezone(&Local)
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    println!(
        "{} {}",
        style("Relationship Start Date: ").color256(CLI_BLUE),
        style(&r_start_date_str).color256(CLI_GREEN)
    );
    let start_date = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to set a start date for the relationship?")
        .item(format!(
            "Set start date to relationship start date: {}",
            &r_start_date_str
        ))
        .item("Set start date to now")
        .item("Set custom start date")
        .item("No start date")
        .default(0)
        .interact()?
    {
        0 => Some(r_created),
        1 => Some(Utc::now()),
        2 => {
            println!(
                "{}",
                style("Timestamp format must be in ISO 8601 Format!").color256(CLI_BLUE)
            );
            let custom_start_date: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom start date (ISO 8601)")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if DateTime::parse_from_rfc3339(input).is_ok() {
                        Ok(())
                    } else {
                        Err("Invalid date-time format. Please use ISO 8601 format.")
                    }
                })
                .interact_text()
                .unwrap();
            Some(
                DateTime::parse_from_rfc3339(&custom_start_date)
                    .unwrap()
                    .to_utc(),
            )
        }
        _ => None,
    };

    println!(
        "{} {}",
        style("Did the requestor want to include a relationship end date?").color256(CLI_BLUE),
        if request.end_date {
            style("YES").color256(CLI_GREEN)
        } else {
            style("NO").color256(CLI_ORANGE)
        }
    );
    let end_date = match Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to set an end date for the relationship?")
        .item("Set end date to now")
        .item("Set custom end date")
        .item("No end date")
        .interact()?
    {
        0 => Some(Utc::now()),
        1 => {
            println!(
                "{}",
                style("Timestamp format must be in ISO 8601 Format!").color256(CLI_BLUE)
            );
            let custom_end_date: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom end date (ISO 8601)")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if DateTime::parse_from_rfc3339(input).is_ok() {
                        Ok(())
                    } else {
                        Err("Invalid date-time format. Please use ISO 8601 format.")
                    }
                })
                .interact_text()
                .unwrap();
            Some(
                DateTime::parse_from_rfc3339(&custom_end_date)
                    .unwrap()
                    .to_utc(),
            )
        }
        _ => None,
    };

    let credential_subject = CredentialSubject {
        from: FromSubject::new(
            config.public.community_did.to_string(),
            their_c_did.to_string(),
            from_name,
            our_also_known_as,
            &tdk.get_shared_state().secrets_resolver,
        )
        .await?,
        to: ToSubject::new(their_c_did.to_string(), their_name, to_also_known_as),
        relationship_type,
        start_date,
        end_date,
        session: None,
    };

    let mut vrc = Vrc {
        issuer: config.public.community_did.to_string(),
        valid_from: valid_from.to_utc(),
        name,
        description,
        credential_subject,
        ..Default::default()
    };

    let secret = config.get_community_keys(tdk).await?.signing.secret;

    let proof = DataIntegrityProof::sign_jcs_data(&vrc, None, &secret, None)?;
    vrc.proof = Some(proof);

    println!(
        "{}\n{}",
        style("Issued VRC").color256(CLI_BLUE).underlined().bold(),
        style(serde_json::to_string_pretty(&vrc)?).color256(CLI_WHITE)
    );

    config.private.vrcs_issued.insert(their_c_did.clone(), vrc);
    config.public.logs.insert(
        LogFamily::Task,
        format!(
            "Issued VRC for remote C-DID({}) Task ID({})",
            their_c_did, task_id
        ),
    );

    config.private.tasks.remove(&task_id);

    Ok(true)
}
