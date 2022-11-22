use anyhow::{Context, Result};
use dyn_clone::DynClone;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, PartialEq)]
pub struct CompileResult {
    pub success: bool,
    pub output: String,
}

pub trait Compiler: DynClone {
    fn compile(&self, script_path: &Path) -> Result<CompileResult>;
}

#[derive(Debug, PartialEq, Clone)]
pub struct CreationKitCompiler {
    compiler_path: PathBuf,
    base_scripts_dir: PathBuf,
    flag_file: String,
}

impl CreationKitCompiler {
    pub fn new(compiler_path: PathBuf, base_scripts_dir: PathBuf, flag_file: String) -> Self {
        CreationKitCompiler {
            compiler_path,
            base_scripts_dir,
            flag_file,
        }
    }
}

impl Compiler for CreationKitCompiler {
    fn compile(&self, script_path: &Path) -> Result<CompileResult> {
        let script_name = script_path
            .file_stem()
            .with_context(|| format!("Unable to get file stem of {}", script_path.display()))?;

        let current_script_dir = script_path.parent().with_context(|| {
            format!(
                "Unable to get parent directory of {}",
                script_path.display()
            )
        })?;

        let process_output = Command::new(&self.compiler_path)
            .stdin(Stdio::null())
            .arg(script_name)
            .arg("-keepasm")
            .arg(format!(
                "-i={};{}",
                &self.base_scripts_dir.display(),
                &current_script_dir.display()
            ))
            .arg(format!("-f={}", &self.flag_file))
            .arg("-o=out")
            .output()
            .with_context(|| format!("Unable to compile file {}", script_path.display()))?;

        let success = process_output.status.success();

        // let stdout = unsafe { String::from_utf8_unchecked(process_output.stdout) };
        let stderr = unsafe { String::from_utf8_unchecked(process_output.stderr) };

        // stderr.push_str(&stdout);

        Ok(CompileResult {
            success,
            output: stderr,
        })
    }
}
