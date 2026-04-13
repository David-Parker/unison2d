use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    /// When true, `SystemInvoker` inherits the parent process's stdout/stderr so
    /// the child's output streams live to the terminal. `Output::stdout` and
    /// `Output::stderr` are then empty. Use this for long-running build tools
    /// (trunk, xcodebuild, gradle, tstl) where real-time progress matters.
    pub streaming: bool,
}

impl Invocation {
    pub fn new(program: impl Into<String>, cwd: impl AsRef<Path>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.as_ref().to_path_buf(),
            env: Vec::new(),
            streaming: false,
        }
    }
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }
    pub fn args<I, S>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for a in iter {
            self.args.push(a.into());
        }
        self
    }
    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.env.push((k.into(), v.into()));
        self
    }
    /// Inherit the parent process's stdout/stderr instead of capturing them.
    /// Output fields will be empty after the call — rely on terminal output
    /// for diagnostics. Use for long-running build tools.
    pub fn streaming(mut self) -> Self {
        self.streaming = true;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// A background process handle returned by [`Invoker::spawn`]. Dropping the
/// handle kills the child — callers that want the process to keep running
/// must hold onto the handle for the lifetime of the parent process.
pub trait SpawnedProcess: Send {
    fn kill(&mut self) -> Result<()>;
}

pub trait Invoker: Send + Sync {
    fn run(&self, inv: &Invocation) -> Result<Output>;
    /// Launch the command in the background and return a handle to it. The
    /// `streaming` flag on `Invocation` still controls whether stdout/stderr
    /// inherit the parent's (useful for `tstl --watch`) or are discarded.
    fn spawn(&self, inv: &Invocation) -> Result<Box<dyn SpawnedProcess>>;
}

/// `SpawnedProcess` wrapping a real OS child process. Kills the child on drop.
pub struct SystemChild {
    child: Option<Child>,
}

impl SpawnedProcess for SystemChild {
    fn kill(&mut self) -> Result<()> {
        if let Some(mut c) = self.child.take() {
            // Ignore "already exited" errors.
            let _ = c.kill();
            let _ = c.wait();
        }
        Ok(())
    }
}

impl Drop for SystemChild {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

pub struct SystemInvoker;

impl Invoker for SystemInvoker {
    fn run(&self, inv: &Invocation) -> Result<Output> {
        let mut cmd = Command::new(&inv.program);
        cmd.args(&inv.args).current_dir(&inv.cwd);
        for (k, v) in &inv.env {
            cmd.env(k, v);
        }
        if inv.streaming {
            // Inherit the parent's stdout/stderr so the child streams live to the
            // terminal. Good for trunk/xcodebuild/gradle/tstl where progress matters.
            let status = cmd.status()
                .with_context(|| format!("running {} {:?}", inv.program, inv.args))?;
            Ok(Output {
                status: status.code().unwrap_or(-1),
                stdout: String::new(),
                stderr: String::new(),
            })
        } else {
            let out = cmd.output()
                .with_context(|| format!("running {} {:?}", inv.program, inv.args))?;
            Ok(Output {
                status: out.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
    }

    fn spawn(&self, inv: &Invocation) -> Result<Box<dyn SpawnedProcess>> {
        let mut cmd = Command::new(&inv.program);
        cmd.args(&inv.args).current_dir(&inv.cwd);
        for (k, v) in &inv.env {
            cmd.env(k, v);
        }
        if !inv.streaming {
            // Background processes that aren't streaming get their stdio silenced
            // so they don't interleave with the foreground command's output.
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }
        let child = cmd.spawn()
            .with_context(|| format!("spawning {} {:?}", inv.program, inv.args))?;
        Ok(Box::new(SystemChild { child: Some(child) }))
    }
}

/// Test double. Records every invocation; returns `default_output` or a programmed response.
pub struct MockInvoker {
    invocations: Mutex<Vec<Invocation>>,
    pub default_output: Output,
}

impl MockInvoker {
    pub fn new() -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            default_output: Output::default(),
        }
    }
    pub fn invocations(&self) -> Vec<Invocation> {
        self.invocations.lock().unwrap().clone()
    }
    pub fn assert_called(&self, program: &str, args: &[&str]) {
        let calls = self.invocations();
        let wanted_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        assert!(
            calls.iter().any(|i| i.program == program && i.args == wanted_args),
            "expected call: {} {:?}\nactual calls: {:#?}",
            program, wanted_args, calls
        );
    }
}

impl Invoker for MockInvoker {
    fn run(&self, inv: &Invocation) -> Result<Output> {
        self.invocations.lock().unwrap().push(inv.clone());
        Ok(self.default_output.clone())
    }

    fn spawn(&self, inv: &Invocation) -> Result<Box<dyn SpawnedProcess>> {
        self.invocations.lock().unwrap().push(inv.clone());
        Ok(Box::new(NoopSpawned))
    }
}

/// No-op spawned process for MockInvoker — kill/drop are free.
struct NoopSpawned;
impl SpawnedProcess for NoopSpawned {
    fn kill(&mut self) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mock_records_invocation() {
        let mock = MockInvoker::new();
        let inv = Invocation::new("echo", PathBuf::from("/tmp")).arg("hi");
        let out = mock.run(&inv).unwrap();
        assert_eq!(out.status, 0);
        assert_eq!(mock.invocations().len(), 1);
        mock.assert_called("echo", &["hi"]);
    }

    #[test]
    fn system_invoker_runs_real_command() {
        let inv = Invocation::new("echo", PathBuf::from(".")).arg("hello");
        let out = SystemInvoker.run(&inv).unwrap();
        assert_eq!(out.status, 0);
        assert!(out.stdout.contains("hello"));
    }
}
