use crate::util;
use crate::util::backup::get_most_recent;
use std::fs;
use std::path::PathBuf;
use rusqlite::Connection;
use rusqlite::params;

pub(crate) fn full_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
	database_connection: &Connection,
) -> () {
	//get vec of paths to all files to backup
	let files = util::dir_operation::get_files_recursive(path_to_world);

	let world_path_cannonicalized = path_to_world
		.canonicalize()
		.expect("Should be able to cannonicalize path to world");

	let path_to_backup_dir = path_to_backups_dir.join(current_time); //path to the directory we are actually backing up to

	//create directory to store new backup
	util::backup::init(
		&path_to_backup_dir,
		&files
			.iter()
			.map(|file| util::trim_path(&file, &world_path_cannonicalized))
			.collect::<Vec<PathBuf>>(),
		current_time,
		database_connection
	);

	let insert_file = format!("INSERT INTO \"{}\" (timestamp, path) VALUES (\"{}\", ?1);", current_time, current_time); 

	//for each file to backup,
	for file in files {
		//trim path to world level
		let trimmed_file_path = util::trim_path(&file, &world_path_cannonicalized);

		let file_destination = path_to_backup_dir.join(&trimmed_file_path);

		//copy the file to the backup dir
		fs::copy(
			&file,
			&file_destination,
		)
		.expect("Should be able to copy file");

		//write it to the manifest
		database_connection.execute(&insert_file, params!(trimmed_file_path.to_string_lossy())).expect("Should be able to insert file into manifest");
	}
}

pub(crate) fn iterative_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
	database_connection: &Connection,
) -> () {
	let path_to_backup = path_to_backups_dir.join(current_time);

	//get the timestamp of the previous backup by getting the table
	let previous_backup_timestamp: String = get_most_recent(database_connection).expect("Should be a previous backup");

	let previous_backup_time = util::timestamp::to_OffsetDateTime(&previous_backup_timestamp);

	let files = util::dir_operation::get_files_recursive(&path_to_world); //get the paths of every file to backup

	let path_to_world_cannonicalized = path_to_world
		.canonicalize()
		.expect("Should be able to cannonicalize path to world");

	//create directory to store new backup
	util::backup::init(
		&path_to_backup,
		&files
			.iter()
			.map(|file| util::trim_path(&file, &path_to_world_cannonicalized))
			.collect::<Vec<PathBuf>>(),
		current_time,
		database_connection
	);

	let get_previous_path = format!("SELECT DISTINCT timestamp, path FROM \"{}\" WHERE path = ?1;", previous_backup_timestamp); //path should only appear once, but just in case use distinct
	let insert_file = format!("INSERT INTO \"{}\" (timestamp, path) VALUES (?1, ?2);", current_time);

	for file in files {
		//for each file,
		//get the timestamp of the file's last modification
		let modified_timestamp = util::timestamp::get_timestamp(&file);

		let trimmed_file_path = util::trim_path(&file, &path_to_world_cannonicalized);

		//compare last modification timestamp to last backup timestamp to determine if a new copy needs to be taken
		match modified_timestamp >= previous_backup_time {
			true => {
				//has been modified since last backup, needs to be updated
				fs::copy(&file, path_to_backup.join(&trimmed_file_path))
					.expect("Should be able to copy files from world");

				//insert path
				database_connection.execute(&insert_file, params!(current_time, trimmed_file_path.to_string_lossy())).expect("Should be able to insert file into manifest");
			}
			false => {
				//hasn't been modified since last backup, insert path to older backup of the file in manifest
				let path: (String, String) = database_connection.query_row(&get_previous_path, params!(trimmed_file_path.to_string_lossy()), |row| Ok((row.get("timestamp")?, row.get("path")?))).expect("Should be able to read old path");
				database_connection.execute(&insert_file, params!(path.0, path.1)).expect("Should be able to insert file into manifest");
			}
		}
	}
}
