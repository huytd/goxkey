//! `ibrus`: ibus component (executable) for pmim-ibus
//!
//! <https://github.com/fm-elpac/pmim-ibus>
#![deny(unsafe_code)]

use std::process::ExitCode;

use pm_bin::{cli_arg, init_env_logger, pm_init};
pm_init!();

use librush::pmim;

fn main() -> Result<(), ExitCode> {
    init_env_logger();

    let mut flatpak = false;
    if let Some(a) = cli_arg(print_version) {
        if a.len() > 0 {
            match a[0].as_str() {
                "--flatpak" => {
                    flatpak = true;
                }
                _ => {}
            }
        }

        pmim::main(flatpak).unwrap();
        Ok(())
    } else {
        Ok(())
    }
}
