use crate::util;
use std::fs;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

pub(crate) fn full_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
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
	);

	for file in files {
		//for each file to backup,
		fs::copy(
			&file,
			&path_to_backup_dir.join(util::trim_path(&file, &world_path_cannonicalized)),
		)
		.expect("Should be able to copy file");
	}
}

pub(crate) fn iterative_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
) -> () {
	let path_to_backup = path_to_backups_dir.join(current_time);

	let path_to_most_recent_backup = util::backup::get_most_recent(&path_to_backups_dir)
		.expect("Should be at least one backup in the backup dir");

	//get the timestamp of the backup
	let most_recent_backup_timestamp =
		util::timestamp::to_OffsetDateTime(util::get_file_name_as_str(&path_to_most_recent_backup));

	let files = util::dir_operation::get_files_recursive(&path_to_world); //get the paths of every file to backup

	let path_to_world_cannonicalized = path_to_world
		.canonicalize()
		.expect("Should be able to cannonicalize path to world");

	let path_to_backups_dir_cannonicalized = path_to_backups_dir
		.canonicalize()
		.expect("Should be able to cannonicalize path to backups directory");

	let prev_manifest = util::backup::read_manifest(&path_to_most_recent_backup);

	//create directory to store new backup. MUST go after finding the most recent backup because if not the most recent check will fail
	util::backup::init(
		&path_to_backup,
		&files
			.iter()
			.map(|file| util::trim_path(&file, &path_to_world_cannonicalized))
			.collect::<Vec<PathBuf>>(),
	);

	//create writer to new manifest csv
	let mut csv_writer = BufWriter::new(
		OpenOptions::new()
			.create(true)
			.write(true)
			.open(path_to_backup.join("manifest.csv"))
			.expect("Should be able to open the manifest"),
	);

	for file in files {
		//for each file,
		//get the timestamp of the file's last modification
		let modified_timestamp = util::timestamp::get_timestamp(&file);

		let trimmed_file_path = util::trim_path(&file, &path_to_world_cannonicalized);

		//compare last modification timestamp to last backup timestamp to determine if a new copy needs to be taken
		match modified_timestamp >= most_recent_backup_timestamp {
			true => {
				//has been modified since last backup, needs to be updated
				fs::copy(&file, path_to_backup.join(trimmed_file_path))
					.expect("Should be able to copy files from world");
			}
			false => {
				//hasn't been modified since last backup, insert path to older backup of the file in csv
				//check previous backup directory for the file
				if path_to_most_recent_backup.join(&trimmed_file_path).exists() {
					//previous backup has the file, so put a reference to it
					csv_writer
						.write_all(
							format!(
								"{},",
								PathBuf::from(util::get_file_name_as_str(
									&path_to_most_recent_backup
								))
								.join(&trimmed_file_path)
								.to_str()
								.expect("Should be able to convert path to str")
							)
							.as_bytes(),
						)
						.expect("Should be able to write to manifest");
				} else if let Some(path) = util::backup::file_in_manifest(&trimmed_file_path, &prev_manifest) { //check previous backup manifest for the file
					//found the path in the old manifest
					//write the path we found to the new manifest
					csv_writer
						.write_all(
							format!(
								"{},",
								path.to_str().expect("Should be able convert path to str")
							)
							.as_bytes(),
						)
						.expect("could not write to manifest");
				} else {
					//something screwy is going on. copy the file and move on
					println!("unexpected error, copied and continued");
					fs::copy(file, path_to_backup.join(&trimmed_file_path))
						.expect("copying file file failed");
				}
			}
		}
	}
	csv_writer.flush().expect("failed to flush write buffer");
}
