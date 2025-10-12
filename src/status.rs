/* Prints diagnostic status for the tool
*
*/

#[cfg(not(feature = "openpgp-card"))]
use crate::CLI_ORANGE;
#[cfg(not(feature = "default"))]
use crate::CLI_RED;
use crate::{CLI_BLUE, CLI_GREEN};
use console::style;

/// Prints diagnostic status to STDOUT
pub fn print_status() {
    println!(
        "{} {}",
        style("lkmv version:").color256(CLI_BLUE),
        style(env!("CARGO_PKG_VERSION")).bold().color256(CLI_GREEN)
    );

    feature_flags();

    openpgp_cards_status();
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

fn openpgp_cards_status() {
    println!();
    print!("{} ", style("OpenPGP-Card support:").color256(CLI_BLUE));

    #[cfg(not(feature = "openpgp-card"))]
    println!("{}", style("DISABLED").color256(CLI_ORANGE).bold());

    #[cfg(feature = "openpgp-card")]
    println!("{} ", style("Enabled").color256(CLI_GREEN).bold());
}
