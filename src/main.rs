use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::path::PathBuf;
use time::OffsetDateTime;
use time::macros::format_description;
use time::{PrimitiveDateTime, UtcOffset};

const FORMAT: &[time::format_description::FormatItem<'static>] =
	format_description!("[year]-[month]-[day]T[hour]-[minute]");

fn main() {
	//temp, will be moved to config later
	let path_to_world = PathBuf::from("testworld");
	let path_to_backup_dir = PathBuf::from("testbackup");
	let dims = vec![
		PathBuf::from("region"),
		PathBuf::from("DIM1/region"),
		PathBuf::from("DIM-1/region"),
	]; //vanilla minecraft uses these three directories. add support for additional directories for modded worlds
	//let backup_frequency; //add flag to force backup

	//set directory timestamp
	let current_time = current_time_as_string();

	//check if previous backup exists
	match prev_backup_exists(&path_to_backup_dir) {
		true => full_backup(&path_to_world, &path_to_backup_dir, &dims, current_time), //if no previous backups, perform full backup
		false => iterative_backup(&path_to_world, &path_to_backup_dir, &dims, current_time), // if there are previous backups, perform iterative backup
	}
}

fn full_backup(path_to_world: &PathBuf, path_to_backup_dir: &PathBuf, dims: &Vec<PathBuf>, current_time: String) -> () {
	//create directory to store new backup
	init_backup_dir(&path_to_backup_dir, &dims, &current_time);

	//for each dimension to backup,
	for dim in dims {
		//generate the path to the backup dir for this dim's regions
		let path_to_dim_backup = path_to_backup_dir.join(&current_time).join(dim);

		//generate the path to this dim's regions
		let path_to_regions = path_to_world.join(dim);

		//get the paths to this dim's regions
		let regions = fs::read_dir(&path_to_regions)
			.expect("world files unreadable")
			.map(|region| region.expect("region file could not be read").path())
			.collect::<Vec<PathBuf>>();

		//for each region,
		for region in regions {
			//copy the region from the world to the backup
			fs::copy(
				&region,
				path_to_dim_backup
					.join(&region.file_name().expect("failed to get the region name")),
			)
			.expect("copying region file failed");
		}
	}
}

fn iterative_backup(path_to_world: &PathBuf, path_to_backup_dir: &PathBuf, dims: &Vec<PathBuf>, current_time: String) -> () {
	//get the paths to the backups in the backup directory
	let mut path_to_backups = fs::read_dir(path_to_backup_dir)
		.expect("backup dir inaccessible")
		.map(|directory| directory.expect("backup inacessible").path())
		.collect::<Vec<PathBuf>>();

	//find most recent backup by sorting, reversing, and getting the first element
	path_to_backups.sort();
	path_to_backups.reverse();
	let path_to_most_recent_backup = path_to_backups.get(0).expect("backup dir empty");

	//get the timestamp of the backup
	let most_recent_backup_timestamp = PrimitiveDateTime::parse(
		path_to_most_recent_backup
			.file_name()
			.expect("&PathBuf to &OsStr conversion failed")
			.to_str()
			.expect("OsStr to Str conversion failed"),
		&FORMAT,
	)
	.expect("could not parse time string")
	.assume_offset(UtcOffset::current_local_offset().expect("could not get current timezone"));

	//create directory to store new backup. MUST go after finding the most recent backup because if not the most recent check will fail
	init_backup_dir(path_to_backup_dir, &dims, &current_time);

	//for each dimension,
	for dim in dims {
		//generate the path to this dim's backup
		let path_to_dim_backup = path_to_backup_dir.join(&current_time).join(dim);

		//get the path to the region files for this dimension
		let region_files = fs::read_dir(path_to_world.join(dim))
			.expect("world files unreadable")
			.map(|region| region.expect("region file could not be read").path())
			.collect::<Vec<PathBuf>>();

		//create writer to manifest csv
		let mut csv_writer = BufWriter::new(
			OpenOptions::new()
				.append(true)
				.open(path_to_dim_backup.join("manifest.csv"))
				.expect("failed to create manifest.csv"),
		);

		//for each region file,
		for region_file in region_files {
			//get the timestamp of the region's last modification
			let modified_timestamp = OffsetDateTime::from(
				fs::metadata(&region_file)
					.expect("failed to read metadata")
					.modified()
					.expect("failed to read timestamp"),
			);

			//compare last modification timestamp to last backup timestamp to determine if a new copy needs to be taken
			match modified_timestamp >= most_recent_backup_timestamp {
				true => {
					//has been modified since last backup, needs to be updated
					fs::copy(
						&region_file,
						&path_to_dim_backup.join(
							&region_file
								.file_name()
								.expect("could not convert file name to os str"),
						),
					)
					.expect("copying region file failed");
				}
				false => {
					//hasn't been modified since last backup, insert path to older backup of the region in csv
					//check previous backup directory for the region
					if fs::read_dir(&path_to_most_recent_backup.join(&dim))
						.expect("could not read most recent backup")
						.any(|dir_entry| {
							dir_entry.expect("backup inacessible").file_name()
								== region_file
									.file_name()
									.expect("file name to os str conversion failed")
						}) {
						//previous backup has the region, so put a reference to it
						csv_writer
							.write_all(
								format!(
									"{},",
									path_to_most_recent_backup
										.join(dim)
										.join(
											region_file
												.file_name()
												.expect("could not get file name")
										)
										.to_str()
										.expect("failed to convert path to str")
								)
								.as_bytes(),
							)
							.expect("failed to write to csv");
					} else if let Some(path) =
						//check previous backup manifest for the region
						fs::read_to_string(
							path_to_most_recent_backup.join(dim).join("manifest.csv"),
						)
						.expect("most recent backup manifest read failed")
						.split(",")
						.find(|item| {
							item.contains(
								region_file
									.file_name()
									.expect("could not get region file name")
									.to_str()
									.expect("region file name to str conversion failed"),
							)
						}) {
						//found the path in the old manifest
						//write the path we found to the new manifest
						csv_writer
							.write_all(format!("{},", path).as_bytes())
							.expect("could not write to manifest");
					} else {
						//something screwy is going on. copy the file and move on
						println!("unexpected error, copied and continued");
						fs::copy(
							&region_file,
							&path_to_dim_backup.join(
								&region_file
									.file_name()
									.expect("could not convert file name to os str"),
							),
						)
						.expect("copying region file failed");
					}
				}
			}
		}
		csv_writer.flush().expect("failed to flush write buffer");
	}
}

fn init_backup_dir(path_to_backup_dir: &PathBuf, dims: &Vec<PathBuf>, current_time: &String) -> () {
	//create directory to store new backup
	let new_backup_dir = path_to_backup_dir.join(&current_time);
	fs::create_dir_all(&new_backup_dir).expect("failed to create backup directory");

	for dim in dims.iter() {
		//create new directory in backup directory to store this dimension
		fs::create_dir_all(new_backup_dir.join(dim))
			.expect("failed to create dimension backup directory");

		//init csv
		fs::File::create(new_backup_dir.join(dim).join("manifest.csv"))
			.expect("failed to create manifest.csv");
	}
}

fn prev_backup_exists(path_to_backup_dir: &PathBuf) -> bool {
	fs::read_dir(path_to_backup_dir)
		.expect("backup dir could not be read")
		.next()
		.is_none()
}

fn current_time_as_string() -> String {
	OffsetDateTime::now_local()
		.expect("could not get local time")
		.format(&FORMAT)
		.expect("could not convert time to String")
}
