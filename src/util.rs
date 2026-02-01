use std::path::PathBuf;

pub(crate) fn trim_path(path: &PathBuf, level: &PathBuf) -> PathBuf {
	//to be faster on batch operations, level should be cannonicalized before passing. Path should be a child of level.
	path.canonicalize()
		.expect("Should be able to cannonicalize path")
		.strip_prefix(level)
		.expect("Path should be below level")
		.to_path_buf()
}

pub(crate) mod timestamp {
	use std::fs;
	use std::path::PathBuf;
	use time::OffsetDateTime;
	use time::PrimitiveDateTime;
	use time::UtcOffset;
	use time::macros::format_description;

	const FORMAT: &[time::format_description::FormatItem<'static>] =
		format_description!("[year]-[month]-[day]T[hour]-[minute]");

	#[allow(non_snake_case)]
	pub(crate) fn to_OffsetDateTime(timestamp: &str) -> OffsetDateTime {
		PrimitiveDateTime::parse(timestamp, &FORMAT)
			.expect("Should be able to parse timestamp")
			.assume_offset(
				UtcOffset::current_local_offset().expect("Should be able to get time zone"),
			)
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
	use std::fs;
	use std::path::PathBuf;

use crate::util::trim_path;

	#[allow(dead_code)]
	pub(crate) fn copy(path_to_src_dir: &PathBuf, path_to_dest_dir: &PathBuf) -> () {
		let path_to_src_dir_cannonicalized = path_to_src_dir.canonicalize().expect("Should be able to cannonicalize path to src dir");

		//get the paths to every file in this dir
		let files = get_files_recursive(&path_to_src_dir).iter().filter_map(|path| {
			if path.is_file() {
				Some(trim_path(path, &path_to_src_dir_cannonicalized))
			} else {
				None
			}
		}).collect::<Vec<PathBuf>>();

		//for each file,
		for file in files {
			//copy the file from the source dir to the destination dir
			fs::copy(
				&path_to_src_dir.join(&file),
				&path_to_dest_dir.join(&file),
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
		if path_to_directory.is_dir() {
			//if path is to dir,
			let mut files = vec![];

			for file in get_files(&path_to_directory) {
				//for each file,
				files.append(&mut get_files_recursive(&file)); //get the files it contains and append
			}

			return files;
		} else {
			//path is to file,
			return vec![path_to_directory.to_path_buf()];
		}
	}
}

pub(crate) mod backup {
	use rusqlite::{Connection, OptionalExtension};
	use std::fs::{self, create_dir_all};
	use std::path::PathBuf;

	pub(crate) fn path_generator(path_to_backup_dir: &PathBuf, timestamp: &String, database_connection: &Connection) -> String {
		if timestamp == "recent" {
			//if most recent backup,
			//find most recent
			get_most_recent(database_connection).expect("Should be a previous backup")
		} else {
			//find the backup specified,
			//generate the path
			path_to_backup_dir.join(timestamp).to_string_lossy().to_string()
		}
	}

	pub(crate) fn get_most_recent(database_connection: &Connection) -> Option<String> {
		database_connection.query_row(
			"SELECT name
			FROM sqlite_schema
			WHERE type = 'table'
			ORDER BY name DESC
			LIMIT 1;",
			[],
			|row| row.get::<_, String>("name")
		)
		.optional()
		.expect("Should be able to get table with most recent timestamp")
	}

	pub(crate) fn get_next(database_connection: &Connection, timestamp: &String) -> Option<String> {
		database_connection.query_row(
			"SELECT name
			FROM sqlite_schema
			WHERE type = 'table' AND name > ?1
			ORDER BY name ASC
			LIMIT 1;",
			[timestamp],
			|row| row.get::<_, String>("name")
		)
		.optional()
		.expect("Should be able to get table with most recent timestamp")
	}

	pub(crate) fn init(path_to_backup: &PathBuf, files: &Vec<PathBuf>, current_time: &String, database_connection: &Connection) -> () {
		//file paths should be trimmed to world directory level
		for file in files {
			//create a directory for the file
			create_dir_all(
				path_to_backup.join(file.parent().expect("File position should have a parent")),
			)
			.expect("Should be able to create dir");
		}

		//create a new table in the manifest
		let create_table = format!(
			"CREATE TABLE IF NOT EXISTS \"{}\" (
				timestamp DATE,
				path TEXT
			);",
			current_time
			);
		database_connection.execute(&create_table, ()).expect("Should be able to create new table");
	}

	pub(crate) fn prev_exists(path_to_backup_dir: &PathBuf) -> bool {
		fs::read_dir(path_to_backup_dir)
			.expect("backup dir could not be read")
			.next()
			.is_some()
	}
}
