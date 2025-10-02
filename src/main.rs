//#![windows_subsystem = "windows"]

use std::{
	collections::{HashMap, HashSet},
	ops::{Deref, DerefMut},
	path::{Path, PathBuf},
	sync::Arc,
};

use fltk::{browser::CheckBrowser, prelude::*, *};
use lazy_static::lazy_static;
use semver::Version;
use tokio::sync::{Mutex, RwLock};

use crate::api::BranchInfo;

mod api;
mod profiles;
mod syncer;
mod utils;

#[derive(Debug, Default, Clone)]
pub struct AppState {
	server_api_address: Option<String>,
	server_main_address: Option<String>,
	branch_name: Option<String>,
	mods_path: Option<PathBuf>,
	branch_info: Option<BranchInfo>,
	to_download_names: HashMap<String, bool>,
	to_delete_names: HashMap<String, bool>,
	profile_name: Option<String>,
}


#[derive(Debug, Clone)]
pub enum Events {
	// Gui events
	//CheckAddress,
	GetBranches,
	BranchesResult(Vec<String>),
	BranchError(String),
	GetMods,
	ModsResult(BranchInfo),
	ModsError(String),
	PathBrowse,
	PathSet,
	DownloadListUpdate,
	DeleteListUpdate,
	Download,
	Alert(String),

	// Download events
	ShowDownloadModal {
		total_size: u64,
	},
	DownloadNewFile {
		title: String,
		size: u64,
		count: usize,
		total_file_count: usize,
	},
	DownloadProgess {
		downloaded_chunk: usize,
	},
	DownloadSpeedMeter {
		bytes_per_s: f64,
	},
	DownloadStop,
	DownloadCancel,
	DeleteMods,

