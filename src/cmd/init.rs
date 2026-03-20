use anyhow::Result;
use rust_i18n::t;

use crate::{
    init,
    style::{
        goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error_bold,
        print_info, print_success_bold, print_warning,
    },
};

pub fn handle_init(dry_run: bool) -> Result<()> {
    match init::init(dry_run) {
        Ok(()) => {
            if dry_run {
                print_info(t!("main.init.dry_run_done"));
            } else {
                print_success_bold(t!("main.init.wrote_config"));
            }
            goodbye_with_success();
        }
        Err(init::InitError::Aborted) => {
            print_warning(t!("main.init.cancelled"));
            goodbye_with_warning();
        }
        Err(init::InitError::FileExists) => {
            print_error_bold(t!("main.init.file_exists"));
            print_info(t!("main.init.file_exists_hint"));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.init.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
