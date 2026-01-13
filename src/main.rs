use std::fs;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::fmt::Write;
use std::path::PathBuf;
use std::ffi::OsString;

fn main() {
	//temp, will be moved to config later
	let world_path = "testworld";
	let backup_dir = "testbackup";
	//let backup_frequency; //add flag to force backup

	//check if previous backup exists
	match fs::read_dir(backup_dir)
		.expect("backup dir could not be read")
		.next()
		.is_none() {
		true => full_backup(world_path, backup_dir), //if no previous backups, perform full backup
		false => iterative_backup(world_path, backup_dir) // if there are previous backups, perform iterative backup
	}
}

fn full_backup(world_path: &str, backup_dir: &str) -> () {
}

fn iterative_backup(world_path: &str, backup_dir: &str) -> () {
}