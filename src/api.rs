use std::{sync::LazyLock, time::Duration};

use reqwest::{Client, Response, Result, header};
use semver::Version;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ZipFile {
	pub size: u64,
	pub is_present: bool,
	pub mod_date: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mod {
	pub name: String,
	pub mod_date: f64,
	pub size: u64,
	pub is_optional: bool,
}

pub type BranchNames = Vec<String>;
pub type Mods = Vec<Mod>;

#[derive(Debug, Clone, Deserialize)]
pub struct BranchInfo {
	pub mods: Mods,
	pub zip: ZipFile,
}

fn get_client() -> &'static Client {
	static CLIENT: LazyLock<Client> = LazyLock::new(|| {
		Client::builder()
			.connect_timeout(Duration::from_secs(2))
			.build()
			.unwrap()
	});

	&CLIENT
}

/// Get this project's latest released version
pub async fn get_repo_version() -> std::result::Result<Version, Box<dyn std::error::Error>> {
	let path = format!("{}/releases/latest", env!("CARGO_PKG_REPOSITORY"));
	let res = Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.connect_timeout(Duration::from_secs(2))
		.build()
		.unwrap()
		.head(path)
		.send()
		.await?;

	let ver_string = res
		.headers()
		.get(header::LOCATION)
		.unwrap()
		.to_str()
		.unwrap()
		.split("/")
		.last()
		.unwrap();

	Ok(Version::parse(ver_string)?)
}

pub async fn website_exists(api_address: &str) -> Result<bool> {
	let path = format!("{}/mods", api_address);
	let res = get_client().head(path).send().await?;

	Ok(res.status().is_success())
}

pub async fn get_branch_names(api_address: &str) -> Result<BranchNames> {
	let path = format!("{}/mods", api_address);
	let res = get_client()
		.get(path)
		.send()
		.await?
		.json::<BranchNames>()
		.await?;

	Ok(res)
}

pub async fn get_mods_in_branch(api_address: &str, branch_name: &str) -> Result<BranchInfo> {
	let path = format!("{}/mods/{}", api_address, branch_name);
	let res = get_client()
		.get(path)
		.send()
		.await?
		.json::<BranchInfo>()
		.await?;

	Ok(res)
}

pub async fn request_mod(
	main_address: &str,
	branch_name: &str,
	file_name: &str,
) -> Result<Response> {
	let path = format!("{}/mods/{}/{}", main_address, branch_name, file_name);
	let res = get_client().get(path).send().await?;

	Ok(res)
}

pub async fn request_mod_zip(main_address: &str, branch_name: &str) -> Result<Response> {
	let path = format!("{}/mods/{}", main_address, branch_name);
	let res = get_client().get(path).send().await?;

	Ok(res)
}
