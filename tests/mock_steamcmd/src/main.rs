use anyhow::Result;
use std::{env, fs, io::Write, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Get the log file path from the environment variable
    let log_file_path_str = env::var("MOCK_STEAMCMD_LOG_FILE")
        .map_err(|_| anyhow::anyhow!("MOCK_STEAMCMD_LOG_FILE environment variable not set"))?;
    let log_file_path = PathBuf::from(log_file_path_str);

    // The first argument is the path to the executable itself, so skip it.
    // The rest of the arguments are the ones passed to "steamcmd".
    let steamcmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    let mut file = fs::File::create(&log_file_path)?;
    writeln!(file, "{}", steamcmd_args.join(" "))?;

    Ok(())
}
