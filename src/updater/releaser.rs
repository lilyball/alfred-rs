use failure::err_msg;
use failure::Error;
#[cfg(test)]
use mockito;
use reqwest;
use semver::Version;
use serde_json;
use std::cell::RefCell;

const GITHUB_API_URL: &str = "https://api.github.com/repos/";
const GITHUB_LATEST_RELEASE_ENDPOINT: &str = "/releases/latest";

#[cfg(test)]
static MOCKITO_URL: &'static str = mockito::SERVER_URL;
#[cfg(test)]
pub const MOCK_RELEASER_REPO_NAME: &str = "MockZnVja29mZg==/fd850fc2e63511e79f720023dfdf24ec";

/// An interface for checking with remote servers to identify the latest release for an
/// Alfred workflow.
///
/// This trait has been implemented for `GithubReleaser` to check for a newer version of a workflow
/// that's maintained on `github.com`
pub trait Releaser {
    /// Creates a new `Releaser` instance that is identified as `name`
    fn new<S: Into<String>>(name: S) -> Self;

    /// Returns an `Ok(url)` that can be used to directly download the `.alfredworkflow`
    ///
    /// Method returns `Err(io::Error)` on file or network error.
    fn downloadable_url(&self) -> Result<String, Error>;

    /// Checks if the latest available release is newer than `version`
    ///
    /// Method returns `Err(io::Error)` on file or network error.
    fn newer_than(&self, version: &Version) -> Result<bool, Error>;
}

// Struct to handle checking and downloading release files from `github.com`
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubReleaser {
    repo: String,
    latest_release: RefCell<Option<ReleaseItem>>,
}

// Struct to store information about a single release point.
//
// Each release point may have multiple downloadable assets.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseItem {
    /// name of release that should hold a semver compatible identifier.
    pub tag_name: String,
    assets: Vec<ReleaseAsset>,
}

/// A single downloadable asset.
#[derive(Debug, Serialize, Deserialize)]
struct ReleaseAsset {
    url: String,
    name: String,
    state: String,
    browser_download_url: String,
}

impl GithubReleaser {
    fn new<S: Into<String>>(s: S) -> Self {
        GithubReleaser {
            repo: s.into(),
            latest_release: RefCell::new(None),
        }
    }

    fn latest_release_data(&self) -> Result<(), Error> {
        let client = reqwest::Client::new();

        #[cfg(test)]
        let url = if self.repo == MOCK_RELEASER_REPO_NAME {
            format!("{}{}", MOCKITO_URL, GITHUB_LATEST_RELEASE_ENDPOINT)
        } else {
            format!(
                "{}{}{}",
                GITHUB_API_URL, self.repo, GITHUB_LATEST_RELEASE_ENDPOINT
            )
        };

        #[cfg(not(test))]
        let url = format!(
            "{}{}{}",
            GITHUB_API_URL, self.repo, GITHUB_LATEST_RELEASE_ENDPOINT
        );

        client
            .get(&url)
            .send()?
            .error_for_status()
            .map_err(|e| e.into())
            .and_then(|resp| {
                let mut latest: ReleaseItem = serde_json::from_reader(resp)?;
                if latest.tag_name.starts_with('v') {
                    latest.tag_name = latest.tag_name.as_str()[1..].to_string();
                }
                *self.latest_release.borrow_mut() = Some(latest);
                Ok(())
            })
    }
}

impl Releaser for GithubReleaser {
    fn new<S: Into<String>>(repo_name: S) -> GithubReleaser {
        GithubReleaser::new(repo_name)
    }

    // This implementation of Releaser will favor urls that end with `alfred3workflow`
    // over `alfredworkflow`
    fn downloadable_url(&self) -> Result<String, Error> {
        let release = self.latest_release.borrow();
        let urls = release.as_ref().map(|r| {
            r.assets
                .iter()
                .filter(|asset| {
                    asset.state == "uploaded"
                        && (asset.browser_download_url.ends_with("alfredworkflow")
                            || asset.browser_download_url.ends_with("alfred3workflow"))
                })
                .map(|asset| &asset.browser_download_url)
                .collect::<Vec<&String>>()
        });
        let urls = urls.ok_or_else(|| {
            err_msg("no release item available, did you check with newer_than?")
        })?;

        if urls.len() == 1 {
            Ok(urls[0].clone())
        } else if urls.len() > 1 {
            let url = urls.iter().find(|item| item.ends_with("alfred3workflow"));
            let u = match url {
                Some(&link) => (*link).clone(),
                None => urls[0].clone(),
            };
            Ok(u)
        } else {
            Err(err_msg("no usable download url"))
        }
    }

    fn newer_than(&self, v: &Version) -> Result<bool, Error> {
        if self.latest_release.borrow().is_none() {
            self.latest_release_data()?;
        }

        let latest_version = self.latest_release
            .borrow()
            .as_ref()
            .map(|r| Version::parse(&r.tag_name).ok())
            .ok_or_else(|| err_msg("Couldn't parse fetched version."))?;
        Ok(*v < latest_version.ok_or_else(|| err_msg("should have version at this point"))?)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use mockito::{mock, Matcher, Mock};

    #[test]
    fn it_tests_releaser() {
        let _m = setup_mock_server(200);
        let releaser = GithubReleaser::new(MOCK_RELEASER_REPO_NAME);

        // Calling downloadable_url before checking for newer_than will return error
        assert!(releaser.downloadable_url().is_err());

        assert!(
            releaser
                .newer_than(&Version::from((0, 0, 0)))
                .expect("couldn't do newer_than")
        );

        assert_eq!("http://127.0.0.1:1234/releases/download/v0.11.1/alfred-pinboard-rust-v0.11.1.alfredworkflow",
                   releaser.downloadable_url().unwrap());
    }

    pub fn setup_mock_server(status_code: usize) -> Mock {
        mock(
            "GET",
            Matcher::Regex(r"^/releases/(latest|download).*$".to_string()),
        ).with_status(status_code)
            .with_header("content-type", "application/json")
            .with_body(include_str!("../../res/latest.json"))
            .create()
    }
}
