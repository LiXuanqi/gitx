use std::process::Command;

pub fn handle_commit(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Passthrough to git commit with all provided arguments
    let mut cmd = Command::new("git");
    cmd.arg("commit");
    cmd.args(args);
    
    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Error running git commit: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}