use crate::util;
use std::fs;
use std::path::PathBuf;
use rusqlite::Connection;

pub(crate) fn restore(
	path_to_world: &PathBuf,
	path_to_backups_dir: &PathBuf,
	timestamp: &String,
	database_connection: &Connection,
) -> () {
	//remove the world directory and recreate it to remove the contents
	fs::remove_dir_all(path_to_world).expect("Should be able to delete world directory");
	fs::create_dir(path_to_world).expect("Should be able to create world directory");

	let resolved_timestamp = util::backup::path_generator(path_to_backups_dir, timestamp, database_connection);

	//get the paths of every file in the backup
	let get_all_files  = format!("SELECT timestamp, path FROM \"{}\"", resolved_timestamp);
	dbg!(&get_all_files);
	let (timestamps, paths_trimmed) = database_connection
		.prepare(&get_all_files)
		.expect("Should be able to prepare SQL query")
		.query_map([], |item| {
				let timestamp: String = item.get("timestamp")?;
				let path: PathBuf = PathBuf::from(item.get::<_, String>("path")?);
				Ok((timestamp, path))
			})
		.expect("Query should succeed")
		.collect::<Result<(Vec<String>, Vec<PathBuf>), _>>()
		.expect("Should be able to collect rows");

	//init the world directory structure
	util::backup::init(&path_to_world, &paths_trimmed, &resolved_timestamp, database_connection);

	for (index, path) in paths_trimmed.iter().enumerate() {
		//for each file,
		//copy the file
		fs::copy(
			path_to_backups_dir.join(timestamps.get(index).expect("Should be able to get timestamp of file")).join(path),
			path_to_world.join(path),
		)
		.expect("Should be able to copy file");
	}
}
