//#![windows_subsystem = "windows"]

use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

use fltk::{browser::CheckBrowser, prelude::*, *};
use lazy_static::lazy_static;
use tokio::sync::RwLock;

use tokio::sync::Mutex;

use crate::{api::BranchInfo, syncer::get_os_default_mods_folder};

mod api;
mod syncer;
mod utils;

#[derive(Debug)]
pub struct AppState {
    server_api_address: Option<String>,
    server_main_address: Option<String>,
    branch_name: Option<String>,
    mods_path: Option<PathBuf>,
    branch_info: Option<BranchInfo>,
    to_download_names: HashMap<String, bool>,
    to_delete_names: HashMap<String, bool>,
}

impl AppState {
    pub fn default() -> AppState {
        AppState {
            branch_name: None,
            server_api_address: None,
            server_main_address: None,
            mods_path: None,
            branch_info: None,
            to_delete_names: HashMap::new(),
            to_download_names: HashMap::new(),
        }
    }
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
    DeleteMods,
}

// INFO: debug only
lazy_static! {
    static ref LABEL_ALIGN: enums::Align = enums::Align::Left | enums::Align::Inside;
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(RwLock::new(AppState::default()));

    let keep_mods_file = syncer::get_keep_mods_file();
    let last_session_file = syncer::get_last_session_file();

    println!("{:?}", syncer::get_keep_mods(&keep_mods_file));

    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut main_wind = window::Window::default()
        .with_size(1000, 700)
        .with_label("Minecraft mod syncer");
    main_wind.make_resizable(true);

    let mut flex = group::Flex::default().size_of_parent().column();

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

    // INFO: debug only, testing alignemts
    if LABEL_ALIGN.intersection(enums::Align::Right) == enums::Align::Right {
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

    flex.fixed(&input_flex, 30);
    flex.fixed(&branch_flex, 30);
    flex.fixed(&mod_dir_flex, 30);
    flex.fixed(&download_but, 30);

    flex.set_margin(30);
    flex.end();

    download_list.clear_visible_focus();
    delete_list.clear_visible_focus();

    main_wind.end();

    // INFO: debug only
    server_ip_input.set_value("themoonbase.dnet.hu/minecraft");

    let (fltk_tx, fltk_rx) = app::channel::<Events>();
    let (progress_stop_tx, progress_stop_rx) = tokio::sync::mpsc::channel::<bool>(1);
    let progress_stop_rx = Arc::new(Mutex::new(progress_stop_rx));

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

    ip_ok_button.do_callback();

    if let Some(mod_folder) = syncer::try_get_mods_folder() {
        mods_path_input.set_value(mod_folder.to_str().unwrap_or(""));
        mods_path_input.do_callback();
    }

    main_wind.show();

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
    cancel_button.set_callback(move |_| {
        let progress_stop_tx = progress_stop_tx.clone();
        tokio::spawn(async move {
            progress_stop_tx.send(true).await.unwrap();
        });
    });

    current_progress.set_selection_color(enums::Color::Green);
    total_progress.set_selection_color(enums::Color::Green);

    download_flex.set_spacing(10);
    download_flex.set_margin(20);
    download_flex.fixed(&filename_label, 30);
    download_flex.fixed(&progress_flex, 30);
    download_flex.fixed(&current_progress, 30);
    download_flex.fixed(&total_progress, 30);
    download_flex.fixed(&cancel_button, 30);

    download_flex.end();
    download_wind.make_modal(true);
    download_wind.end();

    while app.wait() {
        if let Some(val) = fltk_rx.recv() {
            match val {
                Events::GetBranches => {
                    let address = server_ip_input.value().to_string();

                    let mut app_state_locked = app_state.write().await;

                    // TODO: update if branches changed (this line skips that)
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
                    println!("Got branches: {:?}", branch_names);
                    for branch_name in branch_names {
                        branch_chooser.add_choice(&branch_name);
                    }
                    branch_chooser.set_value(0);
                    fltk_tx.send(Events::GetMods);
                }
                Events::BranchError(err) => {
                    branch_chooser.clear();
                    branch_chooser.set_damage(true);

                    fltk_tx.send(Events::Alert(format!("Failed to get branches. {}", err)));
                }
                Events::GetMods => {
                    let mut app_state_locked = app_state.write().await;

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
                    let mut app_state_locked = app_state.write().await;

                    app_state_locked.branch_info = Some(branch_info);

                    let mods_pathbuf = match app_state_locked.mods_path.as_ref() {
                        Some(p) => p,
                        None => continue,
                    };

                    let local_mod_names = syncer::get_local_mods(&mods_pathbuf).unwrap();
                    let remote_mods = &app_state_locked.branch_info.as_ref().unwrap().mods;

                    let to_deletes = syncer::get_mods_to_delete(remote_mods, &local_mod_names);
                    let to_downloads = syncer::get_mods_to_download(remote_mods, &local_mod_names);

                    for to_delete in to_deletes.iter() {
                        let is_checked = true;
                        delete_list.add(to_delete, is_checked);
                        app_state_locked
                            .to_delete_names
                            .insert(to_delete.to_string(), is_checked);
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
                                fltk_tx.send(Events::Alert(String::from("Please set server address (e.g. themoonbase.dnet.hu/minecraft)")));
                                return;
                            }
                        };
                        let mods_pathbuf = match app_state_locked.mods_path.as_ref() {
                            Some(mods_path) => mods_path,
                            None => {
                                fltk_tx.send(Events::Alert(format!(
                                    "Please set 'mods' folder path (e.g. {})!",
                                    get_os_default_mods_folder()
                                        .as_ref()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("whats your platform?")
                                )));
                                return;
                            }
                        };
                        let zip_file = &branch_info.zip;
                        let mcmods = &branch_info.mods;

                        // TODO: fire event to save keep mods
                        let to_deletes: HashSet<&String> = app_state_locked
                            .to_delete_names
                            .iter()
                            .filter_map(|e| e.1.then(|| e.0))
                            .collect();
                        let to_downloads: HashSet<&String> = app_state_locked
                            .to_download_names
                            .iter()
                            .filter_map(|e| e.1.then(|| e.0))
                            .collect();

                        fltk_tx.send(Events::DeleteMods);

                        let mcmods: Vec<&api::Mod> = mcmods
                            .into_iter()
                            .filter(|x| to_downloads.contains(&x.name))
                            .collect();

                        let total_size = mcmods.iter().fold(0, |acc, x| acc + x.size);
                        let total_count = mcmods.len();

                        // INFO: if zip is not present, download all files separately
                        let zip_size = if zip_file.present {
                            zip_file.size
                        } else {
                            u64::max_value()
                        };

                        // INFO: download zip even if it's bigger by 2% than files
                        // TODO: generalize more
                        if total_size > zip_size * 98 / 100 {
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
                    let mut app_state_locked = app_state.write().await;
                    let mods_path_str = mods_path_input.value();
                    let dir = Path::new(&mods_path_str);

                    if !syncer::is_mods_folder(&dir) {
                        dialog::alert_default(&"Selected folder isn't minecraft mod folder!");
                        app_state_locked.mods_path = None;
                        mods_path_input.set_value("");
                        continue;
                    }

                    app_state_locked.mods_path = Some(dir.to_path_buf());

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

                    // TODO: Should 'mods' be a hashmap instead of vec?
                    // INFO: if item gets unchecked but not supposed to (aka it's required) then
                    // give error and recheck the item
                    if !is_checked
                        && !remote_mods
                            .iter()
                            .find(|&e| e.name == modname)
                            .unwrap()
                            .is_optional
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
                }
                Events::Alert(text) => {
                    dialog::alert_default(&text);
                }

                // Download events
                Events::ShowDownloadModal { total_size } => {
                    file_count_label.set_label(&"0/0");
                    total_progress.set_label("Total progess 0%");
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
                    current_progress.set_label("Current progess 0%");
                    current_progress.set_maximum(size as f64);
                }
                // TODO: pass total, current downloaded chunk instead of calculating here
                Events::DownloadProgess { downloaded_chunk } => {
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
                Events::DeleteMods => {
                    let app_state = app_state.clone();
                    tokio::spawn(async move {
                        let app_state_locked = app_state.read().await;
                        let mods_pathbuf = app_state_locked.mods_path.as_ref().unwrap();
                        let to_deletes: HashSet<&String> = app_state_locked
                            .to_delete_names
                            .iter()
                            .filter_map(|e| e.1.then(|| e.0))
                            .collect();

                        for to_delete in to_deletes {
                            tokio::fs::remove_file(mods_pathbuf.join(to_delete))
                                .await
                                .unwrap();
                        }
                    });
                }
            }
        }
    }
}
