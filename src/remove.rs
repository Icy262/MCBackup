use crate::util::backup;
use std::path::PathBuf;

pub(crate) fn remove(path_to_backup_dir: &PathBuf, timestamp: String) {
	//get the path to the next backup
	let path_to_next_backup = backup::get_next(path_to_backup_dir, &timestamp);

	//copy any files from this backup to the next one
	//update the next backup's manifest to remove any files we copied
}

pub(crate) fn remove_range(timestamp: String) {
	//call remove repeatedly for all timestamps in the range
}