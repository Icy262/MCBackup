use rusqlite::{Connection, params};

use crate::util::{self, backup};
use std::fs;
use std::path::PathBuf;

pub(crate) fn remove(
	backups_path: &PathBuf,
	timestamp: &String,
	database_connection: &Connection,
) {
	//get the timestamp of the next backup
	let next_backup_timestamp =
		backup::get_next(database_connection, &timestamp).expect("Should be a next backup");

	//copy any files from this backup to the next one
	util::dir_operation::copy(
		&backups_path.join(&timestamp),
		&backups_path.join(&next_backup_timestamp),
	);

	//update the next backup's manifest to update the timestamps of moved files
	database_connection
		.execute(
			format!(
				"UPDATE \"{}\"
				SET timestamp = ?1
				WHERE timestamp = ?2
				",
				next_backup_timestamp
			)
			.as_str(),
			params![&next_backup_timestamp, &timestamp],
		)
		.expect("Updating timestamps failed");

	database_connection
		.execute(format!("DROP TABLE \"{}\"", timestamp).as_str(), [])
		.expect("Should be able to drop deleted backup table");

	//remove directory
	fs::remove_dir_all(backups_path.join(timestamp))
		.expect("Should be able to delete old backup");
}
