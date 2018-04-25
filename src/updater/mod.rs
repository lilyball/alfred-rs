//! Helper for enabling Alfred workflows to upgrade themselves periodically (Alfred 3)
//!
//! Enable this feature by adding it in your `Cargo.toml`:
//!
//! ```toml
//! alfred = { version = "4", features = ["updater"] }
//! ```
//! Using this module, the workflow author will be able to make Alfred
//! check for & download latest releases from the remote server
//! within adjustable intervals (default is 24 hrs).
//!
//! For convenience, an associated method [`Updater::gh()`] is available to check for workflows hosted on `github.com`.
//!
//! However, it's possible to check with other servers as long as the [`Releaser`] trait is
//! implemented for the desired remote service.
//!
//! The `github.com` hosted repository should have release items following `github`'s process.
//! This can be done by tagging a commit and then manually building a release where you
//! attach/upload `YourWorkflow.alfredworkflow` to the release page.
//!
//! The tag should follow all the [semantic versioning] rules.
//! The only exception to those rules is that you can prepend your
//! semantic version tag with ASCII letter `v`: `v0.3.1` or `0.3.1`
//!
//! You can easily create `YourWorkflow.alfredworkflow` file by using the [export feature] of
//! Alfred in its preferences window.
//!
//! ### Note to workflow authors
//! - Workflow authors should make sure that _released_ workflow files have
//! their version set in [Alfred's preferences window].
//! - However, this module provides [`set_version()`] to set the vesion during runtime.
//!
//! [`Releaser`]: trait.Releaser.html
//! [`Updater`]: struct.Updater.html
//! [`Updater::gh()`]: struct.Updater.html#method.gh
//! [`Updater::new()`]: struct.Updater.html#method.new
//! [semantic versioning]: https://semver.org
//! [export feature]: https://www.alfredapp.com/help/workflows/advanced/sharing-workflows/
//! [Alfred's preferences window]: https://www.alfredapp.com/help/workflows/advanced/variables/
//! [`set_version()`]: struct.Updater.html#method.set_version
//! [`set_interval()`]: struct.Updater.html#method.set_interval
//!
//! # Example
//!
//! Create an updater for a workflow hosted on `github.com/spamwax/alfred-pinboard-rs`.
//! By default, it will check for new releases every 24 hours.
//! To change the interval, use [`set_interval()`] method.
//!
//! ```rust
//! # extern crate alfred;
//! # extern crate failure;
//! use alfred::Updater;
//!
//! # use std::env;
//! # use failure::Error;
//! # use std::io;
//! # fn run() -> Result<(), Error> {
//! # env::set_var("alfred_workflow_uid", "abcdef");
//! # env::set_var("alfred_workflow_data", env::temp_dir());
//! # env::set_var("alfred_workflow_version", "0.0.0");
//! let updater =
//!     Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
//!
//! // The very first call to `update_ready()` will return `false`
//! // since it's assumed that user has just downloaded the workflow.
//! assert_eq!(false, updater.update_ready().unwrap());
//!
//! // Above will save the state of `Updater` in workflow's data folder.
//! // Depending on how long has elapsed since first run consequent calls
//! // to `update_ready()` may return false if it has been less than
//! // interval set for checking (defaults to 24 hours).
//!
//! // However in subsequent runs, when the checking interval period has elapsed
//! // and there actually exists a new release, then `update_ready()` will return true.
//! // In this case, one can download the latest available release
//! // to the workflow's default cache folder.
//! if updater.update_ready().unwrap() {
//!     match updater.download_latest() {
//!         Ok(downloaded_fn) => {
//!           alfred::json::write_items(io::stdout(), &[
//!               alfred::ItemBuilder::new("New version of workflow is available!")
//!                                    .subtitle("Click to upgrade!")
//!                                    .arg(downloaded_fn.to_str().unwrap())
//!                                    .variable("update_ready", "yes")
//!                                    .valid(true)
//!                                    .into_item()
//!           ]);
//!           Ok(())
//!         },
//!         Err(e) => {
//!             // Show an error message to user or log it.
//!             # Err(e)
//!         }
//!     }
//! }
//! #    else {
//! #        Ok(())
//! #    }
//! # }
//!
//! # fn main() {}
//! ```
//!
//! For the above example to automatically work, you then need to connect the output of the script
//! to an **Open File** action so that Alfred can install/upgrade the new version.
//!
//! As suggested in above example, you can add an Alfred variable to the item so that your workflow
//! can use it for further processing.
//!
//! See [`Updater::new()`] documentation if you are hosting your workflow on a service other than
//! `github.com` for an example of how to do it.

