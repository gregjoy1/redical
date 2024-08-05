use std::process::Command;

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

    // Expose GIT_TAG env var
    let git_tag =
        Command::new("git")
            .args(["describe", "--abbrev=0", "--tags"])
            .output();

    if let Ok(tag) = git_tag {
        let tag = String::from_utf8(tag.stdout).unwrap();

        println!("cargo:rustc-env=GIT_TAG={tag}");
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
