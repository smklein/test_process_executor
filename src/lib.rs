use anyhow::anyhow;
use std::ffi::OsStr;
use std::fmt;
use std::process::Command;
use std::str::from_utf8;

/// An execution environment, consisting of environment variables
/// which are provided on the launch of each new process.
pub struct Executor<K, V>
where
    K: AsRef<OsStr> + Clone,
    V: AsRef<OsStr> + Clone,
{
    env: Vec<(K, V)>,
}

impl<K, V> Executor<K, V>
where
    K: AsRef<OsStr> + Clone,
    V: AsRef<OsStr> + Clone,
{
    /// Initializes a new Executor.
    ///
    /// All environment variables are provided to processes launched
    /// with the `run` method.
    pub fn new(env: Vec<(K, V)>) -> Self {
        Executor { env }
    }

    /// Launches a new subprocess and awaits its completion.
    ///
    /// Pretty-prints stdout/stderr on failure.
    ///
    /// # Panics
    ///
    /// This method is a little aggressive about panicking; it
    /// can totally evolve structured errors if that would be useful.
    /// However, given that the primary purpose is testing, this
    /// behavior is *currently* acceptable.
    ///
    /// Panics if...
    /// - `args` is empty.
    /// - The sub-process fails to execute.
    /// - The execution of the sub-process returns a non-zero exit code.
    /// - The sub-process writes invalid UTF-8 stdout/stderr.
    pub fn run<I, S>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Execution::run(args, self.env.clone())
    }
}

struct Execution<S: AsRef<OsStr>> {
    cmd: S,
    args: Vec<S>,
    result: Option<std::process::Output>,
}

impl<S: AsRef<OsStr>> Execution<S> {
    fn run<I, K, V, E>(args: I, envs: E)
    where
        I: IntoIterator<Item = S>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
        E: IntoIterator<Item = (K, V)>,
    {
        let mut iter = args.into_iter();
        let mut exec = Execution {
            cmd: iter
                .next()
                .ok_or_else(|| anyhow!("Missing command"))
                .unwrap(),
            args: iter.collect::<Vec<S>>(),
            result: None,
        };

        exec.result = Some(
            Command::new(&exec.cmd)
                .args(&exec.args)
                .envs(envs)
                .output()
                .expect("Failed to execute command"),
        );
        assert!(
            exec.result.as_ref().unwrap().status.success(),
            format!("{}", exec)
        );
    }
}

impl<S: AsRef<OsStr>> fmt::Display for Execution<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut cmd = Vec::new();
        cmd.push(self.cmd.as_ref());
        for arg in &self.args {
            cmd.push(arg.as_ref());
        }
        let cmd: Vec<String> = cmd
            .into_iter()
            .map(|osstr| osstr.to_string_lossy().to_string())
            .collect();
        write!(f, "\x1b[95m{}\x1b[0m", cmd.join(" "))?;
        if let Some(out) = self.result.as_ref() {
            if !out.status.success() {
                write!(f, "\n{}", out.status)?;
            }
            if !out.stdout.is_empty() {
                write!(f, "\n\x1b[92m{}\x1b[0m", from_utf8(&out.stdout).unwrap())?;
            }
            if !out.stderr.is_empty() {
                write!(f, "\n\x1b[91m{}\x1b[0m", from_utf8(&out.stderr).unwrap())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_right_environment_variable() {
        let executor = Executor::new(vec![("FOO", "BAR")]);
        executor.run(vec!["/bin/bash", "-c", "[ \"$FOO\" == \"BAR\" ]"]);
    }

    #[test]
    #[should_panic]
    fn export_wrong_environment_variable() {
        let executor = Executor::new(vec![("FOO", "BAZINGA")]);
        executor.run(vec!["/bin/bash", "-c", "[ \"$FOO\" == \"BAR\" ]"]);
    }
}
