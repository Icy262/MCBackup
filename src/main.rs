use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use clap::ValueEnum;

use crate::dir_operation::get_files_recursive;

//Iterative backup tool for Minecraft
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
	//backup or restore mode
	#[command(subcommand)]
	mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
	//create a backup
	Backup {
		//full or iterative backup
		#[arg(value_enum, default_value_t = BackupMode::Iterative)]
		backup_mode: BackupMode,
	},

	//restore from a backup
	Restore {
		//timestamp to restore from
		#[arg(default_value = "recent")]
		restore_from: String,
	},
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
enum BackupMode {
	Full,
	Iterative,
}

fn main() {
	let args = Args::parse();

	//temp, will be moved to config later
	let path_to_world = PathBuf::from("testworld");
	let path_to_backup_dir = PathBuf::from("testbackup");

	//set directory timestamp
	let current_time = timestamp::current_time();

	match args.mode {
		Mode::Backup { backup_mode } => {
			//check if the backup is already up to date
			if backup::get_most_recent(&path_to_backup_dir).is_some_and(|most_recent_backup| {
				get_file_name_as_str(&most_recent_backup) == current_time
			}) {
				//if there is a most recent backup and it is the current time,
				println!("Backup already up to date"); //notify user
				return; //return
			}

			if backup_mode == BackupMode::Iterative && backup::prev_exists(&path_to_backup_dir) {
				//if there are previous backups and backup mode iterative specified,
				iterative_backup(&path_to_world, &path_to_backup_dir, &current_time); //perform iterative backup
			} else {
				//no previous backups or backup mode full specified,
				full_backup(&path_to_world, &path_to_backup_dir, &current_time); //perform a full backup
			}
		}
		Mode::Restore { restore_from } => {
			restore(&path_to_world, &path_to_backup_dir,&restore_from);
		}
	}
}

fn full_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
) -> () {
	//get vec of paths to all files to backup
	let files = get_files_recursive(path_to_world);

	let world_path_cannonicalized = path_to_world.canonicalize().expect("Should be able to cannonicalize path to world");

	let path_to_backup_dir = path_to_backups_dir.join(current_time); //path to the directory we are actually backing up to

	//create directory to store new backup
	backup::init(&path_to_backup_dir, &files.iter().map(|file| trim_path(&file, &world_path_cannonicalized)).collect::<Vec<PathBuf>>());

	for file in files { //for each file to backup,
		fs::copy(&file, &path_to_backup_dir.join(trim_path(&file, &world_path_cannonicalized))).expect("Should be able to copy file");
	}
}

fn iterative_backup(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	current_time: &String,
) -> () {
	let path_to_backup = path_to_backups_dir.join(current_time);

	let path_to_most_recent_backup = backup::get_most_recent(&path_to_backups_dir)
		.expect("Should be at least one backup in the backup dir");

	//get the timestamp of the backup
	let most_recent_backup_timestamp =
		timestamp::to_OffsetDateTime(get_file_name_as_str(&path_to_most_recent_backup));

	let files = get_files_recursive(&path_to_world); //get the paths of every file to backup

	let path_to_world_cannonicalized = path_to_world.canonicalize().expect("Should be able to cannonicalize path to world");

	//create directory to store new backup. MUST go after finding the most recent backup because if not the most recent check will fail
	backup::init(&path_to_backup, &files.iter().map(|file| trim_path(&file, &path_to_world_cannonicalized)).collect::<Vec<PathBuf>>());

	//create writer to new manifest csv
	let mut csv_writer = BufWriter::new(
		OpenOptions::new()
			.create(true)
			.write(true)
			.open(path_to_backup.join("manifest.csv"))
			.expect("Should be able to open the manifest"),
	);

	for file in files { //for each file,
		//get the timestamp of the file's last modification
		let modified_timestamp = timestamp::get_timestamp(&file);

		let trimmed_file_path = trim_path(&file, &path_to_world_cannonicalized);

		//compare last modification timestamp to last backup timestamp to determine if a new copy needs to be taken
		match modified_timestamp >= most_recent_backup_timestamp {
			true => {
				//has been modified since last backup, needs to be updated
				fs::copy(
					&file,
					path_to_backup.join(
						trimmed_file_path
					),
				)
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
								PathBuf::from(get_file_name_as_str(&path_to_most_recent_backup))
									.join(&trimmed_file_path)
									.to_str()
									.expect("Should be able to convert path to str")
							)
							.as_bytes(),
						)
						.expect("Should be able to write to manifest");
				} else if let Some(path) =
					//check previous backup manifest for the file
					read_manifest(&path_to_most_recent_backup)
					.into_iter()
					.find(|item| {
						get_file_name_as_str(item) == get_file_name_as_str(&trimmed_file_path) //TODO: Could cause issues if two files have same name. Resolve later
					}) {
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
					fs::copy(
						file,
						path_to_backup.join(&trimmed_file_path)
					)
					.expect("copying file file failed");
				}
			}
		}
	}
	csv_writer.flush().expect("failed to flush write buffer");
}

