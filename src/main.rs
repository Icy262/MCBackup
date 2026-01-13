use std::collections::btree_set::Difference;
use std::ffi::OsString;
use std::fs::{self, Metadata};
use time::OffsetDateTime;
use time::macros::format_description;

const FORMAT: &[time::format_description::FormatItem<'static>] =
	format_description!("[year]-[month]-[day]T[hour]-[minute]");

fn main() {
	//temp, will be moved to config later
	let world_path = "testworld";
	let backup_dir = "testbackup";
	let dims = vec!["region", "DIM1/region", "DIM-1/region"]; //vanilla minecraft uses these three directories. add support for additional directories for modded worlds
	//let backup_frequency; //add flag to force backup

	//check if previous backup exists
	match fs::read_dir(backup_dir)
		.expect("backup dir could not be read")
		.next()
		.is_none()
	{
		true => full_backup(world_path, backup_dir, dims), //if no previous backups, perform full backup
		false => iterative_backup(world_path, backup_dir, dims), // if there are previous backups, perform iterative backup
	}
}

fn full_backup(world_path: &str, backup_dir: &str, dims: Vec<&str>) -> () {}

fn iterative_backup(world_path: &str, backup_dir: &str, dims: Vec<&str>) -> () {
	//get a list of old backups
	let mut backups = fs::read_dir(backup_dir)
		.expect("backup dir inaccessible")
		.map(|folder| folder.expect("backup inacessible").file_name())
		.collect::<Vec<OsString>>();

	//find most recent backup by sorting, reversing, and getting the first element
	backups.sort();
	backups.reverse();
	let most_recent_backup = backups.get(0).expect("backup dir empty").clone();

	//create directory to store new backup
	let new_backup = format!(
		"{}/{}",
		backups,
		OffsetDateTime::now_utc()
			.format(FORMAT)
			.expect("current time to timestamp conversion failed")
	);
	fs::create_dir_all(new_backup);

	//for each dimension,
	for dim in dims {
		let this_dim_backup = format!("{}/{}", new_backup, dim);

		//create new directory in backup folder to store this dimension
		fs::create_dir(this_dim_backup);

		//get the names for the region files for this dimension
		let mut region_files = fs::read_dir(format!("{}/{}", world_path, this_dim_backup))
			.expect("world files unreadable");

		//collect read dir into Vec of file names
		let mut region_files = region_files
			.map(|region| region.expect("region file could not be read").file_name())
			.collect::<Vec<OsString>>();

		//sort files by name
		region_files.sort();

		//check timestamps for changes

		//generate manifest
		//copy changed files
	}
}
