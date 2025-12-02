use std::rc::Rc;

use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::{
    CLI_ORANGE, CLI_RED,
    config::Config,
    log::LogFamily,
    vrc::{Vrcs, visual::vrc_show},
};

/// Remove a VRC by it's ID
pub fn remove_vrc_by_id(config: &mut Config, id: &str) -> bool {
    if let Some(vrc) = config.vrcs.get(&Rc::new(id.to_string())) {
        vrc_show(id, vrc);

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Are you sure you want to delete VRC?")
            .interact()
            .unwrap()
        {
            remove_vrc(&mut config.private.vrcs_received, id);
            remove_vrc(&mut config.private.vrcs_issued, id);

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

fn remove_vrc(vrcs: &mut Vrcs, id: &str) {
    for r in vrcs.values_mut() {
        r.retain(|vrc_id, _| vrc_id.as_str() != id);
    }
}
