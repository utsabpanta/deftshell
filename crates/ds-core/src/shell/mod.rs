pub mod bash;
pub mod fish;
pub mod prompt;
pub mod zsh;

use crate::config::ShellType;

/// Generate the init script for the given shell type
pub fn generate_init_script(shell: ShellType) -> String {
    match shell {
        ShellType::Zsh => zsh::init_script(),
        ShellType::Bash => bash::init_script(),
        ShellType::Fish => fish::init_script(),
    }
}
