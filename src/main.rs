use std::fs::{self, DirEntry, Metadata};
use std::io::BufWriter;
use time::PrimitiveDateTime;
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
		.map(|directory| directory.expect("backup inacessible").file_name().into_string().expect("could not convert os string to String"))
		.collect::<Vec<String>>();

	//find most recent backup by sorting, reversing, and getting the first element
	backups.sort();
	backups.reverse();
	let most_recent_backup = backups.get(0)
			.expect("backup dir empty");

	//get the timestamp of the backup
	let most_recent_backup_timestamp = PrimitiveDateTime::parse(most_recent_backup.as_str(), &FORMAT)
		.expect("failed to parse directory to timestamp");

	//create directory to store new backup
	let new_backup = format!(
		"{}/{}",
		backup_dir,
		OffsetDateTime::now_local()
			.expect("could not get local time")
			.format(&FORMAT)
			.expect("could not convert time to String")
	);
	fs::create_dir_all(&new_backup).expect("failed to create backup directory");

	//for each dimension,
	for dim in dims {
		let this_dim_backup = format!("{}/{}", new_backup, dim);

		//create new directory in backup directory to store this dimension
		fs::create_dir(&this_dim_backup).expect("failed to create dimension backup directory");

		//get the names of the region files for this dimension
		let mut region_files = fs::read_dir(format!("{}/{}", world_path, dim))
			.expect("world files unreadable")
			.map(|region| region.expect("region file could not be read").file_name().into_string().expect("could not convert os string to String"))
			.collect::<Vec<String>>();

		//sort files by name
		region_files.sort();

		//csv to store paths to old region copies
		let output_csv = fs::File::create(format!("{}/{}.csv", this_dim_backup, "manifest"))
			.expect("failed to create manifest.csv");
		let mut csv_writer = BufWriter::new(output_csv);

		//generate csv containing the paths of any regions that have not changed so that they can be retrieved from previous backups and copy the regions that have changed
		for region_file in region_files {
			//check modified timestamp for changes
			let modified_timestamp = OffsetDateTime::from(
				fs::metadata(format!("{}/{}/{}", world_path, dim, region_file))
					.expect("failed to read metadata")
					.modified()
					.expect("failed to read timestamp")
			);

			//compare time of modification to time of last backup
			// match modified_timestamp >= most_recent_backup_timestamp {
			// 	true => {
			// 		fs::copy(
			// 			&region_file,
			// 			format!(
			// 				"{}/{}",
			// 				this_dim_backup,
			// 				&region_file
			// 			),
			// 		)
			// 		.expect("copying region file failed");
			// 	} //has been modified since last backup, needs to be updated
			// 	false => { //hasn't been modified since last backup, insert path to older backup of the region
			// 		//check previous backup directory for the region
			// 		if fs::read_dir(format!("{}/{}", most_recent_backup, dim)).expect("could not read most recent backup").map(|directory| directory.expect("backup inacessible").file_name().into_string().expect("could not convert os string to String")).find(|region_name| *region_name == region_file).is_some() {

			// 		}

			// 		//check previous backup manifest for the region
			// 	}
			// }
		}
	}
}
