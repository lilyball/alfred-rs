//! Helper for enabling Alfred workflows to upgrade themselves periodically (Alfred 3)
//!
//! Enable this feature by adding it in your `Cargo.toml`:
//!
//! ```toml
//! alfred = { version = "4", features = ["updater"] }
//! ```
//! Using this module, the workflow author will be able to make Alfred
//! check for & download latest releases from the remote server
//! within ajustable intervals (default is 24 hrs).
//!
//! For convenience, an associated method [`Updater::gh()`] is available to check for workflows hosted on `github.com`.
//!
//! However, it's possible to check with other servers as long as [`Releaser`] trait is
//! implemented for the desired remote service.
//!
//! The `github.com` hosted repository should have release items following `github`'s process.
//! This can be done by tagging a commit and then manually building a release where you
//! attach/upload `YourWorkflow.alfredworkflow` to the release page.
//!
//! The tag should follow all the [semantic versioning] rules.
//! The only exception to those rulse is that you can prepend your
//! semver tag with ASCII letter `v`: `v0.3.1` or `0.3.1`
//!
//! You can easily create `YourWorkflow.alfredworkflow` file by using the [export feature] of
//! Alfred in its preferences window.
//!
//! ### Note to workflow authors
//! - Workflolw authors should make sure that _released_ workflow files have
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
//! use alfred::Updater;
//!
//! # use std::env;
//! # use std::io;
//! # fn run() -> Result<(), io::Error> {
//! # env::set_var("alfred_workflow_uid", "abcdef");
//! # env::set_var("alfred_workflow_data", env::temp_dir());
//! # env::set_var("alfred_workflow_version", "0.0.0");
//! let updater = Updater::gh("spamwax/alfred-pinboard-rs");
//!
//! // The very first call to `update_ready()` will return `false`
//! // since it's assumed that user has just downloaded the workflow.
//! assert_eq!(false, updater.update_ready().unwrap());
//!
//! // Above will save the state of `Updater` in workflow's data folder.
//! // Depending on how long has elapsed since first run consequent calls
//! // to `update_ready()` may retuan false if it has been less than
//! // intervals set for checking (defaults to 24 hours).
//!
//! // However in subsequent runs, when the checking interval period has elapsed
//! // and there actually exist a new release, then `update_ready()` will return true.
//! // In this case, one can download the lastest available release
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
//! #
//! # fn main() {
//! #     if let Err(_) = run() {
//! #         ::std::process::exit(1);
//! #     } else {
//! #         ::std::process::exit(0);
//! #     }
//! # }
//! ```
//!
//! For the above example to automatically work, you then need to connect the output of the script
//! to an **Open File** action so that Alfred can insall/upgrade the new version.
//!
//! As suggested in above exmaple, you can add an Alfred variable to the item so that your workflow
//! can use it for further processing.
//!
//! See [`Updater::new()`] documentation if you are hosting your workflow on a service other than
//! `github.com` for an example of how to do it.

use chrono::prelude::*;
use env;
use reqwest;
use semver::Version;
use serde_json;
use std::cell::RefCell;
use std::env as StdEnv;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, BufWriter};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use time::Duration;

mod releaser;

use self::releaser::GithubReleaser;
pub use self::releaser::Releaser;

/// Default update interval duration 24 hr
const UPDATE_INTERVAL: i64 = 24 * 60 * 60;

/// Default timestamp for last time a check was done
const EPOCH_TIME: &str = "1970-01-01T00:00:00Z";

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
    last_check: RefCell<DateTime<Utc>>,
    update_interval: i64,
}

impl Updater<GithubReleaser> {
    /// Create an `Updater` object that will talk to a `github` repository to download latest releases.
    ///
    /// the `repo_name` should be in `user_name/repository_name` form.
    ///
    /// See the [module level documentation](./index.html) for full example and description.
    ///
    /// ```rust
    /// # extern crate alfred;
    /// use alfred::Updater;
    /// # use std::env;
    /// # fn ex_new() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let updater = Updater::gh("user_name/repo_name");
    /// # }
    ///
    /// # fn main() {
    /// #     ex_new();
    /// # }
    /// ```
    ///
    /// `Updater` created using this method will look at the assets
    /// available in each release point and download the first file whose name
    /// ends in `alfred3workflow` or `alfredworkflow`.
    pub fn gh<S>(repo_name: S) -> Self
    where
        S: Into<String> + AsRef<str>,
    {
        let releaser = Box::new(GithubReleaser::new(repo_name.as_ref()));

        Self::load_or_new(releaser)
    }
}

