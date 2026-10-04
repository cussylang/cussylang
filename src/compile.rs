//! Native compiler driver. Generation and tool execution finish before replacing output.
use crate::{
    ast::{Program, Span},
    checker::Checked,
    diagnostic::{Diagnostic, Result},
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Emit {
    #[default]
    Executable,
    Assembly,
    Object,
    C,
}

impl Emit {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "exe" | "bin" => Some(Self::Executable),
            "asm" => Some(Self::Assembly),
            "obj" => Some(Self::Object),
            "c" => Some(Self::C),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Executable if cfg!(windows) => "exe",
            Self::Executable => "",
            Self::Assembly => "s",
            Self::Object if cfg!(windows) => "obj",
            Self::Object => "o",
            Self::C => "c",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub emit: Emit,
    pub optimization: String,
    pub compiler: OsString,
    pub native_cpu: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            emit: Emit::Executable,
            optimization: "-O3".into(),
            compiler: std::env::var_os("CUSSY_CC").unwrap_or_else(|| "clang".into()),
            native_cpu: false,
        }
    }
}

struct Workspace(PathBuf);

impl Workspace {
    fn create(parent: &Path) -> std::io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for _ in 0..32 {
            let path = parent.join(format!(
                ".cussy-build-{}-{stamp:x}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique compiler work directory",
        ))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn compile(
    program: &Program,
    checked: &Checked,
    input: &Path,
    output: &Path,
    options: &Options,
) -> Result<()> {
    let span = Span {
        file: input.display().to_string(),
        line: 1,
        col: 1,
        len: 1,
    };
    let error = |message: String| Diagnostic::new("NATIVE_BUILD", &span, message);
    if !matches!(
        options.optimization.as_str(),
        "-O0" | "-O1" | "-O2" | "-O3" | "-Os"
    ) {
        return Err(error(
            "optimization must be -O0, -O1, -O2, -O3, or -Os".into(),
        ));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|e| error(format!("output directory: {e}")))?;
    let name = output
        .file_name()
        .ok_or_else(|| error("output needs a file name".into()))?;
    let target = parent.join(name);
    let resolved = target.canonicalize().unwrap_or_else(|_| target.clone());
    let sources = std::iter::once(input.to_path_buf()).chain(
        program
            .sources
            .keys()
            .filter(|s| !s.starts_with("std:"))
            .map(PathBuf::from),
    );
    for source in sources {
        if source.canonicalize().is_ok_and(|path| path == resolved) {
            return Err(error("compile output would overwrite a source file".into()));
        }
    }
    if target.is_dir() {
        return Err(error("compile output is a directory".into()));
    }
    let source = crate::codegen::emit(program, checked)?;
    let workspace = Workspace::create(&parent).map_err(|e| error(e.to_string()))?;
    let c_path = workspace.0.join("program.c");
    fs::write(&c_path, source).map_err(|e| error(e.to_string()))?;
    let artifact = if options.emit == Emit::C {
        c_path.clone()
    } else {
        let artifact = workspace
            .0
            .join(format!("output.{}", options.emit.extension()));
        let mut command = Command::new(&options.compiler);
        command
            .arg("-std=gnu11")
            .arg(&options.optimization)
            .arg("-fno-fast-math")
            .arg("-ffp-contract=off");
        if options.native_cpu {
            if cfg!(target_arch = "aarch64") {
                command.arg("-mcpu=native");
            } else {
                command.arg("-march=native");
            }
        }
        match options.emit {
            Emit::Assembly => {
                command.arg("-S");
            }
            Emit::Object => {
                command.arg("-c");
            }
            _ => {}
        }
        command
            .arg("-x")
            .arg("c")
            .arg(&c_path)
            .arg("-o")
            .arg(&artifact);
        if options.emit == Emit::Executable && !cfg!(windows) {
            command.arg("-lm");
        }
        if options.emit == Emit::Executable && cfg!(windows) {
            command.arg("-lshell32");
        }
        let result = command.output().map_err(|e| {
            error(format!("could not run compiler `{}`: {e}", options.compiler.to_string_lossy()))
                .help("install Clang or GCC, or select its executable with --cc PATH / CUSSY_CC; --emit c needs no C compiler")
        })?;
        if !result.status.success() {
            let details = String::from_utf8_lossy(&result.stderr);
            return Err(error(format!(
                "native compiler failed ({}):\n{details}",
                result.status
            )));
        }
        if !artifact.is_file() {
            return Err(error(
                "native compiler did not produce the requested output".into(),
            ));
        }
        artifact
    };
    // Same-directory staging allows atomic replacement on supported filesystems.
    // Failed compiles leave an existing destination completely untouched.
    fs::rename(artifact, &target).map_err(|e| error(format!("could not publish output: {e}")))?;
    Ok(())
}
