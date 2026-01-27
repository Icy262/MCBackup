use std::path::PathBuf;

pub(crate) fn get_file_name_as_str(path_to_file: &PathBuf) -> &str {
	path_to_file
		.file_name()
		.expect("Should be able to get the file name of the file referenced in the path")
		.to_str()
		.expect("Should be able to convert OsString to String")
}

pub(crate) fn trim_path(path: &PathBuf, level: &PathBuf) -> PathBuf { //to be faster on batch operations, level should be cannonicalized before passing. Path should be a child of level.
	path.canonicalize()
		.expect("Should be able to cannonicalize path")
		.strip_prefix(level)
		.expect("Path should be below level")
		.to_path_buf()
}

pub(crate) mod timestamp {
	use time::OffsetDateTime;
	use time::PrimitiveDateTime;
	use time::UtcOffset;
	use time::macros::format_description;
	use std::path::PathBuf;
	use std::fs;

	const FORMAT: &[time::format_description::FormatItem<'static>] =
	format_description!("[year]-[month]-[day]T[hour]-[minute]");

	#[allow(non_snake_case)]
	pub(crate) fn to_OffsetDateTime(timestamp: &str) -> OffsetDateTime {
		PrimitiveDateTime::parse(timestamp, &FORMAT)
			.expect("Should be able to parse timestamp")
			.assume_offset(UtcOffset::current_local_offset().expect("Should be able to get time zone"))
	}

	pub(crate) fn get_timestamp(file: &PathBuf) -> OffsetDateTime {
		OffsetDateTime::from(
			fs::metadata(&file)
				.expect("failed to read metadata")
				.modified()
				.expect("failed to read timestamp"),
		)
	}

	pub(crate) fn current_time() -> String {
		OffsetDateTime::now_local()
			.expect("could not get local time")
			.format(&FORMAT)
			.expect("could not convert time to String")
	}
}

pub(crate) mod dir_operation {
	use std::path::PathBuf;
	use std::fs;

	pub(crate) fn copy(path_to_src_dir: &PathBuf, path_to_dest_dir: &PathBuf) -> () {
		//get the paths to every file in this dir
		let files = get_files(&path_to_src_dir);

		//for each file,
		for file in files {
			//copy the file from the source dir to the destination dir
			fs::copy(
				&file,
				&path_to_dest_dir.join(file.file_name().expect("File name should be readable")),
			)
			.expect("Copy should be copyable");
		}
	}

	pub(crate) fn get_files(path_to_directory: &PathBuf) -> Vec<PathBuf> {
		//will get the files in the directory
		fs::read_dir(&path_to_directory)
			.expect("Directory must be readable")
			.map(|file| file.expect("File must be readable").path())
			.collect::<Vec<PathBuf>>()
	}

	pub(crate) fn get_files_recursive(path_to_directory: &PathBuf) -> Vec<PathBuf> {
		if path_to_directory.is_dir() { //if path is to dir,
			let mut files = vec![];

			for file in get_files(&path_to_directory) { //for each file,
				files.append(&mut get_files_recursive(&file)); //get the files it contains and append
			}

			return files;
		} else { //path is to file,
			return vec![path_to_directory.to_path_buf()];
		}
	}
}

pub(crate) mod backup {
	use std::path::PathBuf;
	use std::fs::{self, create_dir_all};
	use crate::util::dir_operation;

	pub(crate) fn path_generator(path_to_backup_dir: &PathBuf, timestamp: &String) -> PathBuf {
		if timestamp == "recent" {
			//if most recent backup,
			//find most recent
			get_most_recent(path_to_backup_dir)
				.expect("Should be at least one backup in backup directory to call this function")
		} else {
			//find the backup specified,
			//generate the path
			path_to_backup_dir.join(timestamp)
		}
	}

	pub(crate) fn get_most_recent(path_to_backup_dir: &PathBuf) -> Option<PathBuf> {
		//get the paths to the backups in the backup directory
		let mut path_to_backups = dir_operation::get_files(path_to_backup_dir);

		//find most recent backup by sorting, reversing, and getting the first element
		path_to_backups.sort();
		path_to_backups.reverse();

		//Return a path to the most recent backup exists, or none
		if path_to_backups.len() != 0 {
			//if there are backups in the backup dir,
			return Some(path_to_backups[0].to_owned());
		} else {
			//no backups,
			return None;
		}
	}

	pub(crate) fn init(path_to_backup: &PathBuf, files: &Vec<PathBuf>) -> () { //file paths should be trimmed to world directory level
		for file in files { //create a directory for the file
			create_dir_all(path_to_backup.join(file.parent().expect("File position should have a parent"))).expect("Should be able to create dir");
		}

		//create a manifest
		fs::File::create(&path_to_backup.join("manifest.csv")).expect("Should be able to create the manifest");
	}

	pub(crate) fn prev_exists(path_to_backup_dir: &PathBuf) -> bool {
		fs::read_dir(path_to_backup_dir)
			.expect("backup dir could not be read")
			.next()
			.is_some()
	}

	pub(crate) fn read_manifest(path_to_manifest: &PathBuf) -> Vec<PathBuf> {
		fs::read_to_string(path_to_manifest.join("manifest.csv"))
			.expect("most recent backup manifest read failed")
			.split(",")
			.map(|str| PathBuf::from(str))
			.filter(|item| item != "") //remove empty items
			.collect::<Vec<PathBuf>>()
	}
}