use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use zip::ZipArchive;

use crate::api::{self, Mod};
use crate::{AppState, Events};

pub type ModNames = Vec<String>;
pub type Mods = Vec<Mod>;

/// Get the offical minecraft launcher's minecraft folder for the os
/// This doesn't checks if folder exists
pub fn get_os_default_mods_folder() -> Option<PathBuf> {
	if cfg!(target_os = "windows") {
		Some(dirs::config_dir().unwrap().join(".minecraft").join("mods"))
	} else if cfg!(target_os = "linux") {
		Some(dirs::home_dir().unwrap().join(".minecraft").join("mods"))
	} else if cfg!(target_os = "macos") {
		Some(dirs::config_dir().unwrap().join("minecraft").join("mods"))
	} else {
		None
	}
}

/// Returns wether path is valid mods folder (ends in mods, exists and accessible)
pub fn is_mods_folder(path: &Path) -> bool {
	if !path.file_name().is_some_and(|x| x == "mods") || !path.is_dir() {
		return false;
	}

	true
}

/// Tries to get a valid mods folder automaticly
/// First checks if ./mods exists, then ./.minecraft/mods lastly the offical launcher's minecraft path
pub fn try_get_mods_folder() -> Option<PathBuf> {
	let current_path = Path::new(".");

	let mods_folder = current_path.join("mods");
	if let Ok(true) = std::fs::exists(&mods_folder) {
		return Some(mods_folder);
	}

	let mods_folder = current_path.join(".minecraft").join("mods");
	if let Ok(true) = std::fs::exists(&mods_folder) {
		return Some(mods_folder);
	}

	get_os_default_mods_folder().filter(|v| is_mods_folder(v))
}

/// Get locally installed mod names
/// Only collects .jar files (case-insensitive)
pub fn get_local_mods(mod_dir_path: &Path) -> Result<ModNames> {
	let mod_names: ModNames = mod_dir_path
		.read_dir()?
		.filter_map(Result::ok)
		.filter(|file| file.path().is_file())
		.filter(|file| {
			file.path()
				.extension()
				.and_then(|ext| ext.to_str())
				.map(|ext| ext.eq_ignore_ascii_case("jar"))
				.unwrap_or(false)
		})
		.map(|file| file.file_name().to_string_lossy().into_owned())
		.collect();

	Ok(mod_names)
}

/// Get all mods that are in remote_mods but not in local_mods
pub fn get_mods_to_download(remote_mods: &Mods, local_mods: &ModNames) -> Mods {
	let local_mod_names: HashSet<&String> = HashSet::from_iter(local_mods);

	remote_mods
		.iter()
		.filter(|e| !local_mod_names.contains(&e.name))
		.cloned()
		.collect()
}

/// Get all mod names that are in local_mods but not in remote_mods
pub fn get_mods_to_delete(remote_mods: &Mods, local_mods: &ModNames) -> ModNames {
	let remote_mod_names: HashSet<&String> =
		HashSet::from_iter(remote_mods.iter().map(|e| &e.name));

	local_mods
		.iter()
		.filter(|e| !remote_mod_names.contains(e))
		.cloned()
		.collect()
}

