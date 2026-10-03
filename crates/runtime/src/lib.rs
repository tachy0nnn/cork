use directories::ProjectDirs;

pub fn initialize_config_dir() {
    if let Some(proj_dirs) = ProjectDirs::from("owo", "cork", "cork") {
        proj_dirs.config_dir();
    }
}

pub fn extract_libraries() {
    
}
