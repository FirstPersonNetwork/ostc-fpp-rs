use std::rc::Rc;

use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::{CLI_ORANGE, CLI_RED, config::Config, log::LogFamily, vrc::visual::vrc_show};

/// Remove a VRC by it's ID
pub fn remove_vrc_by_id(config: &mut Config, id: &Rc<String>) -> bool {
    if let Some(vrc) = config.vrcs.get(id) {
        vrc_show(id, vrc);

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you sure you want to delete VRC?")
            .interact()
            .unwrap()
        {
            config.private.vrcs_received.remove_vrc(id);
            config.private.vrcs_issued.remove_vrc(id);

            config.public.logs.insert(
                LogFamily::Relationship,
                format!("User removed VRC ID: {id}"),
            );
            true
        } else {
            println!("{}", style("Aborting VRC Removal").color256(CLI_ORANGE));
            false
        }
    } else {
        println!(
            "{}{}",
            style("ERROR: No VRC found for ID: ").color256(CLI_RED),
            style(id).color256(CLI_ORANGE)
        );
        false
    }
}
