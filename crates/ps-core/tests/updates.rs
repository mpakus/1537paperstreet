use ps_core::updates::{from_github_json, install_ready_message};

fn release_json(tag: &str, url: &str) -> String {
    format!(r#"{{"tag_name":"{tag}","html_url":"{url}"}}"#)
}

fn release_json_with_asset(
    tag: &str,
    url: &str,
    name: &str,
    asset_url: &str,
    digest: &str,
) -> String {
    format!(
        r#"{{"tag_name":"{tag}","html_url":"{url}","assets":[{{"name":"{name}","browser_download_url":"{asset_url}","digest":"{digest}"}}]}}"#
    )
}

fn release_page(tag: &str) -> String {
    format!("https://github.com/mpakus/1537paperstreet/releases/tag/{tag}")
}

fn zip_name(version: &str) -> String {
    format!("1537paperstreet-{version}-macos-universal.zip")
}

fn zip_url(tag: &str, version: &str) -> String {
    format!(
        "https://github.com/mpakus/1537paperstreet/releases/download/{tag}/{}",
        zip_name(version)
    )
}

#[test]
fn newer_github_tag_is_an_available_update() {
    let check =
        from_github_json("0.2.1", &release_json("v0.2.2", &release_page("v0.2.2"))).expect("parse");
    assert!(check.available);
    assert!(!check.can_install);
    assert_eq!(check.current, "0.2.1");
    assert_eq!(check.latest, "0.2.2");
    assert_eq!(check.release_url, release_page("v0.2.2"));
    assert!(check.asset_url.is_empty());
    assert!(check.asset_sha256.is_empty());
    assert_eq!(check.message, "Version 0.2.2 is available.");
}

#[test]
fn matching_or_older_tags_are_up_to_date() {
    let same =
        from_github_json("0.2.1", &release_json("v0.2.1", &release_page("v0.2.1"))).expect("same");
    assert!(!same.available);
    assert!(!same.can_install);
    assert!(same.release_url.is_empty());
    assert_eq!(same.message, "You're up to date (0.2.1).");

    let older =
        from_github_json("0.3.0", &release_json("0.2.9", &release_page("0.2.9"))).expect("older");
    assert!(!older.available);
    assert!(!older.can_install);
}

#[test]
fn trusted_macos_zip_is_installable() {
    let digest = "sha256:0123456789abcdef0123456789ABCDEF0123456789abcdef0123456789abcdef";
    let check = from_github_json(
        "0.8.0",
        &release_json_with_asset(
            "v0.8.1",
            &release_page("v0.8.1"),
            &zip_name("0.8.1"),
            &zip_url("v0.8.1", "0.8.1"),
            digest,
        ),
    )
    .expect("parse");
    assert!(check.available);
    assert!(check.can_install);
    assert_eq!(check.asset_url, zip_url("v0.8.1", "0.8.1"));
    assert_eq!(
        check.asset_sha256,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

#[test]
fn untagged_download_path_is_accepted() {
    let check = from_github_json(
        "0.8.0",
        &release_json_with_asset(
            "0.8.1",
            &release_page("0.8.1"),
            &zip_name("0.8.1"),
            &zip_url("0.8.1", "0.8.1"),
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    )
    .expect("parse");
    assert!(check.can_install);
    assert_eq!(check.asset_url, zip_url("0.8.1", "0.8.1"));
}

#[test]
fn rejects_a_non_github_release_url() {
    let check = from_github_json(
        "0.2.1",
        &release_json("v0.3.0", "https://evil.example/download"),
    )
    .expect("parse");
    assert!(check.available);
    assert!(!check.can_install);
    assert_eq!(
        check.release_url,
        "https://github.com/mpakus/1537paperstreet/releases"
    );
}

#[test]
fn rejects_an_untrusted_zip_url() {
    let check = from_github_json(
        "0.8.0",
        &release_json_with_asset(
            "v0.8.1",
            &release_page("v0.8.1"),
            &zip_name("0.8.1"),
            "https://evil.example/1537paperstreet-0.8.1-macos-universal.zip",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    )
    .expect("parse");
    assert!(check.available);
    assert!(!check.can_install);
    assert!(check.asset_url.is_empty());
}

#[test]
fn rejects_a_zip_without_a_sha256_digest() {
    let check = from_github_json(
        "0.8.0",
        &release_json_with_asset(
            "v0.8.1",
            &release_page("v0.8.1"),
            &zip_name("0.8.1"),
            &zip_url("v0.8.1", "0.8.1"),
            "",
        ),
    )
    .expect("parse");
    assert!(check.available);
    assert!(!check.can_install);

    let null_digest = format!(
        r#"{{"tag_name":"v0.8.1","html_url":"{}","assets":[{{"name":"{}","browser_download_url":"{}","digest":null}}]}}"#,
        release_page("v0.8.1"),
        zip_name("0.8.1"),
        zip_url("v0.8.1", "0.8.1")
    );
    let missing = from_github_json("0.8.0", &null_digest).expect("null digest");
    assert!(missing.available);
    assert!(!missing.can_install);
}

#[test]
fn ignores_non_macos_zip_assets() {
    let check = from_github_json(
        "0.8.0",
        &release_json_with_asset(
            "v0.8.1",
            &release_page("v0.8.1"),
            "1537paperstreet_0.8.1_universal.dmg",
            "https://github.com/mpakus/1537paperstreet/releases/download/v0.8.1/1537paperstreet_0.8.1_universal.dmg",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    )
    .expect("parse");
    assert!(check.available);
    assert!(!check.can_install);
}

#[test]
fn rejects_invalid_release_payloads() {
    assert!(from_github_json("0.2.1", "not-json").is_err());
    assert!(
        from_github_json(
            "0.2.1",
            r#"{"html_url":"https://github.com/mpakus/1537paperstreet/releases"}"#
        )
        .is_err()
    );
    assert!(from_github_json("0.2.1", &release_json("nightly", &release_page("nightly"))).is_err());
}

#[test]
fn install_ready_message_asks_to_restart() {
    assert_eq!(
        install_ready_message("0.8.2"),
        "Version 0.8.2 is installed. Restart to finish the update."
    );
}
