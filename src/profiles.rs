use std::{
	collections::HashMap,
	fmt::Write,
	io::SeekFrom,
	sync::{Arc, OnceLock},
};

use serde::{Deserialize, Serialize};
use tokio::{
	fs::File,
	io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
	sync::{Mutex, OnceCell},
};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Profile {
	address: String,
	branch: String,
	mods_path: String,
	keep_mod_names: Vec<String>,
}

impl Profile {
	pub fn new<A, B, M>(address: A, branch: B, mods_path: M) -> Self
	where
		A: Into<String>,
		B: Into<String>,
		M: Into<String>,
	{
		Self {
			address: address.into(),
			branch: branch.into(),
			mods_path: mods_path.into(),
			keep_mod_names: Vec::new(),
		}
	}
}

async fn get_profiles_file() -> Arc<Mutex<File>> {
	static PROFILES_FILE: OnceCell<Arc<Mutex<File>>> = OnceCell::const_new();

	PROFILES_FILE
		.get_or_init(|| async {
			let profiles_file_dir = dirs::config_dir()
				.expect("Couldnt access OS's default config dir")
				.join("minecraft-mod-syncer");
			let profiles_file_path = profiles_file_dir.join("profiles.json");

			std::fs::create_dir_all(profiles_file_dir).expect("Couldn't create program dir");

			Arc::new(Mutex::new(
				File::options()
					.read(true)
					.write(true)
					.create(true)
					.open(profiles_file_path)
					.await
					.expect("Couldn't create profiles file"),
			))
		})
		.await
		.clone()
}

fn get_profiles() -> Arc<Mutex<HashMap<String, Profile>>> {
	static PROFILES: OnceLock<Arc<Mutex<HashMap<String, Profile>>>> = OnceLock::new();
	PROFILES
		.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
		.clone()
}

pub async fn load_profiles() {
	let file = get_profiles_file().await;
	let mut file_locked = file.lock().await;

	let profiles = get_profiles();
	let mut profiles_locked = profiles.lock().await;

	let mut buf = String::new();
	file_locked
		.read_to_string(&mut buf)
		.await
		.expect("Failed to read in profiles file");

	if buf.len() == 0 {
		buf.write_str("{}")
			.expect("Failed to initialize profiles file");
	}

	let read_profiles: HashMap<String, Profile> =
		serde_json::from_str(&buf).expect("Failed to serialize profiles file");

	for (k, v) in read_profiles {
		profiles_locked.insert(k, v);
	}
}

pub async fn save_profiles() {
	let file = get_profiles_file().await;
	let mut file_locked = file.lock().await;

	let profiles = get_profiles();
	let profiles_locked = profiles.lock().await;

	let json =
		serde_json::to_string(&*profiles_locked).expect("Failed to convert profiles to json");

	file_locked
		.set_len(0)
		.await
		.expect("Failed to clear profiles file");
	file_locked
		.seek(SeekFrom::Start(0))
		.await
		.expect("Failed to seek profiles file");

	file_locked
		.write_all(json.as_bytes())
		.await
		.expect("Failed to write profiles file");

	file_locked
		.flush()
		.await
		.expect("Failed to flush profiles file");
}

pub async fn with_profile<F, R>(name: &str, f: F) -> Option<R>
where
	F: FnOnce(&mut Profile) -> R,
{
	let profiles = get_profiles();
	profiles.lock().await.get_mut(name).map(f)
}

pub async fn new_profile<N>(name: N, profile: Profile)
where
	N: Into<String>,
{
	let profiles = get_profiles();
	profiles.lock().await.insert(name.into(), profile);
}

pub async fn delete_profile(name: &str) {
	let profiles = get_profiles();
	profiles.lock().await.remove(name);
}

pub async fn profile_exists(name: &str) -> bool {
	let profiles = get_profiles();
	profiles.lock().await.get(name).is_some()
}

pub async fn get_profile_names() -> Vec<String> {
	let profiles = get_profiles();
	profiles.lock().await.keys().map(String::clone).collect()
}
