use reqwest::{Response, Result};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZipFile {
	pub size: u64,
	pub is_present: bool,
	pub mod_date: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BranchInfo {
	pub mods: Mods,
	pub zip: ZipFile,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Mod {
	pub name: String,
	pub mod_date: f64,
	pub size: u64,
	pub is_optional: bool,
}

pub type BranchNames = Vec<String>;
pub type Mods = Vec<Mod>;

// TODO: try http and if doesnt work try https
const HTTP_TYPE: &str = "https://";

pub async fn website_exists(api_address: &str) -> Result<bool> {
	let path = format!("{}{}/mods", HTTP_TYPE, api_address);
	let res = reqwest::get(path).await?;

	Ok(res.status().is_success())
}

pub async fn get_branch_names(api_address: &str) -> Result<BranchNames> {
	let path = format!("{}{}/mods", HTTP_TYPE, api_address);
	let res = reqwest::get(path).await?.json::<BranchNames>().await?;

	Ok(res)
}

pub async fn get_mods_in_branch(api_address: &str, branch_name: &str) -> Result<BranchInfo> {
	let path = format!("{}{}/mods/{}", HTTP_TYPE, api_address, branch_name);
	let res = reqwest::get(path).await?.json::<BranchInfo>().await?;

	Ok(res)
}

pub async fn request_mod(
	main_address: &str,
	branch_name: &str,
	file_name: &str,
) -> Result<Response> {
	let path = format!(
		"{}{}/mods/{}/{}",
		HTTP_TYPE, main_address, branch_name, file_name
	);
	let res = reqwest::get(path).await?;

	Ok(res)
}

pub async fn request_mod_zip(main_address: &str, branch_name: &str) -> Result<Response> {
	let path = format!("{}{}/mods/{}", HTTP_TYPE, main_address, branch_name);
	let res = reqwest::get(path).await?;

	Ok(res)
}
