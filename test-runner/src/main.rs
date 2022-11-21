use std::fs;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use compiler::CreationKitCompiler;

use crate::args::Args;

mod args;
mod compiler;
mod json;
mod runner;

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => println!("{:#?}", err),
    };
}

fn run() -> Result<()> {
    let args = Args::parse();

    if !args.definition.exists() {
        return Err(anyhow!(
            "File {} does not exist!",
            args.definition.display()
        ));
    }

    if !args.scripts_dir.exists() {
        return Err(anyhow!(
            "Directory {} does not exist!",
            args.scripts_dir.exists()
        ));
    }

    if !args.compiler_path.exists() {
        return Err(anyhow!(
            "File {} does not exist!",
            args.compiler_path.display()
        ));
    }

    if !args.import.exists() {
        return Err(anyhow!(
            "Directory {} does not exist!",
            args.import.display()
        ));
    }

    let file_contents = fs::read_to_string(&args.definition)
        .with_context(|| format!("Unable to read file {}", args.definition.display()))?;

    let test_definition: json::TestDefinition = serde_json::from_str(&file_contents)
        .with_context(|| format!("Invalid JSON in {}", args.definition.display()))?;

    let compiler = CreationKitCompiler::new(args.compiler_path, args.import, args.flag);

    runner::run_test_definition(test_definition, args.scripts_dir, Box::new(compiler))
}
