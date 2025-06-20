use crate::config;

pub fn handle_init() -> Result<(), Box<dyn std::error::Error>> {
    match config::interactive_init() {
        Ok(()) => {
            // Initialization completed successfully
        }
        Err(e) => {
            eprintln!("Error during initialization: {}", e);
        }
    }
    Ok(())
}