fn restore(
	path_to_world: &PathBuf,
	path_to_backup_dir: &PathBuf,
	timestamp: &String,
) -> () {
	//remove the world directory and recreate it to remove the contents
	fs::remove_dir_all(path_to_world).expect("Should be able to delete world directory");
	fs::create_dir(path_to_world).expect("Should be able to create world directory");

	let path_to_backup = backup::path_generator(path_to_backup_dir, timestamp);
	let path_to_backup_canonicalized = path_to_backup.canonicalize().expect("Should be able canonicalize the path to backup");
	
	let path_to_backup_dir_canonicalized = path_to_backup_dir.canonicalize().expect("Should be able canonicalize the path to backup");

	//get the paths of every file in the backup
	let files = get_files_recursive(&path_to_backup);
	
	//init the world directory structure
	let files_trimmed = files.clone().iter().map(|file| trim_path(file, &path_to_backup_dir_canonicalized).components().skip(1).collect::<PathBuf>()).collect::<Vec<PathBuf>>();
	backup::init(&path_to_world, &files_trimmed);

	for file in files { //for each file,
		//copy the file
		fs::copy(&file, path_to_world.join(trim_path(&file, &path_to_backup_canonicalized))).expect("Should be able to copy file");
	}

	//resolve the files in the manifest
	let files = read_manifest(&path_to_backup);
	
	//init the world directory structure for the manifest files
	let files_trimmed = files.iter().map(|file| file.components().skip(1).collect::<PathBuf>()).collect::<Vec<PathBuf>>();
	backup::init(&path_to_world, &files_trimmed);

	for file in files { //for each file,
		//copy the file
		//trim to the start of the backup dir, then remove one more step for timestamp
		fs::copy(&path_to_backup_dir.join(&file), path_to_world.join(&file.components().skip(1).collect::<PathBuf>())).expect("Should be able to copy file");
	}
}

fn get_file_name_as_str(path_to_file: &PathBuf) -> &str {
	path_to_file
		.file_name()
		.expect("Should be able to get the file name of the file referenced in the path")
		.to_str()
		.expect("Should be able to convert OsString to String")
}

fn read_manifest(path_to_manifest: &PathBuf) -> Vec<PathBuf> {
	fs::read_to_string(path_to_manifest.join("manifest.csv"))
		.expect("most recent backup manifest read failed")
		.split(",")
		.map(|str| PathBuf::from(str))
		.filter(|item| item != "") //remove empty items
		.collect::<Vec<PathBuf>>()
}

fn trim_path(path: &PathBuf, level: &PathBuf) -> PathBuf { //to be faster on batch operations, level should be cannonicalized before passing. Path should be a child of level.
	path.canonicalize()
		.expect("Should be able to cannonicalize path")
		.strip_prefix(level)
		.expect("Path should be below level")
		.to_path_buf()
}

mod timestamp {
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

mod dir_operation {
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

mod backup {
	use std::path::PathBuf;
	use std::fs::{self, create_dir_all};
	use crate::dir_operation;

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
}
