use inquire::Select;
use std::path::PathBuf;
pub mod backup;
pub mod remove;
pub mod restore;
pub mod util;
use clap::Parser;
use clap::ValueEnum;
use rusqlite::Connection;
use std::io;
use crate::util::config;

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

	//ensure config exists
	config::init_config_if_not_exists(&database_connection);

	//set current timestamp
	let current_time = util::timestamp::current_time();

	//if running in service mode, just take iterative backup and exit
	if run_mode.run_mode == RunMode::Service {
		let world_path = PathBuf::from(config::get_config(String::from("world_path"), &database_connection).expect("Set the world path in config mode"));
		let backups_path = PathBuf::from(config::get_config(String::from("backups_path"), &database_connection).expect("Set the backup path in config mode"));

		backup::iterative_backup(
			&world_path,
			&backups_path,
			&current_time,
			&database_connection,
		);
		return;
	}

	//not running in service mode, enter interactive mode

	let operation = vec!["Backup", "Restore", "Remove", "Config", "Exit"];

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
				let world_path = PathBuf::from(config::get_config(String::from("world_path"), &database_connection).expect("Set the world path in config mode"));
				let backups_path = PathBuf::from(config::get_config(String::from("backups_path"), &database_connection).expect("Set the backup path in config mode"));

				let backup_type = vec!["Iterative", "Full"];
				match Select::new("Select backup type:", backup_type).prompt() {
					Ok("Iterative") => {
						if util::backup::prev_exists(&backups_path) {
							//if there are previous backups and backup mode iterative specified,
							backup::iterative_backup(
								&world_path,
								&backups_path,
								&current_time,
								&database_connection,
							); //perform iterative backup
						}
					}
					Ok("Full") => {
						//backup mode full specified,
						backup::full_backup(
							&world_path,
							&backups_path,
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
			let world_path = PathBuf::from(config::get_config(String::from("world_path"), &database_connection).expect("Set the world path in config mode"));
			let backups_path = PathBuf::from(config::get_config(String::from("backups_path"), &database_connection).expect("Set the backup path in config mode"));

			if let Some(restore_time) = util::backup::get_all(&database_connection) {
				match Select::new("Select restore timestamp:", restore_time).prompt() {
					Ok(restore_from) => {
						restore::restore(
							&world_path,
							&backups_path,
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
			let backups_path = PathBuf::from(config::get_config(String::from("backups_path"), &database_connection).expect("Set the backup path in config mode"));

			let remove_time =
				util::backup::get_all(&database_connection).expect("Should be backups to remove");
			match Select::new("Select timestamp to remove:", remove_time).prompt() {
				Ok(timestamp_to_remove) => {
					remove::remove(
						&backups_path,
						&timestamp_to_remove,
						&database_connection,
					);
				}
				Err(_) => {
					println!("Not the timestamp of a backup");
				}
			}
		}
		Ok("Config") => {
			let configs = vec!["world_path", "backups_path"];
			match Select::new("Select config to update:", configs).prompt() {
				Ok(key) => {
					let mut value = String::new();
					io::stdin().read_line(&mut value).expect("Should be able to get new config value from user");
					value = String::from(value.trim_end());
					config::set_config(String::from(key), value, &database_connection);
				}
				Err(_) => {
					panic!();
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