pub async fn download_files(
	fltk_tx: fltk::app::Sender<Events>,
	progress_stop_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<bool>>>,
	app_state: Arc<RwLock<AppState>>,
	total_count: usize,
) {
	let app_state_locked = app_state.read().await;

	let branch_name = app_state_locked.branch_name.as_ref().unwrap();
	let download_address = app_state_locked.server_main_address.as_ref().unwrap();
	let branch_info = app_state_locked.branch_info.as_ref().unwrap();
	let mcmods = &branch_info.mods;
	let mods_pathbuf = app_state_locked.mods_path.as_ref().unwrap();

	let to_downloads: HashSet<&String> = app_state_locked
		.to_download_names
		.iter()
		.filter_map(|e| e.1.then(|| e.0))
		.collect();

	let mcmods: Vec<&api::Mod> = mcmods
		.into_iter()
		.filter(|x| to_downloads.contains(&x.name))
		.collect();

	for (i, mcmod) in mcmods.iter().enumerate() {
		let res = api::request_mod(&download_address, &branch_name, &mcmod.name).await;
		match res {
			Ok(res) => {
				if !res.status().is_success() {
					println!("http status: {}", res.status().as_u16());
					return;
				}

				// TODO: change server zipping code
				let file_size = res.content_length().unwrap_or(u64::max_value());

				let path = mods_pathbuf.join(&mcmod.name);
				let file = tokio::fs::File::create(&path).await.unwrap();
				let mut file_out = tokio::io::BufWriter::new(file);

				// TODO: move total_count out of here
				fltk_tx.send(Events::DownloadNewFile {
					title: mcmod.name.clone(),
					size: file_size,
					count: i,
					total_file_count: total_count,
				});

				let mut stream = res.bytes_stream();
				let mut stopped = false;

				let mut prev_time = tokio::time::Instant::now();
				let check_ms = tokio::time::Duration::from_millis(500);
				let mut size_under_time = 0;
				let mut prev_bps = 0.0;

				let mut progress_stop_rx = progress_stop_rx.lock().await;
				while let Some(chunk) = stream.next().await {
					if let Ok(true) = progress_stop_rx.try_recv() {
						stopped = true;
						break;
					}

					// INFO: try again chunk
					if chunk.is_err() {
						continue;
					}

					let c = chunk.unwrap();
					let chunk_size = c.len();
					size_under_time += chunk_size;

					let now_time = tokio::time::Instant::now();
					let elapsed = now_time.duration_since(prev_time);
					if elapsed >= check_ms {
						let secs = elapsed.as_secs_f64();
						let bps = size_under_time as f64 / secs;

						if bps != prev_bps {
							fltk_tx.send(Events::DownloadSpeedMeter { bytes_per_s: bps });
							prev_bps = bps;
						}

						prev_time = now_time;
						size_under_time = 0;
					}

					file_out.write_all(&c).await.unwrap();

					fltk_tx.send(Events::DownloadProgess {
						downloaded_chunk: chunk_size,
					});
				}

				file_out.shutdown().await.unwrap();

				if stopped {
					fltk_tx.send(Events::DownloadStop);
					tokio::fs::remove_file(path).await.unwrap();
					return;
				}
			}
			Err(err) => {
				println!("error in download: {}", err);
			}
		}
	}

	fltk_tx.send(Events::DownloadStop);
}

pub async fn download_zip(
	fltk_tx: fltk::app::Sender<Events>,
	progress_stop_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<bool>>>,
	app_state: Arc<RwLock<AppState>>,
) {
	let app_state_locked = app_state.read().await;

	let branch_name = app_state_locked.branch_name.clone().unwrap();
	let download_address = app_state_locked.server_main_address.clone().unwrap();

	let res = api::request_mod_zip(&download_address, &branch_name).await;
	match res {
		Ok(res) => {
			if !res.status().is_success() {
				println!("http status: {}", res.status().as_u16());
				return;
			}

			// TODO: change server zipping code
			let file_size = res.content_length().unwrap_or(u64::max_value());

			let file_name = format!("{}.zip", &branch_name);
			let path = Path::new(".").join(&file_name);
			let file = tokio::fs::File::create(&path).await.unwrap();
			let mut file_out = tokio::io::BufWriter::new(file);

			// TODO: move total_count out of here
			fltk_tx.send(Events::DownloadNewFile {
				title: file_name,
				size: file_size,
				count: 1,
				total_file_count: 1,
			});

			let mut stream = res.bytes_stream();
			let mut stopped = false;

			let mut prev_time = tokio::time::Instant::now();
			let check_ms = tokio::time::Duration::from_millis(500);
			let mut size_under_time = 0;
			let mut prev_bps = 0.0;

			let mut progress_stop_rx_locked = progress_stop_rx.lock().await;
			while let Some(chunk) = stream.next().await {
				if let Ok(true) = progress_stop_rx_locked.try_recv() {
					stopped = true;
					break;
				}

				// INFO: try again chunk
				if chunk.is_err() {
					continue;
				}

				let c = chunk.unwrap();
				let chunk_size = c.len();
				size_under_time += chunk_size;

				let now_time = tokio::time::Instant::now();
				let elapsed = now_time.duration_since(prev_time);
				if elapsed >= check_ms {
					let secs = elapsed.as_secs_f64();
					let bps = size_under_time as f64 / secs;

					if bps != prev_bps {
						fltk_tx.send(Events::DownloadSpeedMeter { bytes_per_s: bps });
						prev_bps = bps;
					}

					prev_time = now_time;
					size_under_time = 0;
				}

				file_out.write_all(&c).await.unwrap();

				fltk_tx.send(Events::DownloadProgess {
					downloaded_chunk: chunk_size,
				});
			}

			file_out.shutdown().await.unwrap();

			if stopped {
				fltk_tx.send(Events::DownloadStop);
				tokio::fs::remove_file(path).await.unwrap();
				return;
			}

			// INFO: unzip locks progress_stop_rx too, so have to drop it now
			drop(progress_stop_rx_locked);
			unzip_mod_zip(&path, fltk_tx, progress_stop_rx.clone(), app_state.clone()).await;

			fltk_tx.send(Events::DownloadStop);
		}
		Err(err) => {
			println!("error in download: {}", err);
		}
	}
}

