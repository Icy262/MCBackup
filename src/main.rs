use std::ffi::OsString;
use std::fmt::format;
use std::fs::{self, Metadata};
use std::io::BufWriter;
use std::io::Write;
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
	let most_recent_backup = OffsetDateTime::parse(
		backups
			.get(0)
			.expect("backup dir empty")
			.clone()
			.to_str()
			.expect("most recent backup path conversion to string failed"),
		&FORMAT,
	)
	.expect("failed to parse directory to timestamp");

	//create directory to store new backup
	let new_backup = format!(
		"{}/{}",
		backup_dir,
		OffsetDateTime::now_utc()
			.format(FORMAT)
			.expect("current time to timestamp conversion failed")
	);
	fs::create_dir_all(&new_backup).expect("failed to create backup folder");

	//for each dimension,
	for dim in dims {
		let this_dim_backup = format!("{}/{}", new_backup, dim);

		//create new directory in backup folder to store this dimension
		fs::create_dir(&this_dim_backup).expect("failed to create dimension backup folder");

		//get the names of the region files for this dimension
		let mut region_files = fs::read_dir(format!("{}/{}", world_path, dim))
			.expect("world files unreadable")
			.map(|region| region.expect("region file could not be read").file_name())
			.collect::<Vec<OsString>>();

		//sort files by name
		region_files.sort();

		//csv to store paths to old region copies
		let output_csv = fs::File::create(format!("{}/{}.csv", this_dim_backup, "manifest"))
			.expect("failed to create manifest.csv");
		let mut csv_writer = BufWriter::new(output_csv);

		//generate csv containing the paths of any regions that have not changed so that they can be retrieved from previous backups and copy and regions that have changed
		for region_file in region_files {
			//check modified timestamp for changes
			let modified_timestamp = OffsetDateTime::from(
				fs::metadata(&region_file)
					.expect("failed to read metadata")
					.modified()
					.expect("failed to read timestamp"),
			);

			//compare time of modification to time of last backup
			match modified_timestamp >= most_recent_backup {
				true => {
					fs::copy(
						&region_file,
						format!(
							"{}/{}",
							this_dim_backup,
							&region_file
								.to_str()
								.expect("failed to convert os path to &str")
						),
					)
					.expect("copying region file failed");
				} //has been modified since last backup, needs to be updated
				false => { //hasn't been modified since last backup, insert path to older backup of the region
					//check previous backup directory for the region
					//check previous backup manifest for the region
				}
			}
		}
	}
}