impl<T> Updater<T>
where
    T: Releaser,
{
    /// Create an `Updater` that will check & download latest releases from a remote server.
    ///
    /// How the `Updater` interacts with remote server should be implemented using the [`Releaser`]
    /// trait.
    ///
    /// ```rust
    /// # extern crate alfred;
    /// # extern crate semver;
    /// use std::io;
    ///
    /// use semver::Version;
    /// use alfred::Updater;
    /// use alfred::updater::Releaser;
    /// # use std::env;
    /// # fn ex_remote_releaser() {
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
    ///     fn downloadable_url(&self) -> Result<String, io::Error> {
    ///         Ok("ci.remote.cc".to_string())
    ///     }
    ///     fn newer_than(&self, v: &Version) -> Result<bool, io::Error> {
    ///         Ok(true)
    ///     }
    /// }
    ///
    /// let updater: Updater<RemoteCIReleaser> = Updater::new("my_hidden_proj");
    /// # }
    ///
    /// # fn main() {
    /// #     ex_remote_releaser();
    /// # }
    /// ```
    ///
    /// # Panic
    /// Method will panic if
    /// - `Updater` state cannot be read/written during instantiation, or
    /// - The workflow version cannot be parsed as semver compatible identifier.
    pub fn new<S>(repo_name: S) -> Updater<T>
    where
        S: Into<String> + AsRef<str>,
    {
        let releaser = Box::new(Releaser::new(repo_name.as_ref()));
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
    /// ```rust
    /// # extern crate alfred;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # fn ex_set_version() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs");
    /// updater.set_version("0.23.3");
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
    /// # Panic
    /// The method will panic if:
    /// - the passed value `version` cannot be parsed as a semver compatible string the
    /// function will panic.
    /// - any file io error happens during saving the `Updater` data to disk.
    pub fn set_version<S: AsRef<str>>(&mut self, version: S) {
        self.state.current_version =
            Version::parse(version.as_ref()).expect("version should follow semver rules.");
        StdEnv::set_var("alfred_workflow_version", version.as_ref());
        self.save().expect("cannot save updater data to file.");
    }

    /// Set the interval between checks for a newer release (in seconds)
    ///
    /// Example: Set interval to be 7 days
    ///
    /// ```rust
    /// # extern crate alfred;
    /// # use alfred::Updater;
    /// # use std::env;
    /// # fn ex_set_interval() {
    /// # env::set_var("alfred_workflow_uid", "abcdef");
    /// # env::set_var("alfred_workflow_data", env::temp_dir());
    /// # env::set_var("alfred_workflow_version", "0.0.0");
    /// let mut updater = Updater::gh("spamwax/alfred-pinboard-rs");
    /// updater.set_interval(7 * 24 * 60 * 60);
    /// # }
    ///
    /// # fn main() {
    /// #     ex_set_interval();
    /// # }
    /// ```
    ///
    /// # Panic
    /// Method will panic if any file io error happens during saving the `Updater` data to disk.
    pub fn set_interval(&mut self, tick: i64) {
        self.set_update_interval(tick);
        self.save().expect("cannot save updater data to file.");
    }

    /// Checks if a new update is availablle.
    ///
    /// This method will fetch the latess release information from repoository and compare it to
    /// current release of the workflow. The repository should tag each release according to
    /// semantic versioning for this to work.
    ///
    /// The workflow will store the timestamp for the very first call to this function as well as
    /// all subsequent calls that take place after the set interval time has passed.
    ///
    /// # Errors
    /// Checking for update can fail if network error, file error or Alfred environment variable
    /// errors happen.
    pub fn update_ready(&self) -> Result<bool, Error> {
        // last_check equal to EPOCH_TIME indicates that workflow is being run for first time.
        // Thus we update last_check to now and just save the updater state without checking for
        // updates since we assume user just downloaded the workflow. This change will trigger the
        // check after UPDATE_INTERVAL seconds.
        let epoch = EPOCH_TIME.parse::<DateTime<Utc>>().unwrap();

        if self.last_check() == epoch {
            self.set_last_check(Utc::now());
            self.save()?;
            Ok(false)
        } else if self.due_to_check() {
            let r = self.releaser.newer_than(self.current_version());
            self.set_last_check(Utc::now());
            self.save()?;
            r
        } else {
            Ok(false)
        }
    }

    /// Function to download and save the latest release and save into workflow's cache dir.
    ///
    /// If download and save operations are both successful, it returns name of file in which the
    /// downloaded Alfred workflow bundle is saved.
    ///
    /// The downloaded workflow will be saved in dedicated cache folder of the workflow, and it
    /// will be always renamed to `latest_release_WORKFLOW-UID.alfredworkflow`
    ///
    /// To install the downloaded release, your workflow needs to somehow open the saved file.
    ///
    /// Within shell, it can be installed by issuing something like:
    /// ```bash
    /// open -a "Alfred 3" latest_release_WORKFLOW-UID.alfredworkflow
    /// ```
    ///
    /// Or you can add "Run script" object to your workflow and use environment variables set by
    /// Alfred to automaticallly open the downloaded release:
    /// ```bash
    /// open "$alfred_workflow_cache/latest_release_$alfred_workflow_uid.alfredworkflow"
    /// ```
    /// # Errors
    /// Downloading latest workflow can fail if network error, file error or Alfred environment variable
    /// errors happen, or if `Releaser` cannot produce a usable download url.
    pub fn download_latest(&self) -> Result<PathBuf, Error> {
        let url = self.releaser.downloadable_url()?;
        let client = reqwest::Client::new();
        let mut resp = client
            .get(url.as_str())
            .send()
            .map_err(|e| str_to_io_err(e.to_string()))?;
        if !resp.status().is_success() {
            Err(str_to_io_err("unsuccessful github download"))
        } else {
            let mut latest_release_downloaded_fn = env::workflow_cache()
                .ok_or_else(|| str_to_io_err("couldn't get workflow's cache dir"))?;
            latest_release_downloaded_fn.push(format!(
                "latest_release_{}.alfredworkflow",
                env::workflow_uid().ok_or_else(|| str_to_io_err("workflow without uid!"))?
            ));

            File::create(&latest_release_downloaded_fn).and_then(|fp| {
                let mut buf_writer = BufWriter::with_capacity(0x10_0000, fp);
                resp.copy_to(&mut buf_writer)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
                Ok(latest_release_downloaded_fn)
            })
        }
    }
}

use self::releaser::str_to_io_err;
impl<T> Updater<T>
where
    T: Releaser,
{
    fn load_or_new(r: Box<T>) -> Self {
        let data_path = Self::build_data_fn().unwrap();
        if data_path.exists() {
            Updater {
                state: Self::load(&data_path),
                releaser: r,
            }
        } else {
            let last_check = RefCell::new(
                EPOCH_TIME
                    .parse::<DateTime<Utc>>()
                    .expect("couldn't create UTC epoch time"),
            );
            let current_version = env::workflow_version()
                .map(|s| Version::parse(&s).expect("version should follow semver rules"))
                .unwrap_or_else(|| Version::from((0, 0, 0)));
            let state = UpdaterState {
                current_version,
                last_check,
                update_interval: UPDATE_INTERVAL,
            };
            let updater = Updater { state, releaser: r };
            updater.save().expect("cannot save updater data to file.");
            updater
        }
    }

    fn current_version(&self) -> &Version {
        &self.state.current_version
    }

    fn last_check(&self) -> DateTime<Utc> {
        *self.state.last_check.borrow()
    }

    fn set_last_check(&self, t: DateTime<Utc>) {
        *self.state.last_check.borrow_mut() = t;
    }

    fn update_interval(&self) -> i64 {
        self.state.update_interval
    }

    fn set_update_interval(&mut self, t: i64) {
        self.state.update_interval = t;
    }

    fn due_to_check(&self) -> bool {
        Utc::now().signed_duration_since(self.last_check())
            > Duration::seconds(self.update_interval())
    }

    fn load(data_file: &PathBuf) -> UpdaterState {
        File::open(data_file)
            .and_then(|fp| {
                let buf_reader = BufReader::with_capacity(128, fp);
                serde_json::from_reader(buf_reader).map_err(|e| {
                    use std::io::{Error, ErrorKind};
                    Error::new(ErrorKind::Other, e)
                })
            })
            .expect("couldn't read updater's saved data")
    }

    fn save(&self) -> Result<(), Error> {
        let data_path =
            env::workflow_data().ok_or_else(|| str_to_io_err("cannot get workflow data dir"))?;
        create_dir_all(data_path)?;

        let data_file_path = Self::build_data_fn()?;
        let fp = File::create(data_file_path)
            .map_err(|_| str_to_io_err("canno create updater's saved data"))?;
        let buf_writer = BufWriter::with_capacity(128, fp);
        serde_json::to_writer(buf_writer, &self.state).map_err(|e| str_to_io_err(e.to_string()))
    }

    fn build_data_fn() -> Result<PathBuf, Error> {
        let mut data_path =
            env::workflow_data().ok_or_else(|| str_to_io_err("cannot get workflow data dir"))?;

        let saved_updater_fn = [
            env::workflow_uid()
                .ok_or_else(|| str_to_io_err("cannot get workflow_uid"))?
                .as_ref(),
            "-",
            env::workflow_name()
                .unwrap_or_else(|| "YouForgotToNameYourOwnWorkflow".to_string())
                .replace('/', "-")
                .replace(' ', "-")
                .as_str(),
            "-updater.json",
        ].concat();
        data_path.push(saved_updater_fn);
        Ok(data_path)
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
    use std::thread;
    use std::time;
    use tempfile::Builder;
    const VERSION_TEST: &str = "0.10.5";

    #[cfg(not(feature = "ci"))]
    #[test]
    fn it_loads_last_updater_state() {
        let mut updater_state_fn = StdEnv::temp_dir();

        setup_workflow_env_vars(false);

        updater_state_fn.push(
            String::from("workflow.B0AC54EC-601C") + "-YouForgotToNameYourOwnWorkflow-updater.json",
        );
        let _ = remove_file(&updater_state_fn);
        assert!(!updater_state_fn.exists());

        // Create a new Updater, and check if there is an update available
        let updater: Updater<GithubReleaser> = Updater::new("spamwax/alfred-pinboard-rs");
        assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));
        assert!(!updater.update_ready().expect("couldn't check for update"));

        // Now creating another one, will load the updater from file
        assert!(updater_state_fn.exists());
        let _updater: Updater<GithubReleaser> = Updater::new("spamwax/alfred-pinboard-rs");
    }

    #[test]
    #[should_panic(expected = "400 Bad Request")]
    fn it_handles_server_error_1() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME);
        // First update_ready is always false.
        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be due in 1 seconds
        updater.set_interval(1);
        thread::sleep(time::Duration::from_millis(1001));
        let _m = setup_mock_server(400);
        updater.update_ready().unwrap();
    }

    #[test]
    fn it_get_latest_info_from_releaser() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME);

        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be due in 1 seconds
        updater.set_interval(1);
        thread::sleep(time::Duration::from_millis(1001));

        assert!(updater.update_ready().expect("couldn't check for update"));
    }

    #[cfg(not(feature = "ci"))]
    #[test]
    fn it_talks_to_github() {
        setup_workflow_env_vars(true);

        let mut updater = Updater::gh("spamwax/alfred-pinboard-rs");
        assert_eq!(VERSION_TEST, format!("{}", updater.current_version()));

        // New temp folder causes due_to_check to be true since last check is assumed to be in 1970
        let seventies_called = EPOCH_TIME
            .parse::<DateTime<Utc>>()
            .expect("couldn't create UTC epoch time");
        assert_eq!(seventies_called, updater.last_check());
        assert!(updater.due_to_check());

        // Calling update_ready on first run of workflow will return false since we assume workflow
        // was just downloaded.
        assert!(!updater.update_ready().expect("couldn't check for update"));

        // Next check will be due in 1 seconds
        updater.set_interval(1);
        thread::sleep(time::Duration::from_millis(1001));

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
    fn it_tests_download() {
        setup_workflow_env_vars(true);
        let _m = setup_mock_server(200);

        let mut updater = Updater::gh(MOCK_RELEASER_REPO_NAME);
        assert_eq!(
            false,
            updater.update_ready().expect("couldn't check for update")
        );

        // Next check will be due in 1 seconds
        updater.set_interval(1);
        thread::sleep(time::Duration::from_millis(1001));
        // Force current version to be really old.
        updater.set_version("0.0.1");
        assert!(updater.update_ready().expect("couldn't check for update"));
        assert!(updater.download_latest().is_ok());
    }

    pub fn setup_workflow_env_vars(secure_temp_dir: bool) {
        // Mimic Alfred's environment variables
        let v = if secure_temp_dir {
            Builder::new()
                .prefix("download_latest_test")
                .rand_bytes(5)
                .tempdir()
                .unwrap()
                .into_path()
        } else {
            StdEnv::temp_dir()
        };
        let v: &OsStr = v.as_ref();
        StdEnv::set_var("alfred_workflow_data", v);
        StdEnv::set_var("alfred_workflow_bundleid", "MY_BUNDLE_ID");
        StdEnv::set_var("alfred_workflow_cache", v);
        StdEnv::set_var("alfred_workflow_uid", "workflow.B0AC54EC-601C");
        StdEnv::set_var("alfred_workflow_name", "YouForgotToNameYourOwnWorkflow");
        StdEnv::set_var("alfred_workflow_version", VERSION_TEST);
    }
}
