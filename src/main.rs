use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;
use clap::ValueEnum;
pub mod util;
pub mod backup;

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
				backup::iterative_backup(&path_to_world, &path_to_backup_dir, &current_time); //perform iterative backup
			} else {
				//no previous backups or backup mode full specified,
				backup::full_backup(&path_to_world, &path_to_backup_dir, &current_time); //perform a full backup
			}
		}
		Mode::Restore { restore_from } => {
			restore(&path_to_world, &path_to_backup_dir,&restore_from);
		}
	}
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
	let files = util::dir_operation::get_files_recursive(&path_to_backup);
	
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