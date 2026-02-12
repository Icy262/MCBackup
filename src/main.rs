use inquire::Select;
use std::path::PathBuf;
pub mod backup;
pub mod remove;
pub mod restore;
pub mod util;
use crate::backup::iterative_backup;
use clap::Parser;
use clap::ValueEnum;
use rusqlite::Connection;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
	/// Run in interactive or service mode
	#[arg(value_enum, default_value_t = RunMode::Interactive)]
	run_mode: RunMode,
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
enum RunMode {
	Service,
	Interactive,
}

//Iterative backup tool for Minecraft
fn main() {
	//parse the service or interactive mode command
	let run_mode = Args::parse();

	//create DB connection
	let database_connection =
		Connection::open("manifest.db").expect("Should be able to load or create sql database");

	//temp, will be moved to config later
	let path_to_world = PathBuf::from("testworld");
	let path_to_backups_dir = PathBuf::from("testbackup");

	//set directory timestamp
	let current_time = util::timestamp::current_time();

	//if running in service mode, just take iterative backup and exit
	if run_mode.run_mode == RunMode::Service {
		iterative_backup(
			&path_to_world,
			&path_to_backups_dir,
			&current_time,
			&database_connection,
		);
		return;
	}

	//not running in service mode, enter interactive mode

	let operation = vec!["Backup", "Restore", "Remove", "Exit"];

	match Select::new("Select mode:", operation).prompt() {
		Ok("Backup") => {
			//check if the backup is already up to date
			//TODO: update to use database
			if util::backup::get_most_recent(&database_connection)
				.is_some_and(|most_recent_backup| most_recent_backup == current_time)
			{
				//if there is a most recent backup and it is the current time,
				println!("Backup already up to date"); //notify user
			} else {
				let backup_type = vec!["Iterative", "Full"];
				match Select::new("Select backup type:", backup_type).prompt() {
					Ok("Iterative") => {
						if util::backup::prev_exists(&path_to_backups_dir) {
							//if there are previous backups and backup mode iterative specified,
							backup::iterative_backup(
								&path_to_world,
								&path_to_backups_dir,
								&current_time,
								&database_connection,
							); //perform iterative backup
						}
					}
					Ok("Full") => {
						//backup mode full specified,
						backup::full_backup(
							&path_to_world,
							&path_to_backups_dir,
							&current_time,
							&database_connection,
						); //perform a full backup
					}
					Ok(_) => {
						panic!();
					}
					Err(_) => {
						println!("Invalid backup type specified");
					}
				}
			}
		}
		Ok("Restore") => {
			if let Some(restore_time) = util::backup::get_all(&database_connection) {
				match Select::new("Select restore timestamp:", restore_time).prompt() {
					Ok(restore_from) => {
						restore::restore(
							&path_to_world,
							&path_to_backups_dir,
							&restore_from,
							&database_connection,
						);
					}
					Err(_) => {
						println!("Not the timestamp of a backup");
					}
				}
			} else {
				println!("No backups to restore from");
			}
		}
		Ok("Remove") => {
			let remove_time =
				util::backup::get_all(&database_connection).expect("Should be backups to remove");
			match Select::new("Select timestamp to remove:", remove_time).prompt() {
				Ok(timestamp_to_remove) => {
					remove::remove(
						&path_to_backups_dir,
						&timestamp_to_remove,
						&database_connection,
					);
				}
				Err(_) => {
					println!("Not the timestamp of a backup");
				}
			}
		}
		Ok("Exit") => {
			return;
		}
		Ok(_) => {
			panic!();
		}
		Err(_) => {
			println!("Invalid mode specified");
		}
	}
}
