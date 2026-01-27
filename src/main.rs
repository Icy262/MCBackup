use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use clap::ValueEnum;
pub mod util;
use crate::util::dir_operation::get_files_recursive;

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
	let current_time = util::timestamp::current_time();

	match args.mode {
		Mode::Backup { backup_mode } => {
			//check if the backup is already up to date
			if util::backup::get_most_recent(&path_to_backup_dir).is_some_and(|most_recent_backup| {
				util::get_file_name_as_str(&most_recent_backup) == current_time
			}) {
				//if there is a most recent backup and it is the current time,
				println!("Backup already up to date"); //notify user
				return; //return
			}

			if backup_mode == BackupMode::Iterative && util::backup::prev_exists(&path_to_backup_dir) {
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
	util::backup::init(&path_to_backup_dir, &files.iter().map(|file| util::trim_path(&file, &world_path_cannonicalized)).collect::<Vec<PathBuf>>());

	for file in files { //for each file to backup,
		fs::copy(&file, &path_to_backup_dir.join(util::trim_path(&file, &world_path_cannonicalized))).expect("Should be able to copy file");
	}
}

fn iterative_backup(
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

	let files = get_files_recursive(&path_to_world); //get the paths of every file to backup

	let path_to_world_cannonicalized = path_to_world.canonicalize().expect("Should be able to cannonicalize path to world");

	//create directory to store new backup. MUST go after finding the most recent backup because if not the most recent check will fail
	util::backup::init(&path_to_backup, &files.iter().map(|file| util::trim_path(&file, &path_to_world_cannonicalized)).collect::<Vec<PathBuf>>());

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
		let modified_timestamp = util::timestamp::get_timestamp(&file);

		let trimmed_file_path = util::trim_path(&file, &path_to_world_cannonicalized);

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
								PathBuf::from(util::get_file_name_as_str(&path_to_most_recent_backup))
									.join(&trimmed_file_path)
									.to_str()
									.expect("Should be able to convert path to str")
							)
							.as_bytes(),
						)
						.expect("Should be able to write to manifest");
				} else if let Some(path) =
					//check previous backup manifest for the file
					util::backup::read_manifest(&path_to_most_recent_backup)
					.into_iter()
					.find(|item| {
						util::get_file_name_as_str(item) == util::get_file_name_as_str(&trimmed_file_path) //TODO: Could cause issues if two files have same name. Resolve later
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

	let path_to_backup = util::backup::path_generator(path_to_backup_dir, timestamp);
	let path_to_backup_canonicalized = path_to_backup.canonicalize().expect("Should be able canonicalize the path to backup");
	
	let path_to_backup_dir_canonicalized = path_to_backup_dir.canonicalize().expect("Should be able canonicalize the path to backup");

	//get the paths of every file in the backup
	let files = get_files_recursive(&path_to_backup);
	
	//init the world directory structure
	let files_trimmed = files.clone().iter().map(|file| util::trim_path(file, &path_to_backup_dir_canonicalized).components().skip(1).collect::<PathBuf>()).collect::<Vec<PathBuf>>();
	util::backup::init(&path_to_world, &files_trimmed);

	for file in files { //for each file,
		//copy the file
		fs::copy(&file, path_to_world.join(util::trim_path(&file, &path_to_backup_canonicalized))).expect("Should be able to copy file");
	}

	//resolve the files in the manifest
	let files = util::backup::read_manifest(&path_to_backup);
	
	//init the world directory structure for the manifest files
	let files_trimmed = files.iter().map(|file| file.components().skip(1).collect::<PathBuf>()).collect::<Vec<PathBuf>>();
	util::backup::init(&path_to_world, &files_trimmed);

	for file in files { //for each file,
		//copy the file
		//trim to the start of the backup dir, then remove one more step for timestamp
		fs::copy(&path_to_backup_dir.join(&file), path_to_world.join(&file.components().skip(1).collect::<PathBuf>())).expect("Should be able to copy file");
	}
}