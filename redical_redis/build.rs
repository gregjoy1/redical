use std::process::Command;

// Takes a semver string and converts it to an integer
// e.g. 0.2.1 -> 10200
// e.g. 1.2.3 -> 30201
fn convert_semver_to_integer(tag: String) -> u32 {
    let mut version = 0;
    let mut multiplier = 1;

    for part in tag.trim().split(".") {
        let part = part.parse::<u32>().unwrap_or(0);

        version += part * multiplier;
        multiplier *= 100;
    }

    version
}

fn main() {
    // Expose GIT_SHA env var
    let git_sha =
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output();

    if let Ok(sha) = git_sha {
        let sha = String::from_utf8(sha.stdout).unwrap();

        println!("cargo:rustc-env=GIT_SHA={sha}");
    }

    // Fetch latest git tags
    Command::new("git")
        .args(["fetch", "--tags"])
        .output()
        .unwrap();

    // Expose version via the latest git tag
    let git_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output();

    if let Ok(tag) = git_tag {
        let tag = String::from_utf8(tag.stdout).unwrap();
        println!("cargo:rustc-env=GIT_TAG={tag}");

        let module_version = convert_semver_to_integer(tag);
        println!("cargo:rustc-env=MODULE_VERSION={module_version}");
    }

    // Expose BUILD_DATE_STRING env var
    let build_date_string =
        Command::new("date")
            .args(["-u", "+%Y-%m-%dT%H:%M:%S %Z"])
            .output();

    if let Ok(date_string) = build_date_string {
        let date_string = String::from_utf8(date_string.stdout).unwrap();

        println!("cargo:rustc-env=BUILD_DATE_STRING={date_string}");
    }
}