use chrono::prelude::*;
use env;
use failure::{err_msg, Error};
use reqwest;
use semver::Version;
use serde_json;
use std::cell::Cell;
use std::env as StdEnv;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use time::Duration;

mod releaser;

use self::releaser::GithubReleaser;
pub use self::releaser::Releaser;

/// Default update interval duration 24 hr
const UPDATE_INTERVAL: i64 = 24 * 60 * 60;

const LATEST_UPDATE_INFO_CACHE_FN: &str = "last_check_status.json";

/// Struct to check for & download the latest release of workflow from a remote server.
pub struct Updater<T>
where
    T: Releaser,
{
    state: UpdaterState,
    releaser: Box<T>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdaterState {
    current_version: Version,
    last_check: Cell<Option<DateTime<Utc>>>,
    update_interval: i64,
}

impl Updater<GithubReleaser> {
    /// Create an `Updater` object that will interface with a `github` repository.
    ///
    /// The `repo_name` should be in `user_name/repository_name` form. See the
    /// [module level documentation](./index.html) for full example and description.
    ///
    /// ```rust
    /// # extern crate alfred;
    /// use alfred::Updater;
    /// # use std::env;
    /// # fn ex_new() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let updater = Updater::gh("user_name/repo_name").expect("cannot initiate Updater");
    /// # }
    /// #
    /// # fn main() {}
    /// ```
    ///
    /// This only creates an `Updater` without performing any network operations.
    /// To check availability of a new release use [`update_ready()`] method.
    ///
    /// To download an available release use [`download_latest()`] method.
    ///
    /// # Errors
    /// Error will happen during calling this method if:
    /// - `Updater` state cannot be read/written during instantiation, or
    /// - The workflow version cannot be parsed as semantic version compatible identifier.
    ///
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    /// [`download_latest()`]: struct.Updater.html#method.download_latest
    pub fn gh<S>(repo_name: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let releaser = Box::new(GithubReleaser::new(repo_name));

        Self::load_or_new(releaser)
    }
}

impl<T> Updater<T>
where
    T: Releaser,
{
    /// Create an `Updater` object that will interface with a remote repository for updating operations.
    ///
    /// How the `Updater` interacts with the remote server should be implemented using the [`Releaser`]
    /// trait.
    ///
    /// ```rust
    /// # extern crate alfred;
    /// # extern crate semver;
    /// # extern crate failure;
    /// # extern crate url;
    /// use std::io;
    ///
    /// use semver::Version;
    /// use alfred::Updater;
    /// use alfred::updater::Releaser;
    /// # use std::env;
    /// # use failure::Error;
    /// # use url::Url;
    /// # fn main() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// # env::set_var("alfred_workflow_name", "NameName");
    ///
    /// struct RemoteCIReleaser {/* inner */};
    ///
    /// // You need to actually implement the trait, following is just a mock.
    /// impl Releaser for RemoteCIReleaser {
    ///     fn new<S: Into<String>>(project_id: S) -> Self {
    ///         RemoteCIReleaser {}
    ///     }
    ///     fn downloadable_url(&self) -> Result<Url, Error> {
    ///         Ok(Url::parse("https://ci.remote.cc")?)
    ///     }
    ///     fn latest_version(&self) -> Result<Version, Error> {
    ///         Ok(Version::from((1, 0, 12)))
    ///     }
    /// }
    ///
    /// let updater: Updater<RemoteCIReleaser> =
    ///     Updater::new("my_hidden_proj").expect("cannot initiate Updater");
    /// # }
    /// ```
    ///
    /// Note that the method only creates an `Updater` without performing any network operations.
    ///
    /// To check availability of a new release use [`update_ready()`] method.
    ///
    /// To download an available release use [`download_latest()`] method.
    ///
    /// # Errors
    /// Error will happen during calling this method if:
    /// - `Updater` state cannot be read/written during instantiation, or
    /// - The workflow version cannot be parsed as semantic version compatible identifier.
    ///
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    /// [`download_latest()`]: struct.Updater.html#method.download_latest
    /// [`Releaser`]: trait.Releaser.html
    pub fn new<S>(repo_name: S) -> Result<Updater<T>, Error>
    where
        S: Into<String>,
    {
        let releaser = Box::new(Releaser::new(repo_name));
        Self::load_or_new(releaser)
    }

    /// Set workflow's version to `version`.
    ///
    /// Content of `version` needs to follow semantic versioning.
    ///
    /// This method is provided so workflow authors can set the version from within the Rust code.
    ///
    /// For example, by reading cargo or git info during compile time and using this method to
    /// assign the version to workflow.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # use failure::Error;
    /// # fn ex_set_version() -> Result<(), Error> {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
    /// updater.set_version("0.23.3");
    /// # Ok(())
    /// # }
    ///
    /// # fn main() {
    /// #     ex_set_version();
    /// # }
    /// ```
    /// An alternative (recommended) way of setting version is through [Alfred's preferences window].
    ///
    /// [Alfred's preferences window]: https://www.alfredapp.com/help/workflows/advanced/variables/
    ///
    /// # Panics
    /// The method will panic if the passed value `version` cannot be parsed as a semantic version compatible string.
    pub fn set_version<S: AsRef<str>>(&mut self, version: S) {
        self.state.current_version = Version::parse(version.as_ref())
            .expect("version should follow semantic version rules.");
        StdEnv::set_var("alfred_workflow_version", version.as_ref());
    }

    /// Set the interval between checks for a newer release (in seconds)
    ///
    /// # Example
    /// Set interval to be 7 days
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # fn main() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater =
    ///     Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot initiate Updater");
    /// updater.set_interval(7 * 24 * 60 * 60);
    /// # }
    /// ```
    ///
    /// # Errors
    /// Error will happen during this method if any file io error happens while saving
    /// the `Updater` data to disk.
    pub fn set_interval(&mut self, tick: i64) {
        self.set_update_interval(tick);
        self.save().expect("cannot save updater data to file.");
    }

    /// Checks if a new update is available.
    ///
    /// This method will fetch the latest release information from repository (without a full download)
    /// and compare it to the current release of the workflow. The repository should
    /// tag each release according to semantic version scheme for this to work.
    ///
    /// The method **will** make a network call to fetch metadata of releases *only if* UPDATE_INTERVAL
    /// seconds has passed since the last network call, or in rare case of local cache file being corrupted.
    ///
    /// All calls, which happen before the UPDATE_INTERVAL seconds, will use a local cache
    /// to report availability of a release.
    ///
    /// For `Updater`s talking to `github.com`, this method will only fetch a small metadata file to extract
    /// the version info of the latest release.
    ///
    /// # Errors
    /// Checking for update can fail if network error, file error or Alfred environment variable
    /// errors happen.
    pub fn update_ready(&self) -> Result<bool, Error> {
        // A None value for last_check indicates that workflow is being run for first time.
        // Thus we update last_check to now and just save the updater state without asking
        // Releaser to do a remote call/check for us since we assume that user just downloaded
        // the workflow.

        // wf's data dir
        let p: &PathBuf = &env::workflow_data()
            .ok_or_else(|| err_msg("missing env variable for data dir"))
            .and_then(|mut dir| {
                dir.push(LATEST_UPDATE_INFO_CACHE_FN);
                Ok(dir)
            })?;

        // write version of latest avail. release (if any) to a cache file
        let write_last_check_status = |version: Option<Version>| -> Result<(), Error> {
            File::create(p).and_then(|fp| {
                let buf_writer = BufWriter::with_capacity(128, fp);
                serde_json::to_writer(buf_writer, &version)?;
                Ok(())
            })?;
            Ok(())
        };

        // read version of latest avail. release (if any) from a cache file
        let read_last_check_status = || -> Result<Option<Version>, Error> {
            Ok(File::open(p).and_then(|fp| {
                let buf_reader = BufReader::with_capacity(128, fp);
                let v = serde_json::from_reader(buf_reader)?;
                Ok(v)
            })?)
        };

        // make a network call to see if a newer version is avail.
        // save the result of call to cache file.
        let ask_releaser_for_update = || -> Result<bool, Error> {
            self.releaser
                .latest_version()
                .and_then(|v| Ok((*self.current_version() < v, v)))
                .and_then(|(r, v)| {
                    write_last_check_status(if r { Some(v) } else { None })?;
                    Ok(r)
                })
                .and_then(|r| {
                    self.set_last_check(Utc::now());
                    self.save()?;
                    Ok(r)
                })
        };

        // if first time checking, just update the updater's timestamp, no network call
        if self.last_check().is_none() {
            self.set_last_check(Utc::now());
            self.save()?;
            Ok(false)
        } else if self.due_to_check() {
            // it's time to talk to remote server
            ask_releaser_for_update()
        } else {
            // if we can't read the cache (corrupted or missing which can happen
            // if wf is cancelled while the network call or file operation was undergoing)
            // we make another network call. Otherwise we use its content to report if an
            // update is ready or not until the next due check is upon us.
            match read_last_check_status() {
                Err(_) => ask_releaser_for_update(),
                Ok(last_check_status) => {
                    if let Some(ref last_check_status) = last_check_status {
                        Ok(self.current_version() < last_check_status)
                    } else {
                        Ok(false)
                    }
                }
            }
        }
    }

    /// Check if it is time to ask remote server for latest updates.
    ///
    /// It returns `true` if it has been more than UPDATE_INTERVAL seconds since we last
    /// checked with server (i.e. ran [`update_ready()`]), otherwise returns false.
    ///
    /// [`update_ready()`]: struct.Updater.html#method.update_ready
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # extern crate alfred;
    /// # extern crate failure;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # use failure::Error;
    /// # fn run() -> Result<(), Error> {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs")?;
    ///
    /// // Assuming it is has been UPDATE_INTERVAL seconds since last time we ran the
    /// // `update_ready()`:
    /// # updater.update_ready();
    /// # updater.set_interval(0);
    /// assert_eq!(true, updater.due_to_check());
    /// # Ok(())
    /// # }
    /// # fn main() {
    /// # run();
    /// # }
    /// ```
    ///
    pub fn due_to_check(&self) -> bool {
        self.last_check().map_or(true, |dt| {
            Utc::now().signed_duration_since(dt) > Duration::seconds(self.update_interval())
        })
    }

    /// Function to download and save the latest release into workflow's cache dir.
    ///
    /// If the download and save operations are both successful, it returns name of file in which the
    /// downloaded Alfred workflow bundle is saved.
    ///
    /// The downloaded workflow will be saved in dedicated cache folder of the workflow, and it
    /// will be always renamed to `latest_release_WORKFLOW-UID.alfredworkflow`
    ///
    /// To install the downloaded release, your workflow needs to somehow open the saved file.
    ///
    /// Within shell, it can be installed by issuing something like:
    /// ```bash
    /// open -b com.runningwithcrayons.Alfred-3 latest_release_WORKFLOW-UID.alfredworkflow
    /// ```
    ///
    /// Or you can add "Run script" object to your workflow and use environment variables set by
    /// Alfred to automatically open the downloaded release:
    /// ```bash
    /// open -b com.runningwithcrayons.Alfred-3 "$alfred_workflow_cache/latest_release_$alfred_workflow_uid.alfredworkflow"
    /// ```
    ///
    /// ## Note:
    /// The method may take longer than other Alfred-based actions to complete. Workflow authors using this crate
    /// should implement strategies to prevent unpleasant long blocks of user's typical work flow.
    ///
    /// One solution is to make upgrade & download steps of workflow launchable by separate keyboard shortcuts
    /// or keyword within Alfred.
    ///
    /// # Errors
    /// Downloading latest workflow can fail if network error, file error or Alfred environment variable
    /// errors happen, or if `Releaser` cannot produce a usable download url.
    pub fn download_latest(&self) -> Result<PathBuf, Error> {
        let url = self.releaser.downloadable_url()?;
        let client = reqwest::Client::new();

        client
            .get(url)
            .send()?
            .error_for_status()
            .map_err(|e| e.into())
            .and_then(|mut resp| {
                env::workflow_cache() // Get workflow's dedicated cache folder
                    .ok_or_else(|| err_msg("missing env variable for cache dir"))
                    .and_then(|mut cache_dir| {
                        env::workflow_uid()
                            .ok_or_else(|| err_msg("missing env variable for uid"))
                            .and_then(|ref uid| { // Build file name for the downloaded data
                                cache_dir
                                    .push(["latest_release_", uid, ".alfredworkflow"].concat());
                                Ok(cache_dir)
                            })
                    })
                    .and_then(|latest_release_downloaded_fn| {
                        File::create(&latest_release_downloaded_fn) // Save downloaded data
                            .map_err(|e| e.into())
                            .and_then(|fp| {
                                let mut buf_writer = BufWriter::with_capacity(0x10_0000, fp);
                                resp.copy_to(&mut buf_writer)?;
                                Ok(latest_release_downloaded_fn)
                            })
                    })
            })
    }
}

impl<T> Updater<T>
where
    T: Releaser,
{
    fn load_or_new(r: Box<T>) -> Result<Self, Error> {
        if let Ok(saved_state) = Self::load() {
            Ok(Updater {
                state: saved_state,
                releaser: r,
            })
        } else {
            let current_version = env::workflow_version()
                .map_or_else(|| Ok(Version::from((0, 0, 0))), |v| Version::parse(&v))?;
            let state = UpdaterState {
                current_version,
                last_check: Cell::new(None),
                update_interval: UPDATE_INTERVAL,
            };
            let updater = Updater { state, releaser: r };
            updater.save()?;
            Ok(updater)
        }
    }

    fn current_version(&self) -> &Version {
        &self.state.current_version
    }

    fn last_check(&self) -> Option<DateTime<Utc>> {
        self.state.last_check.get()
    }

    fn set_last_check(&self, t: DateTime<Utc>) {
        self.state.last_check.set(Some(t));
    }

    fn update_interval(&self) -> i64 {
        self.state.update_interval
    }

    fn set_update_interval(&mut self, t: i64) {
        self.state.update_interval = t;
    }

    fn load() -> Result<UpdaterState, Error> {
        Self::build_data_fn().and_then(|data_file_path| {
            if data_file_path.exists() {
                Ok(File::open(data_file_path).and_then(|fp| {
                    let buf_reader = BufReader::with_capacity(128, fp);
                    Ok(serde_json::from_reader(buf_reader)?)
                })?)
            } else {
                Err(err_msg("missing updater data file"))
            }
        })
    }

    fn save(&self) -> Result<(), Error> {
        Self::build_data_fn()
            .and_then(|data_file_path| {
                create_dir_all(data_file_path.parent().unwrap())?;
                Ok(data_file_path)
            })
            .and_then(|data_file_path| Ok(File::create(data_file_path)?))
            .and_then(|fp| {
                let buf_writer = BufWriter::with_capacity(128, fp);
                serde_json::to_writer(buf_writer, &self.state)?;
                Ok(())
            })
    }

    fn build_data_fn() -> Result<PathBuf, Error> {
        let workflow_name = env::workflow_name()
            .unwrap_or_else(|| "YouForgotTo/フ:NameYourOwnWork}flowッ".to_string())
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();

        env::workflow_data()
            .ok_or_else(|| err_msg("missing env variable for data dir"))
            .and_then(|mut data_path| {
                env::workflow_uid()
                    .ok_or_else(|| err_msg("missing env variable for uid"))
                    .and_then(|ref uid| {
                        let filename = [uid, "-", workflow_name.as_str(), "-updater.json"].concat();
                        data_path.push(filename);

                        Ok(data_path)
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use self::releaser::tests::setup_mock_server;
    #[cfg(not(feature = "ci"))]
    use self::releaser::GithubReleaser;
    use self::releaser::MOCK_RELEASER_REPO_NAME;
    use super::*;
    use std::ffi::OsStr;
    #[cfg(not(feature = "ci"))]
    use std::fs::remove_file;
    use tempfile::Builder;
    const VERSION_TEST: &str = "0.10.5";

    #[test]
    fn it_tests_settings_filename() {
        setup_workflow_env_vars(true);
        let updater_state_fn = Updater::<GithubReleaser>::build_data_fn().unwrap();
        assert_eq!(
            "workflow.B0AC54EC-601C-YouForgotTo___NameYourOwnWork_flow_-updater.json",
            updater_state_fn.file_name().unwrap().to_str().unwrap()
        );
    }

    #[cfg(not(feature = "ci"))]
    #[test]
    fn it_loads_last_updater_state() {
        setup_workflow_env_vars(true);
        let updater_state_fn = Updater::<GithubReleaser>::build_data_fn().unwrap();

        let _ = remove_file(&updater_state_fn);
        assert!(!updater_state_fn.exists());

        // Create a new Updater, and check if there is an update available
        let mut updater: Updater<GithubReleaser> =
            Updater::new("spamwax/alfred-pinboard-rs").expect("cannot build Updater");
        assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));
        assert!(!updater.update_ready().expect("couldn't check for update"));
        updater.set_interval(-1);

        // Now creating another one, will load the updater from file
        assert!(updater_state_fn.exists());
        let updater: Updater<GithubReleaser> =
            Updater::new("spamwax/alfred-pinboard-rs").expect("cannot build Updater");
        assert_eq!(-1, updater.update_interval())
    }

    #[test]
    #[should_panic(expected = "ClientError(BadRequest)")]
    fn it_handles_server_error_1() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        // First update_ready is always false.
        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be immediate
        updater.set_interval(0);
        let _m = setup_mock_server(400);
        updater.update_ready().unwrap();
    }

    #[test]
    fn it_get_latest_info_from_releaser() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");

        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be immediate
        updater.set_interval(0);

        assert!(updater.update_ready().expect("couldn't check for update"));
    }

    #[cfg(not(feature = "ci"))]
    #[ignore]
    #[test]
    fn it_talks_to_github() {
        setup_workflow_env_vars(true);

        let mut updater = Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot build Updater");
        assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));

        assert!(updater.last_check().is_none());
        assert!(updater.due_to_check());

        // Calling update_ready on first run of workflow will return false since we assume workflow
        // was just downloaded.
        assert!(!updater.update_ready().expect("couldn't check for update"));

        // Next check will be immediate
        updater.set_interval(0);

        assert!(updater.due_to_check());
        // update should be ready since alfred-pinboard-rs
        // already has newer than VERSION_TEST.
        assert!(updater.update_ready().expect("couldn't check for update"));

        // Download from github
        assert!(updater.download_latest().is_ok());
        // no more updates.
        updater.set_interval(60);
        assert!(!updater.due_to_check());
    }

    #[test]
    #[cfg(not(feature = "ci"))]
    fn it_does_one_network_call_per_interval() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh("spamwax/alfred-pinboard-rs").expect("cannot build Updater");

        // Calling update_ready on first run of workflow will return false since we assume workflow
        // was just downloaded.
        assert!(!updater.update_ready().expect("couldn't check for update"));

        // Next check will be immediate
        updater.set_interval(0);

        // Next update_ready will make a network call
        assert!(updater.update_ready().expect("couldn't check for update"));

        // Increase interval
        updater.set_interval(86400);
        assert!(!updater.due_to_check());

        // make mock server return error. This way we can test that no network call was made
        let _m = setup_mock_server(503);
        let t = updater.update_ready();
        assert!(t.is_ok());
        // Make sure we stil report update is ready
        assert_eq!(true, t.unwrap());
    }

    #[test]
    fn it_tests_download() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME).expect("cannot build Updater");
        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be immediate
        updater.set_interval(0);
        // Force current version to be really old.
        updater.set_version("0.0.1");
        assert!(updater.update_ready().expect("couldn't check for update"));
        assert!(updater.download_latest().is_ok());
    }

    pub fn setup_workflow_env_vars(secure_temp_dir: bool) -> PathBuf {
        // Mimic Alfred's environment variables
        let path = if secure_temp_dir {
            Builder::new()
                .prefix("alfred_rs_test")
                .rand_bytes(5)
                .tempdir()
                .unwrap()
                .into_path()
        } else {
            StdEnv::temp_dir()
        };
        {
            let v: &OsStr = path.as_ref();
            StdEnv::set_var("alfred_workflow_data", v);
            StdEnv::set_var("alfred_workflow_cache", v);
            StdEnv::set_var("alfred_workflow_uid", "workflow.B0AC54EC-601C");
            StdEnv::set_var(
                "alfred_workflow_name",
                "YouForgotTo/フ:NameYourOwnWork}flowッ",
            );
            StdEnv::set_var("alfred_workflow_bundleid", "MY_BUNDLE_ID");
            StdEnv::set_var("alfred_workflow_version", VERSION_TEST);
            // println!(
            //     "\ndata: {:#?}\ncache: {:#?}",
            //     env::workflow_data().unwrap(),
            //     env::workflow_cache().unwrap()
            // );
        }
        path
    }
}
