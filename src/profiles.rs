use std::{io::SeekFrom, sync::Arc};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::{
	fs::File,
	io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
	sync::{Mutex, OnceCell},
};

pub const DEFAULT: &'static str = "default";

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Profile {
	pub address: String,
	pub branch: String,
	pub mods_path: String,
	pub keep_mods_in_branch: DashMap<String, Vec<String>>,
}

impl Profile {
	pub fn new<A, M>(address: A, mods_path: M, branch: Option<String>) -> Self
	where
		A: Into<String>,
		M: Into<String>,
	{
		Self {
			address: address.into(),
			branch: branch.unwrap_or_default(),
			mods_path: mods_path.into(),
			keep_mods_in_branch: DashMap::new(),
		}
	}
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProfilesMap {
	version: u8,
	last_profile: String,
	profiles: DashMap<String, Profile>,
}

impl ProfilesMap {
	pub fn new() -> Self {
		ProfilesMap {
			version: 1,
			last_profile: String::new(),
			profiles: DashMap::new(),
		}
	}

	pub fn get_profile(
		&self,
		name: &str,
	) -> Option<dashmap::mapref::one::Ref<'_, String, Profile>> {
		self.profiles.get(name)
	}

	pub fn get_mut_profile(
		&self,
		name: &str,
	) -> Option<dashmap::mapref::one::RefMut<'_, String, Profile>> {
		self.profiles.get_mut(name)
	}

	pub fn get_last_profile_name(&self) -> &str {
		self.last_profile.as_ref()
	}

	pub fn set_last_profile_name<N>(&mut self, name: N)
	where
		N: Into<String>,
	{
		self.last_profile = name.into();
	}

	pub fn new_profile<N>(&self, name: N, profile: Profile)
	where
		N: Into<String>,
	{
		let name = name.into();
		if !validate_profile_name(&name) {
			return;
		}
		self.profiles.insert(name, profile);
	}

	pub fn delete_profile(&self, name: &str) {
		if !validate_profile_name(&name) {
			return;
		}
		self.profiles.remove(name);
	}

	pub fn profile_exists(&self, name: &str) -> bool {
		self.profiles.contains_key(name)
	}

	pub fn get_profile_names(&self) -> Vec<String> {
		self.profiles.iter().map(|v| v.key().clone()).collect()
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

// INFO: not longer used, maybe handy in future
fn validate_profile_name(name: &str) -> bool {
	true
}

pub async fn load_profiles() -> ProfilesMap {
	let file = get_profiles_file().await;
	let mut file_locked = file.lock().await;

	let mut buf = String::new();
	file_locked
		.read_to_string(&mut buf)
		.await
		.expect("Failed to read in profiles file");

	if buf.len() == 0 {
		return ProfilesMap::new();
	}

	let read_profiles: ProfilesMap =
		serde_json::from_str(&buf).expect("Failed to serialize profiles file");

	read_profiles
}

pub async fn save_profiles(profiles_map: &ProfilesMap) {
	let file = get_profiles_file().await;
	let mut file_locked = file.lock().await;

	let json = serde_json::to_string(&profiles_map).expect("Failed to convert profiles to json");

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
