use crate::status_display;

pub async fn handle_prs() -> Result<(), Box<dyn std::error::Error>> {
    match status_display::display_status().await {
        Ok(()) => {
            // Status displayed successfully
        }
        Err(e) => {
            eprintln!("Error displaying status: {}", e);
        }
    }
    Ok(())
}