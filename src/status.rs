/* Prints diagnostic status for the tool
*
*/

use crate::{CLI_BLUE, CLI_GREEN, CLI_ORANGE, CLI_RED, config::Config};
use anyhow::Result;
use console::{Term, style};

/// Prints diagnostic status to STDOUT
pub fn print_status(term: &Term) {
    println!(
        "{} {}",
        style("lkmv version:").color256(CLI_BLUE),
        style(env!("CARGO_PKG_VERSION")).bold().color256(CLI_GREEN)
    );

    feature_flags();

    // Show any openpgp-cards and corresponding status
    if let Err(error) = openpgp_cards_status() {
        println!(
            "{} {}",
            style("An error occurred in handling openpgp-cards:").color256(CLI_RED),
            style(error.to_string()).color256(CLI_ORANGE)
        );
    }

    // Check if we can load config
    println!();
    let config = match Config::load(term) {
        Ok(cfg) => {
            println!(
                "{} {}",
                style("lkmv configuration:").color256(CLI_BLUE),
                style("successfully loaded").color256(CLI_GREEN)
            );
            cfg
        }
        Err(e) => {
            println!(
                "{}{}",
                style("ERROR: Couldn't load configuration: ").color256(CLI_RED),
                style(e).color256(CLI_ORANGE)
            );
            return;
        }
    };

    // Check DID Resolution status
    /*
        match did_status(&config.public.community_did) {
            Ok(doc) => {
                println!("{}\n{}", style("Community DID Resolved"))
            }
        }
    */
}

// Rust Feature Flags enabled for this build
fn feature_flags() {
    print!("{} ", style("lkmv enabled features:").color256(CLI_BLUE),);
    let mut prev_flag = false; // set to true if a feature has been enabled

    #[cfg(not(feature = "default"))]
    {
        print!("{}", style("no-default").color256(CLI_RED));
        prev_flag = true;
    }

    #[cfg(feature = "default")]
    {
        if prev_flag {
            print!("{}", style(", ").bold().color256(CLI_GREEN))
        }
        print!("{}", style("default").bold().color256(CLI_GREEN));
        prev_flag = true;
    }

    #[cfg(feature = "openpgp-card")]
    {
        if prev_flag {
            print!("{}", style(", ").bold().color256(CLI_GREEN))
        }
        print!("{}", style("openpgp-card").bold().color256(CLI_GREEN));
    }

    println!();
}

fn openpgp_cards_status() -> Result<()> {
    println!();
    print!("{} ", style("OpenPGP Card support:").color256(CLI_BLUE));

    #[cfg(not(feature = "openpgp-card"))]
    println!("{}", style("DISABLED").color256(CLI_ORANGE).bold());

    #[cfg(feature = "openpgp-card")]
    {
        use crate::openpgp_card::{cards, print_cards};

        println!("{} ", style("Enabled").color256(CLI_GREEN).bold());

        let mut cards = cards()?;
        print_cards(&mut cards)?;
    }

    Ok(())
}
