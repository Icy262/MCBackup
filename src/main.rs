use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use std::path::PathBuf;
pub mod backup;
pub mod remove;
pub mod restore;
pub mod util;
use rusqlite::Connection;

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

	//remove a backup
	Remove {
		#[arg()]
		timestamp_to_remove: String,
	},
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
enum BackupMode {
	Full,
	Iterative,
}

fn main() {
	//create DB connection
	let database_connection =
		Connection::open("manifest.db").expect("Should be able to load or create sql database");

	let args = Args::parse();

	//temp, will be moved to config later
	let path_to_world = PathBuf::from("testworld");
	let path_to_backups_dir = PathBuf::from("testbackup");

	//set directory timestamp
	let current_time = util::timestamp::current_time();

	match args.mode {
		Mode::Backup { backup_mode } => {
			//check if the backup is already up to date
			if util::backup::get_most_recent(&database_connection)
				.is_some_and(|most_recent_backup| most_recent_backup == current_time)
			{
				//if there is a most recent backup and it is the current time,
				println!("Backup already up to date"); //notify user
				return; //return
			}

			if backup_mode == BackupMode::Iterative
				&& util::backup::prev_exists(&path_to_backups_dir)
			{
				//if there are previous backups and backup mode iterative specified,
				backup::iterative_backup(
					&path_to_world,
					&path_to_backups_dir,
					&current_time,
					&database_connection,
				); //perform iterative backup
			} else {
				//no previous backups or backup mode full specified,
				backup::full_backup(
					&path_to_world,
					&path_to_backups_dir,
					&current_time,
					&database_connection,
				); //perform a full backup
			}
		}
		Mode::Restore { restore_from } => {
			restore::restore(
				&path_to_world,
				&path_to_backups_dir,
				&restore_from,
				&database_connection,
			);
		}
		Mode::Remove {
			timestamp_to_remove,
		} => {
			remove::remove(
				&path_to_backups_dir,
				&timestamp_to_remove,
				&database_connection,
			);
		}
	}
}
