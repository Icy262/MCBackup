use clap::Parser;
use clap::Subcommand;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use time::OffsetDateTime;
use time::macros::format_description;
use time::{PrimitiveDateTime, UtcOffset};

const FORMAT: &[time::format_description::FormatItem<'static>] =
	format_description!("[year]-[month]-[day]T[hour]-[minute]");

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
		#[arg(default_value = "iterative")]
		backup_mode: String,
	},

	//restore from a backup
	Restore {
		//timestamp to restore from
		#[arg(default_value = "recent")]
		restore_from: String,
	},
}

fn main() {
	let args = Args::parse();

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

	match args.mode {
		Mode::Backup { backup_mode } => {
			if backup_mode.as_str() == "iterative" && prev_backup_exists(&path_to_backup_dir) {
				//if there are previous backups and backup mode iterative specified,
				iterative_backup(&path_to_world, &path_to_backup_dir, &dims, &current_time); //perform iterative backup
			} else {
				//no previous backups or backup mode full specified,
				full_backup(&path_to_world, &path_to_backup_dir, &dims, &current_time); //perform a full backup
			}
		}
		Mode::Restore { restore_from } => {
			restore(&path_to_world, &path_to_backup_dir, &dims, &restore_from);
		}
	}
}

fn full_backup(
	path_to_world: &PathBuf,
	path_to_backup_dir: &PathBuf,
	dims: &Vec<PathBuf>,
	current_time: &String,
) -> () {
	//create directory to store new backup
	init_backup_dir(&path_to_backup_dir, &dims, &current_time);

	//for each dimension to backup,
	for dim in dims {
		//generate the path to the backup dir for this dim's regions
		let path_to_dim_backup = path_to_backup_dir.join(&current_time).join(dim);

		//generate the path to this dim's regions
		let path_to_regions = path_to_world.join(dim);

		copy_entire_dir(&path_to_regions, &path_to_dim_backup);
	}
}

fn iterative_backup(
	path_to_world: &PathBuf,
	path_to_backup_dir: &PathBuf,
	dims: &Vec<PathBuf>,
	current_time: &String,
) -> () {
	let path_to_most_recent_backup = get_most_recent_backup(&path_to_backup_dir);

	//get the timestamp of the backup
	let most_recent_backup_timestamp =
		timestamp_as_str_to_OffsetDateTime(get_file_name_as_str(&path_to_most_recent_backup));

	//create directory to store new backup. MUST go after finding the most recent backup because if not the most recent check will fail
	init_backup_dir(path_to_backup_dir, &dims, &current_time);

	//for each dimension,
	for dim in dims {
		//generate the path to this dim's backup
		let path_to_dim_backup = path_to_backup_dir.join(&current_time).join(dim);

		//get the path to the region files for this dimension
		let region_files = get_files_in_dir(&path_to_world.join(dim));

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
			let modified_timestamp = get_file_timestamp(&region_file);

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
						.find(|item| item.contains(get_file_name_as_str(&region_file)))
					{
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

fn restore(
	path_to_world: &PathBuf,
	path_to_backup_dir: &PathBuf,
	dims: &Vec<PathBuf>,
	timestamp: &String,
) -> () {
}

fn path_to_backup_generator(path_to_backup_dir: &PathBuf, timestamp: &String) -> PathBuf {
	if timestamp == "recent" { //if most recent backup,
		//find most recent
		get_most_recent_backup(path_to_backup_dir)
	} else { //find the backup specified,
		//generate the path
		path_to_backup_dir.join(timestamp)
	}
}

fn get_most_recent_backup(path_to_backup_dir: &PathBuf) -> PathBuf{
	//get the paths to the backups in the backup directory
	let mut path_to_backups = get_files_in_dir(path_to_backup_dir);

	//find most recent backup by sorting, reversing, and getting the first element
	path_to_backups.sort();
	path_to_backups.reverse();
	let path_to_most_recent_backup = path_to_backups.get(0).expect("backup dir empty").to_owned();
	return path_to_most_recent_backup;
}

fn init_backup_dir(path_to_backup_dir: &PathBuf, dims: &Vec<PathBuf>, current_time: &String) -> () {
	//create directory to store new backup
	let new_backup_dir = path_to_backup_dir.join(&current_time);
	fs::create_dir_all(&new_backup_dir).unwrap(); //panic if directory already exists

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
		.is_some()
}

fn current_time_as_string() -> String {
	OffsetDateTime::now_local()
		.expect("could not get local time")
		.format(&FORMAT)
		.expect("could not convert time to String")
}

fn get_file_timestamp(region_file: &PathBuf) -> OffsetDateTime {
	OffsetDateTime::from(
		fs::metadata(&region_file)
			.expect("failed to read metadata")
			.modified()
			.expect("failed to read timestamp"),
	)
}

fn get_files_in_dir(path_to_directory: &PathBuf) -> Vec<PathBuf> {
	//will get the files in the directory
	fs::read_dir(&path_to_directory)
		.expect("Directory must be readable")
		.map(|file| file.expect("File must be readable").path())
		.collect::<Vec<PathBuf>>()
}

fn copy_entire_dir(path_to_src_dir: &PathBuf, path_to_dest_dir: &PathBuf) -> () {
	//get the paths to every file in this dir
	let files = get_files_in_dir(&path_to_src_dir);

	//for each region,
	for file in files {
		//copy the rfileegion from the source dir to the destination dir
		fs::copy(
			&file,
			&path_to_dest_dir.join(file.file_name().expect("File name should be readable")),
		)
		.expect("Copy should be copyable");
	}
}

fn get_file_name_as_str(path_to_file: &PathBuf) -> &str {
	path_to_file
		.file_name()
		.expect("Should be able to get the file name of the file referenced in the path")
		.to_str()
		.expect("Should be able to convert OsString to String")
}

#[allow(non_snake_case)]
fn timestamp_as_str_to_OffsetDateTime(timestamp: &str) -> OffsetDateTime {
	PrimitiveDateTime::parse(timestamp, &FORMAT)
		.expect("Should be able to parse timestamp")
		.assume_offset(UtcOffset::current_local_offset().expect("Should be able to get time zone"))
}