pub async fn unzip_mod_zip(
	zip_path: &Path,
	fltk_tx: fltk::app::Sender<Events>,
	progress_stop_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<bool>>>,
	app_state: Arc<RwLock<AppState>>,
) {
	let app_state_locked = app_state.read().await;

	let branch_info = app_state_locked.branch_info.as_ref().unwrap();
	let mcmods = &branch_info.mods;
	let mods_pathbuf = app_state_locked.mods_path.as_ref().unwrap();

	let to_downloads: HashSet<&String> = app_state_locked
		.to_download_names
		.iter()
		.filter_map(|e| e.1.then(|| e.0))
		.collect();

	let mcmods: Vec<&api::Mod> = mcmods
		.into_iter()
		.filter(|x| to_downloads.contains(&x.name))
		.collect();

	let total_size = mcmods.iter().fold(0, |acc, x| acc + x.size);

	// TODO: async?
	let zip_file = File::open(zip_path).unwrap();
	let zip_reader = BufReader::new(zip_file);
	let mut archive = ZipArchive::new(zip_reader).unwrap();

	fltk_tx.send(Events::ShowDownloadModal {
		total_size: total_size,
	});

	let file_count = archive.len();
	let mut stopped = false;
	let mut progress_stop_rx_locked = progress_stop_rx.lock().await;

	let mut buf = [0u8; 64 * 1024];

	for i in 0..file_count {
		let mut file = archive.by_index(i).unwrap();

		if !to_downloads.contains(&file.name().to_string()) {
			continue;
		}

		let outpath = match file.enclosed_name() {
			Some(path) => mods_pathbuf.join(path),
			None => continue,
		};

		fltk_tx.send(Events::DownloadNewFile {
			title: file.name().to_string(),
			size: file.size(),
			count: i,
			total_file_count: file_count,
		});

		let out_file = tokio::fs::File::create(&outpath).await.unwrap();
		let mut out_buf = tokio::io::BufWriter::new(out_file);

		let mut prev_time = tokio::time::Instant::now();
		let check_ms = tokio::time::Duration::from_millis(10);
		let mut size_since_update = 0;

		loop {
			if let Ok(true) = progress_stop_rx_locked.try_recv() {
				stopped = true;
				break;
			}

			match file.read(&mut buf) {
				Ok(size) => {
					if size == 0 {
						break;
					}

					out_buf.write_all(&buf[0..size]).await.unwrap();
					size_since_update += size;

					if prev_time.elapsed() > check_ms {
						fltk_tx.send(Events::DownloadProgess {
							downloaded_chunk: size_since_update,
						});

						prev_time = tokio::time::Instant::now();
						size_since_update = 0;
					}
				}
				Err(err) => {
					println!("failed to write out file from zip: {}", err);
					stopped = true;
					break;
				}
			}
		}

		out_buf.shutdown().await.unwrap();

		if stopped {
			tokio::fs::remove_file(outpath).await.unwrap();
			break;
		}
	}

	tokio::fs::remove_file(zip_path).await.unwrap();
}
