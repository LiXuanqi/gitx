use git2::{Repository, BranchType};

pub fn get_all_branches() -> Result<Vec<String>, git2::Error> {
    let repo = Repository::open(".")?;
    let mut branches = Vec::new();
    
    let branch_iter = repo.branches(Some(BranchType::Local))?;
    
    for branch in branch_iter {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            branches.push(name.to_string());
        }
    }
    
    Ok(branches)
}

pub fn switch_branch(branch_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    
    // Get the branch reference
    let branch_ref = format!("refs/heads/{}", branch_name);
    let obj = repo.revparse_single(&branch_ref)?;
    
    // Checkout the branch
    repo.checkout_tree(&obj, None)?;
    
    // Set HEAD to point to the branch
    repo.set_head(&branch_ref)?;
    
    Ok(())
}