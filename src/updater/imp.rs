use super::Error;
use super::Releaser;
use super::*;
use Updater;

#[doc(hidden)]
impl<T> Updater<T>
where
    T: Releaser,
{
    pub fn load_or_new(r: Box<T>) -> Result<Self, Error> {
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

    pub fn current_version(&self) -> &Version {
        &self.state.current_version
    }

    pub fn last_check(&self) -> Option<DateTime<Utc>> {
        self.state.last_check.get()
    }

    pub fn set_last_check(&self, t: DateTime<Utc>) {
        self.state.last_check.set(Some(t));
    }

    pub fn update_interval(&self) -> i64 {
        self.state.update_interval
    }

    pub fn set_update_interval(&mut self, t: i64) {
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

    pub fn save(&self) -> Result<(), Error> {
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

    pub fn build_data_fn() -> Result<PathBuf, Error> {
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
