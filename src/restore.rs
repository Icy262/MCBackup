use crate::util;
use std::fs;
use std::path::PathBuf;

pub(crate) fn restore(
	path_to_world: &PathBuf,
	path_to_backup_dir: &PathBuf,
	timestamp: &String,
) -> () {
	//remove the world directory and recreate it to remove the contents
	fs::remove_dir_all(path_to_world).expect("Should be able to delete world directory");
	fs::create_dir(path_to_world).expect("Should be able to create world directory");

	let path_to_backup = util::backup::path_generator(path_to_backup_dir, timestamp);
	let path_to_backup_canonicalized = path_to_backup
		.canonicalize()
		.expect("Should be able canonicalize the path to backup");

	let path_to_backup_dir_canonicalized = path_to_backup_dir
		.canonicalize()
		.expect("Should be able canonicalize the path to backup");

	//get the paths of every file in the backup
	let files = util::dir_operation::get_files_recursive(&path_to_backup);

	//init the world directory structure
	let files_trimmed = files
		.clone()
		.iter()
		.map(|file| {
			util::trim_path(file, &path_to_backup_dir_canonicalized)
				.components()
				.skip(1)
				.collect::<PathBuf>()
		})
		.collect::<Vec<PathBuf>>();
	util::backup::init(&path_to_world, &files_trimmed);

	for file in files {
		//for each file,
		//copy the file
		fs::copy(
			&file,
			path_to_world.join(util::trim_path(&file, &path_to_backup_canonicalized)),
		)
		.expect("Should be able to copy file");
	}

	//resolve the files in the manifest
	let files = util::backup::read_manifest(&path_to_backup);

	//init the world directory structure for the manifest files
	let files_trimmed = files
		.iter()
		.map(|file| file.components().skip(1).collect::<PathBuf>())
		.collect::<Vec<PathBuf>>();
	util::backup::init(&path_to_world, &files_trimmed);

	for file in files {
		//for each file,
		//copy the file
		//trim to the start of the backup dir, then remove one more step for timestamp
		fs::copy(
			&path_to_backup_dir.join(&file),
			path_to_world.join(&file.components().skip(1).collect::<PathBuf>()),
		)
		.expect("Should be able to copy file");
	}
}
