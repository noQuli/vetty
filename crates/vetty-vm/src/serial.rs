use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::{ChildStdin, ChildStdout};
use std::thread;

use anyhow::Result;
use nix::libc;
use nix::sys::termios::{self, SetArg, Termios};

struct TerminalGuard {
    original: Termios,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let stdin = std::io::stdin();
        let _ = termios::tcsetattr(&stdin, SetArg::TCSANOW, &self.original);
    }
}

pub fn attach_serial(child_stdin: &mut ChildStdin, child_stdout: &mut ChildStdout) -> Result<()> {
    let stdin = std::io::stdin();
    let original = termios::tcgetattr(&stdin)?;
    let mut raw = original.clone();
    termios::cfmakeraw(&mut raw);
    termios::tcsetattr(&stdin, SetArg::TCSANOW, &raw)?;
    let _guard = TerminalGuard { original };

    let mut child_in = duplicate_as_file(child_stdin.as_raw_fd())?;
    let output_thread = {
        let mut out = duplicate_as_file(child_stdout.as_raw_fd())?;
        thread::spawn(move || -> Result<()> {
            let mut host_stdout = std::io::stdout();
            let mut buffer = [0u8; 4096];
            loop {
                let bytes = out.read(&mut buffer)?;
                if bytes == 0 {
                    break;
                }
                host_stdout.write_all(&buffer[..bytes])?;
                host_stdout.flush()?;
            }
            Ok(())
        })
    };

    let mut host_stdin = std::io::stdin();
    let mut input = [0u8; 4096];
    loop {
        let bytes = host_stdin.read(&mut input)?;
        if bytes == 0 {
            break;
        }
        if let Some(pos) = input[..bytes].iter().position(|byte| *byte == 0x1d) {
            if pos > 0 {
                child_in.write_all(&input[..pos])?;
                child_in.flush()?;
            }
            break;
        } else {
            child_in.write_all(&input[..bytes])?;
            child_in.flush()?;
        }
    }

    output_thread
        .join()
        .map_err(|_| anyhow::anyhow!("serial output thread panicked"))??;
    Ok(())
}

fn duplicate_as_file(fd: i32) -> Result<File> {
    let duplicated = unsafe { libc::dup(fd) };
    if duplicated < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(duplicated) };
    Ok(file)
}
