use std::fs;
use std::ffi::OsString;
use time::OffsetDateTime;
use time::macros::format_description;

fn main() {
	const FORMAT: &[time::format_description::FormatItem<'static>] = format_description!("[year]-[month]-[day]T[hour]-[minute]");
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
	//get a list of old backups
	let mut backup_folders = fs::read_dir(backup_dir)
		.expect("backup dir inaccessible")
		.map(|folder| {
			folder
				.expect("backup inacessible")
				.file_name()
		})
		.collect::<Vec<OsString>>();

	//find most recent backup by sorting, reversing, and getting the first element
	backup_folders.sort();
	backup_folders.reverse();
	let most_recent_backup = backup_folders
		.get(0)
		.expect("backup dir empty")
		.clone();
	print!("{:?}", most_recent_backup);

	//get the current save files
	//check timestamps for changes
	//generate manifest
	//copy changed files

}