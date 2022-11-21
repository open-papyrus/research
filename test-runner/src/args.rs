use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub definition: PathBuf,

    #[arg(long)]
    pub scripts_dir: PathBuf,

    #[arg(long)]
    pub compiler_path: PathBuf,

    #[arg(long)]
    pub import: PathBuf,

    #[arg(long)]
    pub flag: String,
}
