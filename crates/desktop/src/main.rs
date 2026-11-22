//! Thin desktop host. The UI and its generated Slint types live in openwatch-ui.

use openwatch_ui::AppError;

/// Entry point used by the desktop binary and by tests.
pub fn run() -> Result<(), AppError> {
    openwatch_ui::run()
}

fn main() -> Result<(), AppError> {
    run()
}