	// Menu events
	MenuSettings,
	MenuAbout,
	MenuHelp,
	MenuProfile(String),
	MenuNewProfile,
	MenuSaveProfile(String),
	MenuDeleteProfile,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &'static str = env!("CARGO_PKG_REPOSITORY");

const DEFAULT_PROFILE_NAME: &'static str = "default";

// TODO: make this setting
lazy_static! {
	static ref LABEL_ALIGN: enums::Align = enums::Align::Left | enums::Align::Inside;
}

// TODO:
// Should app_state.branch_info.mods be a hashmap instead of vec?
// Possible changes in profiles
//   - dont revert to default when deleting selected profile
//     - in this state there shouldnt be any red text
//     - pressing save should make popup to create new profile

#[tokio::main]
async fn main() {
	let app_state = Arc::new(RwLock::new(AppState::default()));

	let mut profiles_map = profiles::load_profiles().await;

	if !profiles_map.profile_exists(DEFAULT_PROFILE_NAME) {
		profiles_map.new_profile(
			DEFAULT_PROFILE_NAME,
			profiles::Profile::new("https://themoonbase.dnet.hu/minecraft", "", None),
		);
		profiles_map.set_last_profile_name(DEFAULT_PROFILE_NAME);
		profiles::save_profiles(&profiles_map).await;
	}

	let app = app::App::default();
	let widget_theme = fltk_theme::WidgetTheme::new(fltk_theme::ThemeType::Classic);
	widget_theme.apply();
	let widget_scheme = fltk_theme::WidgetScheme::new(fltk_theme::SchemeType::Fleet1);
	widget_scheme.apply();

	let (fltk_tx, fltk_rx) = app::channel::<Events>();
	let (progress_stop_tx, progress_stop_rx) = tokio::sync::mpsc::channel::<bool>(1);
	let progress_stop_rx = Arc::new(Mutex::new(progress_stop_rx));

	// Check if new version is avaliable
	if let Ok(repo_version) = api::get_repo_version().await {
		if Version::parse(VERSION).is_ok_and(|v| repo_version > v) {
			fltk_tx.send(Events::Alert(String::from(&format!(
				"Update avaliable. New version: {}!",
				repo_version
			))));
		}
	}

	// ----- Main window section  -----

	let mut main_wind = window::Window::default()
		.with_size(1000, 700)
		.with_label("Minecraft mod syncer");
	main_wind.make_resizable(true);

	// flex so menubar doesn't scale with window
	let mut flex = group::Flex::default().size_of_parent().column();
	let mut menubar = menu::MenuBar::default();
	menubar.set_frame(enums::FrameType::ThinUpBox);
	flex.fixed(&menubar, 20);
	flex.end();

	let mut flex = group::Flex::default().size_of_parent().column();

	flex.fixed(&frame::Frame::default(), 10);

	let mut input_flex = group::Flex::default();
	let server_ip_label = frame::Frame::default()
		.with_label("Server ip/domain: ")
		.with_align(*LABEL_ALIGN);
	let mut server_ip_input = input::Input::default().with_align(enums::Align::Right);
	let mut ip_ok_button = button::Button::default().with_label("OK");
	input_flex.end();

	let mut branch_flex = group::Flex::default();
	let branch_label = frame::Frame::default()
		.with_label("Branch: ")
		.with_align(*LABEL_ALIGN);
	let mut branch_chooser = menu::Choice::default();
	branch_flex.end();

	let mut mod_dir_flex = group::Flex::default();
	let mod_dir_label = frame::Frame::default()
		.with_label("Mods directory: ")
		.with_align(*LABEL_ALIGN);
	let mut mods_path_input = input::FileInput::default();
	let mut mods_path_button = button::Button::default().with_label("Browse");
	mod_dir_flex.end();

	flex.fixed(&frame::Frame::default(), 10);

	let info_flex = group::Flex::default().size_of_parent().row();
	let mut download_list = browser::CheckBrowser::default()
		.with_label("To download")
		.with_align(enums::Align::Top);
	let mut delete_list = browser::CheckBrowser::default()
		.with_label("To delete")
		.with_align(enums::Align::Top);
	info_flex.end();

	let mut download_but = button::Button::default().with_label("Download");

	main_wind.end();

	if LABEL_ALIGN.contains(enums::Align::Right) {
		let width = server_ip_label.measure_label().0 + 15;
		input_flex.fixed(&server_ip_label, width);
		branch_flex.fixed(&branch_label, width);
		mod_dir_flex.fixed(&mod_dir_label, width);
	} else {
		let width = server_ip_label.measure_label().0;
		input_flex.fixed(&server_ip_label, width);
		branch_flex.fixed(&branch_label, width);
		mod_dir_flex.fixed(&mod_dir_label, width);
	}

	input_flex.fixed(&ip_ok_button, 60);
	mod_dir_flex.fixed(&mods_path_button, 60);

	flex.fixed(&menubar, 30);
	flex.fixed(&input_flex, 30);
	flex.fixed(&branch_flex, 30);
	flex.fixed(&mod_dir_flex, 30);
	flex.fixed(&download_but, 30);

	flex.set_margin(30);
	flex.end();

	download_list.clear_visible_focus();
	delete_list.clear_visible_focus();

	server_ip_input.emit(fltk_tx, Events::GetBranches);
	ip_ok_button.emit(fltk_tx, Events::GetBranches);
	branch_chooser.emit(fltk_tx, Events::GetMods);
	download_but.emit(fltk_tx, Events::Download);
	mods_path_button.emit(fltk_tx, Events::PathBrowse);
	mods_path_input.emit(fltk_tx, Events::PathSet);
	download_list.emit(fltk_tx, Events::DownloadListUpdate);
	delete_list.emit(fltk_tx, Events::DeleteListUpdate);

	server_ip_input.set_trigger(enums::CallbackTrigger::EnterKeyAlways);
	mods_path_input.set_trigger(enums::CallbackTrigger::EnterKeyAlways);
	download_list.set_trigger(enums::CallbackTrigger::Changed);
	delete_list.set_trigger(enums::CallbackTrigger::Changed);

	// TODO: setting for auto save on exit
	// TODO: dont use sleep
	// INFO: save current profile before quiting
	main_wind.set_callback(move |_| {
		fltk_tx.send(Events::MenuSaveProfile(String::from("")));
		tokio::spawn(async {
			tokio::time::sleep(std::time::Duration::from_millis(50)).await;
			app::awake_callback(|| app::quit());
		});
	});
	main_wind.set_trigger(enums::CallbackTrigger::Closed);

	menubar.add_emit(
		"&File/Preferences",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuSettings,
	);
	menubar.add_emit(
		"&Help/About",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuAbout,
	);
	menubar.add_emit(
		"&Help/Help",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuHelp,
	);
	{
		let mut profile_names = profiles_map.get_profile_names();
		profile_names.sort();

		for profile_name in profile_names {
			if profile_name == DEFAULT_PROFILE_NAME {
				continue;
			}

			menubar.add_emit(
				&format!("&File/Profiles/{}", &profile_name),
				enums::Shortcut::None,
				menu::MenuFlag::Normal,
				fltk_tx,
				Events::MenuProfile(String::from(profile_name)),
			);
		}
	}
	menubar.add_emit(
		&format!("&File/Profiles/{}", DEFAULT_PROFILE_NAME),
		enums::Shortcut::None,
		menu::MenuFlag::MenuDivider,
		fltk_tx,
		Events::MenuProfile(String::from(DEFAULT_PROFILE_NAME)),
	);
	menubar.add_emit(
		"&File/Profiles/New",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuNewProfile,
	);
	menubar.add_emit(
		"&File/Profiles/Save",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuSaveProfile(String::from("")),
	);
	menubar.add_emit(
		"&File/Profiles/Delete",
		enums::Shortcut::None,
		menu::MenuFlag::Normal,
		fltk_tx,
		Events::MenuDeleteProfile,
	);

	fltk_tx.send(Events::MenuProfile(String::from(
		profiles_map.get_last_profile_name(),
	)));

	main_wind.show();

	// ----- Download dialog section  -----

	let mut download_wind = window::Window::default()
		.with_size(400, 250)
		.with_label("Downloading ...");
	let mut download_flex = group::Flex::default()
		.size_of_parent()
		.with_type(group::FlexType::Column);

	let mut filename_label =
		frame::Frame::default().with_align(enums::Align::Left | enums::Align::Inside);

	let progress_flex = group::Flex::default().with_type(group::FlexType::Row);
	let mut download_speed_label =
		frame::Frame::default().with_align(enums::Align::Left | enums::Align::Inside);
	let mut file_count_label =
		frame::Frame::default().with_align(enums::Align::Right | enums::Align::Inside);
	progress_flex.end();

	let mut current_progress = misc::Progress::default();
	let mut total_progress = misc::Progress::default();

	let mut cancel_button = button::Button::default().with_label("Cancel");

	current_progress.set_selection_color(enums::Color::Green);
	total_progress.set_selection_color(enums::Color::Green);

	download_flex.set_spacing(10);
	download_flex.set_margin(20);
	download_flex.fixed(&filename_label, 30);
	download_flex.fixed(&progress_flex, 30);
	download_flex.fixed(&current_progress, 30);
	download_flex.fixed(&total_progress, 30);
	download_flex.fixed(&cancel_button, 30);

	download_wind.set_trigger(enums::CallbackTrigger::Closed);

	download_wind.emit(fltk_tx, Events::DownloadCancel);
	cancel_button.emit(fltk_tx, Events::DownloadCancel);

	download_flex.end();
	download_wind.make_modal(true);
	download_wind.end();

	// ----- About dialog section  -----

	let mut about_win = window::Window::default()
		.with_size(500, 100)
		.with_label("About");
	let mut about_flex = group::Flex::default()
		.with_type(group::FlexType::Column)
		.size_of_parent();

	frame::Frame::default()
		.with_label(&format!("Developer: {}", env!("CARGO_PKG_AUTHORS")))
		.with_align(enums::Align::Left | enums::Align::Inside);
	frame::Frame::default()
		.with_label(&format!("Version: {}", VERSION))
		.with_align(enums::Align::Left | enums::Align::Inside);

	let mut link_flex = group::Flex::default().with_type(group::FlexType::Row);

	let link_label = frame::Frame::default()
		.with_label("Repository:")
		.with_align(enums::Align::Left | enums::Align::Inside);
	let mut link_button = button::Button::default()
		.with_label(&REPOSITORY)
		.with_align(enums::Align::Left | enums::Align::Inside);

	link_button.clear_visible_focus();
	link_button.set_frame(enums::FrameType::NoBox);
	link_button.set_label_color(enums::Color::Blue);
	link_button.set_label_font(enums::Font::HelveticaItalic);
	link_button.set_callback(|_| {
		let _ = fltk::utils::open_uri(&REPOSITORY);
	});

	link_flex.fixed(&link_label, link_label.measure_label().0);
	link_flex.end();

	about_flex.set_spacing(10);
	about_flex.set_margin(20);

	about_flex.end();
	about_win.end();

	// ----- Event handling section  -----

	while app.wait() {
		if let Some(val) = fltk_rx.recv() {
			match val {
				Events::GetBranches => {
					let mut app_state_locked = app_state.write().await;

					let address = server_ip_input.value().to_string();

					// TODO: update if branches changed (this line skips that)
					// INFO: returns if current address is the same as the previous
					if app_state_locked
						.server_main_address
						.as_ref()
						.is_some_and(|e| *e == address)
					{
						fltk_tx.send(Events::GetMods);
						continue;
					}

					delete_list.clear();
					download_list.clear();
					branch_chooser.clear();

					app_state_locked.to_delete_names.clear();
					app_state_locked.to_download_names.clear();
					app_state_locked.branch_info = None;
					app_state_locked.branch_name = None;
					app_state_locked.server_api_address = None;
					app_state_locked.server_main_address = None;

					if address.len() == 0 {
						continue;
					}

					// TODO: only set these after checking if address works
					app_state_locked.server_api_address = Some(address.clone() + "/api");
					app_state_locked.server_main_address = Some(address);

					let fltk_tx = fltk_tx.clone();
					let app_state = app_state.clone();
					tokio::spawn(async move {
						let app_state_locked = app_state.read().await;
						let api_path = app_state_locked.server_api_address.as_ref().unwrap();

						match api::get_branch_names(api_path).await {
							Ok(branch_names) => {
								fltk_tx.send(Events::BranchesResult(branch_names));
							}
							Err(err) => {
								println!("Cannot get branch names. {}", err);
								fltk_tx.send(Events::BranchError(err.to_string()));
							}
						}
					});
				}
				Events::BranchesResult(branch_names) => {
					let app_state_locked = app_state.read().await;

					println!("Got branches: {:?}", branch_names);

					for branch_name in branch_names {
						branch_chooser.add_choice(&branch_name);
					}

					branch_chooser.set_value(0);

					// this is for profile switching with address change
					if let Some(profile) = app_state_locked
						.profile_name
						.as_ref()
						.and_then(|v| profiles_map.get_profile(v))
					{
						let i = branch_chooser.find_index(&profile.branch);
						if i >= 0 {
							branch_chooser.set_value(i);
						}
					}

					fltk_tx.send(Events::GetMods);
				}
				Events::BranchError(err) => {
					branch_chooser.clear();
					branch_chooser.set_damage(true);

					fltk_tx.send(Events::Alert(format!("Failed to get branches. {}", err)));
				}
				Events::GetMods => {
					let mut app_state_locked = app_state.write().await;

					let mods_path_str = mods_path_input.value();
					let dir = PathBuf::from(&mods_path_str);

					if !syncer::is_mods_folder(&dir) {
						fltk_tx.send(Events::Alert(String::from(
							"Selected folder isn't minecraft mods folder!",
						)));
						app_state_locked.mods_path = None;
						mods_path_input.set_value("");
					} else {
						app_state_locked.mods_path = Some(dir);
					}

					delete_list.clear();
					download_list.clear();

					app_state_locked.to_delete_names.clear();
					app_state_locked.to_download_names.clear();
					app_state_locked.branch_info = None;
					app_state_locked.branch_name = None;

					if app_state_locked.mods_path.is_none() {
						continue;
					}

					if let Some(branch) = branch_chooser.choice() {
						app_state_locked.branch_name = Some(branch);
					} else {
						continue;
					}

					let fltk_tx = fltk_tx.clone();
					let app_state = app_state.clone();
					tokio::spawn(async move {
						let app_state_locked = app_state.read().await;
						let branch_name = app_state_locked.branch_name.as_ref().unwrap();
						let api_path = app_state_locked.server_api_address.as_ref().unwrap();

						match api::get_mods_in_branch(api_path, branch_name).await {
							Ok(mods) => {
								fltk_tx.send(Events::ModsResult(mods));
							}
							Err(err) => {
								println!("Cannot get mods. {}", err);
								fltk_tx.send(Events::ModsError(err.to_string()));
							}
						}
					});
				}
				Events::ModsResult(branch_info) => {
					// INFO: dont let this event run multiple times at once
					let app_state_write_access = app_state.try_write();
					if app_state_write_access.is_err() {
						continue;
					}
					let mut app_state_locked = app_state_write_access.unwrap();

					delete_list.clear();
					download_list.clear();

					app_state_locked.to_delete_names.clear();
					app_state_locked.to_download_names.clear();

					app_state_locked.branch_info = Some(branch_info);

					let mods_pathbuf = match app_state_locked.mods_path.as_ref() {
						Some(p) => p,
						None => continue,
					};

					let local_mod_names = syncer::get_local_mods(&mods_pathbuf).unwrap();
					let remote_mods = &app_state_locked.branch_info.as_ref().unwrap().mods;

					let (to_deletes, to_delete_optionals) =
						syncer::get_mods_to_delete(remote_mods, &local_mod_names);
					let to_downloads = syncer::get_mods_to_download(remote_mods, &local_mod_names);

					let profile = profiles_map
						.get_profile(app_state_locked.profile_name.as_ref().unwrap())
						.unwrap();
					let keep_mods_branch = profile
						.keep_mods_in_branch
						.get(app_state_locked.branch_name.as_ref().unwrap());

					for to_delete in to_deletes.iter() {
						let is_checked = keep_mods_branch
							.as_ref()
							.and_then(|v| Some(!v.contains(to_delete)))
							.unwrap_or(true);

						delete_list.add(to_delete, is_checked);
						app_state_locked
							.to_delete_names
							.insert(to_delete.to_string(), is_checked);
					}

					for to_delete_optional in to_delete_optionals.iter() {
						let is_checked = false;

						delete_list.add(to_delete_optional, is_checked);
						app_state_locked
							.to_delete_names
							.insert(to_delete_optional.to_string(), is_checked);
					}

					for to_download in to_downloads.iter() {
						let is_checked = !to_download.is_optional;

						download_list.add(&to_download.name, is_checked);
						app_state_locked
							.to_download_names
							.insert(to_download.name.clone(), is_checked);
					}

					delete_list.set_damage(true);
					download_list.set_damage(true);
				}
				Events::ModsError(err) => {
					fltk_tx.send(Events::Alert(format!("Failed to get mods. {}", err)));
				}
				Events::Download => {
					let app_state = app_state.clone();
					let progress_stop_rx = progress_stop_rx.clone();

					tokio::spawn(async move {
						// INFO: this wont drop until download is complete
						// no need to drop() it manually, write wont be used until download completed
						let app_state_locked = app_state.read().await;

						let branch_info = match app_state_locked.branch_info.as_ref() {
							Some(branch_info) => branch_info,
							None => {
								fltk_tx.send(Events::Alert(String::from(
									"Please set server address (e.g. themoonbase.dnet.hu/minecraft)",
								)));
								return;
							}
						};
						let zip_file = &branch_info.zip;
						let mcmods = &branch_info.mods;

						if app_state_locked.mods_path.is_none() {
							fltk_tx.send(Events::Alert(format!(
								"Please set 'mods' folder path (e.g. {})!",
								syncer::get_os_default_mods_folder()
									.as_ref()
									.and_then(|e| e.to_str())
									.unwrap_or("whats your platform?")
							)));
							return;
						}

						let to_downloads: HashSet<&String> = app_state_locked
							.to_download_names
							.iter()
							.filter_map(|e| e.1.then_some(e.0))
							.collect();

						fltk_tx.send(Events::DeleteMods);

						let mcmods: Vec<&api::Mod> = mcmods
							.into_iter()
							.filter(|x| to_downloads.contains(&x.name))
							.collect();

						let total_size = mcmods.iter().fold(0, |acc, x| acc + x.size);
						let total_count = mcmods.len();

						// INFO: if zip is not present, download all files separately
						let zip_size = zip_file
							.is_present
							.then_some(zip_file.size)
							.unwrap_or_else(u64::max_value);

						// INFO: download zip even if it's bigger by 5% than files
						// TODO: generalize more
						if total_size > zip_size * 95 / 100 {
							fltk_tx.send(Events::ShowDownloadModal {
								total_size: zip_size,
							});

							syncer::download_zip(fltk_tx, progress_stop_rx, app_state.clone())
								.await;
						} else {
							fltk_tx.send(Events::ShowDownloadModal {
								total_size: total_size,
							});

							syncer::download_files(
								fltk_tx,
								progress_stop_rx,
								app_state.clone(),
								total_count,
							)
							.await;
						}
					});
				}
				Events::PathSet => {
					fltk_tx.send(Events::GetMods);
				}
				Events::PathBrowse => {
					if let Some(dir) = dialog::dir_chooser("Choose a directory", "", false) {
						mods_path_input.set_value(&dir);
						fltk_tx.send(Events::PathSet);
					}
				}
				Events::DownloadListUpdate => {
					let mut app_state_locked = app_state.write().await;

					let modname = download_list.text(download_list.value()).unwrap();
					let is_checked = download_list.checked(download_list.value());

					let remote_mods = &app_state_locked.branch_info.as_ref().unwrap().mods;

					// INFO: if item gets unchecked but not supposed to (aka it's required) then
					// give error and recheck the item
					if !is_checked
						&& !remote_mods
							.iter()
							.find(|&e| e.name == modname)
							.is_some_and(|v| v.is_optional)
					{
						download_list.set_checked(download_list.value());
						fltk_tx.send(Events::Alert(String::from("Cannot uncheck required mod!")));
						continue;
					}

					*app_state_locked
						.to_download_names
						.get_mut(&modname)
						.unwrap() = is_checked;
				}
				Events::DeleteListUpdate => {
					let mut app_state_locked = app_state.write().await;

					let modname = delete_list.text(delete_list.value()).unwrap();
					let is_checked = delete_list.checked(delete_list.value());

					*app_state_locked.to_delete_names.get_mut(&modname).unwrap() = is_checked;

					// INFO: skip changing keep mods if mod is optional
					if app_state_locked
						.branch_info
						.as_ref()
						.unwrap()
						.mods
						.iter()
						.find(|v| v.is_optional && v.name == modname)
						.is_some()
					{
						continue;
					}

					if let Some(profile) = app_state_locked
						.profile_name
						.as_ref()
						.and_then(|v| profiles_map.get_mut_profile(v))
					{
						if let Some(branch_name) = app_state_locked.branch_name.as_ref() {
							if !profile.keep_mods_in_branch.contains_key(branch_name) {
								profile
									.keep_mods_in_branch
									.insert(branch_name.clone(), Vec::new());
							}

							let mut keep_mods_branch =
								profile.keep_mods_in_branch.get_mut(branch_name).unwrap();

							let index = keep_mods_branch.value().iter().position(|n| *n == modname);

							if is_checked && index.is_some() {
								keep_mods_branch.swap_remove(index.unwrap());
							} else if !is_checked && index.is_none() {
								keep_mods_branch.push(modname.clone());
							}
						}
					}
				}
				Events::Alert(text) => {
					dialog::alert_default(&text);
				}

				// Download events
				Events::ShowDownloadModal { total_size } => {
					file_count_label.set_label(&"0/0");
					total_progress.set_label("Total progress 0%");
					total_progress.set_maximum(total_size as f64);
					total_progress.set_value(0.0);
					download_wind.show();
				}
				Events::DownloadNewFile {
					title,
					size,
					count,
					total_file_count,
				} => {
					filename_label.set_label(&title);
					file_count_label.set_label(&format!("{}/{}", count, total_file_count));
					download_speed_label.set_label("0 B/s");
					current_progress.set_value(0.0);
					current_progress.set_label("Current progress 0%");
					current_progress.set_maximum(size as f64);
				}
				// TODO: pass total, current downloaded chunk instead of calculating here
				Events::DownloadProgess { downloaded_chunk } => {
					// INFO: add chunk size to progress bars value

					current_progress.set_value(current_progress.value() + downloaded_chunk as f64);
					current_progress.set_label(&format!(
						"Current progress {:.2}%",
						current_progress.value() / current_progress.maximum() * 100.0
					));

					total_progress.set_value(total_progress.value() + downloaded_chunk as f64);
					total_progress.set_label(&format!(
						"Total progress {:.2}%",
						total_progress.value() / total_progress.maximum() * 100.0
					));
				}
				Events::DownloadSpeedMeter { bytes_per_s } => {
					download_speed_label.set_label(&utils::readable_bps(bytes_per_s));
				}
				Events::DownloadStop => {
					download_wind.hide();
					fltk_tx.send(Events::GetMods);
				}
				Events::DownloadCancel => {
					let _ = progress_stop_tx.send(true).await;
				}
				Events::DeleteMods => {
					let app_state = app_state.clone();

					tokio::spawn(async move {
						let app_state_locked = app_state.read().await;

						let mods_pathbuf = app_state_locked.mods_path.as_ref().unwrap();
						let to_deletes: HashSet<&String> = app_state_locked
							.to_delete_names
							.iter()
							.filter_map(|e| e.1.then_some(e.0))
							.collect();

						for to_delete in to_deletes {
							tokio::fs::remove_file(mods_pathbuf.join(to_delete))
								.await
								.unwrap();
						}
					});
				}

				// Menu events
				Events::MenuHelp => {}
				Events::MenuAbout => {
					about_win.show();
				}
				Events::MenuSettings => {}
				Events::MenuProfile(name) => {
					let mut app_state_locked = app_state.write().await;

					if let Some(prev_profile_name) = app_state_locked.profile_name.as_ref() {
						if let Some(mut prev_item) =
							menubar.find_item(&format!("&File/Profiles/{}", prev_profile_name))
						{
							prev_item.set_label_color(enums::Color::Black);
						}
					}

					let mut item = menubar
						.find_item(&format!("&File/Profiles/{}", &name))
						.unwrap();
					item.set_label_color(enums::Color::Red);

					let profile = profiles_map.get_profile(&name).unwrap();

					server_ip_input.set_value(&profile.address);

					if profile.mods_path.len() == 0 {
						if let Some(mod_folder) = syncer::try_get_mods_folder() {
							mods_path_input.set_value(
								mod_folder
									.canonicalize() // INFO: windows returns UNC path (e.g. \\?\C:\)
									.unwrap_or_default()
									.to_str()
									.unwrap_or_default()
									.strip_prefix("\\\\?\\")
									.unwrap_or_default(),
							);
						}
					} else {
						mods_path_input.set_value(&profile.mods_path);
					}

					// this is for profile switching without address change
					let i = branch_chooser.find_index(&profile.branch);
					if i >= 0 {
						branch_chooser.set_value(i);
					}

					fltk_tx.send(Events::GetBranches);

					app_state_locked.profile_name = Some(name);
				}
				Events::MenuNewProfile => {
					let mut app_state_locked = app_state.write().await;

					let name = dialog::input_default("Name for new profile:", "")
						.map(|v| String::from(v.trim()));

					if name.is_none() {
						continue;
					}

					let name = name.unwrap();

					if name.len() == 0 {
						fltk_tx.send(Events::Alert(String::from("Name cannot be empty")));
						continue;
					}

					if profiles_map.profile_exists(&name) {
						fltk_tx.send(Events::Alert(format!("Profile '{}' already exists", &name)));
						continue;
					}

					fltk_tx.send(Events::MenuSaveProfile(name.clone()));

					let default_profile_index =
						menubar.find_index(&format!("&File/Profiles/{}", DEFAULT_PROFILE_NAME));

					let new_index = menubar.insert_emit(
						default_profile_index,
						&name,
						enums::Shortcut::None,
						menu::MenuFlag::Normal,
						fltk_tx,
						Events::MenuProfile(name.clone()),
					);

					if let Some(prev_profile_name) = app_state_locked.profile_name.as_ref() {
						if let Some(mut prev_item) =
							menubar.find_item(&format!("&File/Profiles/{}", prev_profile_name))
						{
							prev_item.set_label_color(enums::Color::Black);
						}
					}

					let mut item = menubar.at(new_index).unwrap();
					item.set_label_color(enums::Color::Red);

					dialog::message_default(&format!("Successfully created '{}' profile", &name));

					app_state_locked.profile_name = Some(name);

					ip_ok_button.do_callback();
				}
				Events::MenuDeleteProfile => {
					let app_state_locked = app_state.read().await;

					let name = dialog::input_default("Name of profile to delete:", "")
						.map(|v| String::from(v.trim()));

					if name.is_none() {
						continue;
					}

					let name = name.unwrap();

					if name == DEFAULT_PROFILE_NAME {
						fltk_tx.send(Events::Alert(String::from("Good try")));
						continue;
					}

					if !profiles_map.profile_exists(&name) {
						fltk_tx.send(Events::Alert(format!("Profile '{}' doesn't exists", &name)));
						continue;
					}

					profiles_map.delete_profile(&name);

					let i = menubar.find_index(&format!("&File/Profiles/{}", &name));
					menubar.remove(i);

					if app_state_locked
						.profile_name
						.as_ref()
						.is_some_and(|v| *v == name)
					{
						fltk_tx.send(Events::MenuProfile(String::from(DEFAULT_PROFILE_NAME)));
					}

					profiles::save_profiles(&profiles_map).await;

					dialog::message_default(&format!("Successfully deleted '{}' profile", &name));
				}
				Events::MenuSaveProfile(name) => {
					let app_state_locked = app_state.read().await;

					let download_address = app_state_locked
						.server_main_address
						.as_deref()
						.unwrap_or_default();
					let branch_name = app_state_locked.branch_name.as_deref().unwrap_or_default();
					let mods_pathbuf = app_state_locked
						.mods_path
						.as_deref()
						.unwrap_or(Path::new(""))
						.to_str()
						.unwrap();

					// INFO: if name is not empty save profile as new, else use current profile
					if name.len() > 0 {
						let profile = profiles::Profile::new(
							download_address,
							mods_pathbuf,
							Some(String::from(branch_name)),
						);

						profiles_map.set_last_profile_name(&name);
						profiles_map.new_profile(name, profile);
					} else {
						let profile_name = app_state_locked.profile_name.as_ref().unwrap();

						profiles_map.set_last_profile_name(profile_name.clone());

						let mut profile = profiles_map.get_mut_profile(profile_name).unwrap();

						profile.address = String::from(download_address);
						profile.branch = String::from(branch_name);
						profile.mods_path = String::from(mods_pathbuf);
					}

					profiles::save_profiles(&profiles_map).await;
				}
			}
		}
	}
}
