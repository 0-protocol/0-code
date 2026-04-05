/// Paths or filenames that should always require explicit user permission.
pub fn is_dangerous_file(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_lowercase();

    const EXACT_SEGMENTS: &[&str] = &[
        "/etc/passwd",
        "/etc/shadow",
    ];
    for seg in EXACT_SEGMENTS {
        if lower == *seg || lower.ends_with(seg) {
            return true;
        }
    }

    let basename = normalized
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase();

    const DANGEROUS_NAMES: &[&str] = &[
        ".gitconfig",
        ".bashrc",
        ".bash_profile",
        ".zshrc",
        ".profile",
        ".env",
        "id_rsa",
        "authorized_keys",
    ];
    for name in DANGEROUS_NAMES {
        if basename == *name {
            return true;
        }
    }

    if lower.contains("/.ssh/") || lower.ends_with("/.ssh") || basename == ".ssh" {
        return true;
    }

    false
}

/// Shell / SQL patterns that should always require explicit user permission.
pub fn is_dangerous_command(command: &str) -> bool {
    let c = command.to_lowercase();

    const SUBSTRINGS: &[&str] = &[
        "rm -rf",
        "rm -fr",
        "git push --force",
        "git push -f",
        "drop table",
        "delete from",
        "chmod 777",
        "sudo",
        "mkfs",
        "dd if=",
    ];
    for s in SUBSTRINGS {
        if c.contains(s) {
            return true;
        }
    }

    if c.contains(":(){:|:&};:") || c.contains(": () { :; };") {
        return true;
    }

    false
}

/// Read-only tools that are safe to allow without prompting.
pub fn is_always_allowed_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "file_read" | "glob" | "grep" | "FileRead" | "Glob" | "Grep"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_files() {
        assert!(is_dangerous_file("/home/user/.bashrc"));
        assert!(is_dangerous_file("C:\\Users\\x\\.zshrc"));
        assert!(is_dangerous_file("/foo/.env"));
        assert!(is_dangerous_file("/home/u/.ssh/known_hosts"));
        assert!(is_dangerous_file("/etc/passwd"));
        assert!(is_dangerous_file("/etc/shadow"));
        assert!(is_dangerous_file("id_rsa"));
        assert!(is_dangerous_file("./authorized_keys"));
        assert!(!is_dangerous_file("/tmp/readme.txt"));
        assert!(!is_dangerous_file("/src/main.rs"));
    }

    #[test]
    fn dangerous_commands() {
        assert!(is_dangerous_command("rm -rf /"));
        assert!(is_dangerous_command("git push --force"));
        assert!(is_dangerous_command("DROP TABLE users"));
        assert!(is_dangerous_command("DELETE FROM t"));
        assert!(is_dangerous_command("chmod 777 x"));
        assert!(is_dangerous_command("sudo reboot"));
        assert!(is_dangerous_command("mkfs.ext4 /dev/sda1"));
        assert!(is_dangerous_command("dd if=/dev/zero"));
        assert!(is_dangerous_command(":(){:|:&};:"));
        assert!(!is_dangerous_command("ls -la"));
        assert!(!is_dangerous_command("cargo build"));
    }

    #[test]
    fn always_allowed_tools() {
        assert!(is_always_allowed_tool("FileRead"));
        assert!(is_always_allowed_tool("Glob"));
        assert!(is_always_allowed_tool("Grep"));
        assert!(is_always_allowed_tool("file_read"));
        assert!(is_always_allowed_tool("glob"));
        assert!(is_always_allowed_tool("grep"));
        assert!(!is_always_allowed_tool("Shell"));
    }
}
