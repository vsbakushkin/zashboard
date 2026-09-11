use super::argument::{self, Argument};
use std::{fs, io, path::Path, str};

const NFQWS2_PROCESS_NAME: &str = "nfqws2";

#[derive(Debug)]
pub struct NfqwsProcess {
    pub pid: u32,
    pub cmdline: Vec<u8>,
}

impl NfqwsProcess {
    pub fn command(&self) -> Option<&[u8]> {
        self.cmdline
            .split(|byte| *byte == 0)
            .next()
            .filter(|command| !command.is_empty())
    }

    pub fn args(&self) -> impl Iterator<Item = &[u8]> {
        self.cmdline
            .strip_suffix(&[0])
            .unwrap_or(&self.cmdline)
            .split(|byte| *byte == 0)
            .skip(1)
    }

    pub fn parsed_args(&self) -> impl Iterator<Item = Argument<'_>> {
        self.args().filter_map(|arg| argument::parse(arg))
    }

    pub fn find_arg(&self, name: &[u8]) -> Option<Argument<'_>> {
        self.parsed_args().find(|arg| arg.name == name)
    }

    pub fn find_args<'a>(&'a self, name: &'a [u8]) -> impl Iterator<Item = Argument<'a>> {
        self.parsed_args().filter(move |arg| arg.name == name)
    }

    pub fn queue_number(&self) -> Option<u16> {
        let value = self.find_arg(b"qnum")?.value?;
        let value = std::str::from_utf8(value).ok()?;
        value.parse().ok()
    }
}

pub fn find_nfqws2_process() -> io::Result<Option<NfqwsProcess>> {
    find_process(Path::new("/proc"), NFQWS2_PROCESS_NAME)
}

pub fn find_nfqws2_pid() -> io::Result<Option<u32>> {
    find_process_pid(Path::new("/proc"), NFQWS2_PROCESS_NAME)
}

pub fn is_nfqws2_running() -> io::Result<bool> {
    find_nfqws2_pid().map(|pid| pid.is_some())
}

fn find_process(proc_root: &Path, process_name: &str) -> io::Result<Option<NfqwsProcess>> {
    let Some(pid) = find_process_pid(proc_root, process_name)? else {
        return Ok(None);
    };

    let cmdline = read_process_cmdline(proc_root, pid)?;
    Ok(Some(NfqwsProcess { pid, cmdline }))
}

fn find_process_pid(proc_root: &Path, process_name: &str) -> io::Result<Option<u32>> {
    for entry in fs::read_dir(proc_root)? {
        let entry = entry?;
        let name = entry.file_name();

        let Some(pid) = name.to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };

        let comm = match fs::read_to_string(entry.path().join("comm")) {
            Ok(comm) => comm,
            Err(e) if matches!(e.kind(), io::ErrorKind::NotFound) => continue,
            Err(e) => return Err(e),
        };

        if comm.trim_end_matches('\n') == process_name {
            return Ok(Some(pid));
        }
    }

    Ok(None)
}

fn read_process_cmdline(proc_root: &Path, pid: u32) -> io::Result<Vec<u8>> {
    let path = proc_root.join(pid.to_string()).join("cmdline");
    fs::read(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn create_fake_proc(test_name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("zashboard-{test_name}-{}", std::process::id()));

        match fs::remove_dir_all(&root) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove stale fake proc: {error}"),
        }

        fs::create_dir(&root).expect("failed to create fake proc");
        root
    }

    #[test]
    fn extracts_command_from_cmdline() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
        };

        let result = process.command();

        assert_eq!(result, Some(&b"/opt/zapret2/nfq2/nfqws2"[..]));
    }

    #[test]
    fn returns_none_when_command_is_empty() {
        for cmdline in [b"".as_slice(), b"\0--qnum=200\0".as_slice()] {
            let process = NfqwsProcess {
                pid: 123,
                cmdline: cmdline.to_vec(),
            };

            let result = process.command();

            assert!(result.is_none());
        }
    }

    #[test]
    fn returns_process_with_pid_and_cmdline() {
        let proc_root = create_fake_proc("process-details");
        let process_dir = proc_root.join("123");
        let expected = b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0";

        fs::create_dir(&process_dir).expect("failed to create fake process");
        fs::write(process_dir.join("comm"), "nfqws2\n").expect("failed to write comm");
        fs::write(process_dir.join("cmdline"), expected).expect("failed to write cmdline");

        let result = find_process(&proc_root, NFQWS2_PROCESS_NAME);

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        let process = result
            .expect("process inspection should succeed")
            .expect("process should be found");

        assert_eq!(process.pid, 123);
        assert_eq!(process.cmdline, expected);
    }

    #[test]
    fn extracts_arguments_from_cmdline() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0--fwmark=0x10000000\0".to_vec(),
        };

        let args: Vec<_> = process.args().collect();

        assert_eq!(args, [&b"--qnum=200"[..], &b"--fwmark=0x10000000"[..],]);
    }

    #[test]
    fn parses_process_arguments() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0--debug\0".to_vec(),
        };

        let args: Vec<_> = process.parsed_args().collect();

        assert_eq!(
            args,
            [
                Argument {
                    name: b"qnum",
                    value: Some(b"200"),
                },
                Argument {
                    name: b"debug",
                    value: None,
                },
            ]
        );
    }

    #[test]
    fn finds_argument_by_name() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0--fwmark=0x10000000\0".to_vec(),
        };

        assert_eq!(
            process.find_arg(b"qnum"),
            Some(Argument {
                name: b"qnum",
                value: Some(b"200"),
            })
        );
    }

    #[test]
    fn finds_argument_without_value() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--debug\0".to_vec(),
        };

        assert_eq!(
            process.find_arg(b"debug"),
            Some(Argument {
                name: b"debug",
                value: None,
            })
        );
    }

    #[test]
    fn returns_none_when_argument_is_absent() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
        };

        assert_eq!(process.find_arg(b"fwmark"), None);
    }

    #[test]
    fn returns_first_argument_when_name_is_repeated() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--lua-init=first.lua\0--lua-init=second.lua\0"
                .to_vec(),
        };

        assert_eq!(
            process.find_arg(b"lua-init"),
            Some(Argument {
                name: b"lua-init",
                value: Some(b"first.lua"),
            })
        );
    }

    #[test]
    fn ignores_invalid_process_arguments() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0invalid\0--debug\0".to_vec(),
        };

        let args: Vec<_> = process.parsed_args().collect();

        assert_eq!(
            args,
            [
                Argument {
                    name: b"qnum",
                    value: Some(b"200"),
                },
                Argument {
                    name: b"debug",
                    value: None,
                },
            ]
        );
    }

    #[test]
    fn returns_no_parsed_arguments_when_process_has_no_arguments() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0".to_vec(),
        };

        let args: Vec<_> = process.parsed_args().collect();

        assert!(args.is_empty());
    }

    #[test]
    fn returns_no_arguments_when_process_has_only_command() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0".to_vec(),
        };

        let args: Vec<_> = process.args().collect();

        assert!(args.is_empty());
    }

    #[test]
    fn returns_no_arguments_when_cmdline_is_empty() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: Vec::new(),
        };

        let args: Vec<_> = process.args().collect();

        assert!(args.is_empty());
    }

    #[test]
    fn finds_all_arguments_by_name() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0\
                        --lua-init=first.lua\0\
                        --qnum=200\0\
                        --lua-init=second.lua\0"
                .to_vec(),
        };

        let args: Vec<_> = process.find_args(b"lua-init").collect();

        assert_eq!(
            args,
            [
                Argument {
                    name: b"lua-init",
                    value: Some(b"first.lua"),
                },
                Argument {
                    name: b"lua-init",
                    value: Some(b"second.lua"),
                },
            ]
        );
    }

    #[test]
    fn returns_no_arguments_when_name_is_absent() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
        };

        let args: Vec<_> = process.find_args(b"lua-init").collect();

        assert!(args.is_empty());
    }

    #[test]
    fn finds_repeated_arguments_without_values() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--debug\0--qnum=200\0--debug\0".to_vec(),
        };

        let args: Vec<_> = process.find_args(b"debug").collect();

        assert_eq!(
            args,
            [
                Argument {
                    name: b"debug",
                    value: None,
                },
                Argument {
                    name: b"debug",
                    value: None,
                },
            ]
        );
    }

    #[test]
    fn returns_queue_number() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0".to_vec(),
        };

        assert_eq!(process.queue_number(), Some(200));
    }

    #[test]
    fn returns_none_when_queue_number_is_absent() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--fwmark=0x10000000\0".to_vec(),
        };

        assert_eq!(process.queue_number(), None);
    }

    #[test]
    fn returns_none_when_queue_number_has_no_value() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum\0".to_vec(),
        };

        assert_eq!(process.queue_number(), None);
    }

    #[test]
    fn returns_none_when_queue_number_is_invalid() {
        let process = NfqwsProcess {
            pid: 123,
            cmdline: b"/opt/zapret2/nfq2/nfqws2\0--qnum=abc\0".to_vec(),
        };

        assert_eq!(process.queue_number(), None);
    }

    #[test]
    fn returns_none_when_process_details_are_absent() {
        let proc_root = create_fake_proc("details-absent");

        let result = find_process(&proc_root, NFQWS2_PROCESS_NAME);
        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");
        let process = result.expect("process inspection should succeed");
        assert!(process.is_none())
    }

    #[test]
    fn propagates_error_when_cmdline_is_missing() {
        let proc_root = create_fake_proc("missing-cmdline");
        let process_dir = proc_root.join("123");

        fs::create_dir(&process_dir).expect("failed to create fake process");
        fs::write(process_dir.join("comm"), "nfqws2\n").expect("failed to write comm");

        let result = find_process(&proc_root, NFQWS2_PROCESS_NAME);

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        let error = result.expect_err("missing cmdline should produce an error");

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn returns_none_when_process_is_absent() {
        let proc_root = create_fake_proc("process-absent");

        let pid = find_process_pid(&proc_root, NFQWS2_PROCESS_NAME)
            .expect("process inspection should succeed");

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        assert!(pid.is_none());
    }

    #[test]
    fn returns_pid_when_process_is_present() {
        let proc_root = create_fake_proc("process-present");
        let process_dir = proc_root.join("123");

        fs::create_dir(&process_dir).expect("failed to create fake process");
        fs::write(process_dir.join("comm"), "nfqws2\n").expect("failed to write process name");

        let pid = find_process_pid(&proc_root, NFQWS2_PROCESS_NAME)
            .expect("process inspection should succeed");

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        assert_eq!(pid, Some(123));
    }

    #[test]
    fn ignores_non_numeric_proc_entries() {
        let proc_root = create_fake_proc("non-numeric-entry");
        let non_process_dir = proc_root.join("self");

        fs::create_dir(&non_process_dir).expect("failed to create fake process");
        fs::write(non_process_dir.join("comm"), "nfqws2\n").expect("failed to write process name");

        let pid = find_process_pid(&proc_root, NFQWS2_PROCESS_NAME)
            .expect("process inspection should succeed");

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        assert!(pid.is_none());
    }

    #[test]
    fn ignores_process_that_disappeared() {
        let proc_root = create_fake_proc("disappeared-process");
        let process_dir = proc_root.join("456");

        fs::create_dir(&process_dir).expect("failed to create fake process");

        // don't create comm

        let pid = find_process_pid(&proc_root, NFQWS2_PROCESS_NAME)
            .expect("missing comm should be tolerated");

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        assert!(pid.is_none());
    }

    #[test]
    fn propagates_error_when_comm_cannot_be_read() {
        let proc_root = create_fake_proc("unreadable-comm");

        fs::create_dir_all(proc_root.join("789").join("comm"))
            .expect("failed to create comm directory");

        let result = find_process_pid(&proc_root, NFQWS2_PROCESS_NAME);

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        result.expect_err("serious I/O error should be propagated");
    }

    #[test]
    fn reads_process_cmdline_bytes() {
        let proc_root = create_fake_proc("process-cmdline");
        let process_dir = proc_root.join("123");
        let expected = b"/opt/zapret2/nfq2/nfqws2\0--qnum=200\0";

        fs::create_dir(&process_dir).expect("failed to create fake process");
        fs::write(process_dir.join("cmdline"), expected).expect("failed to write cmdline");

        let result = read_process_cmdline(&proc_root, 123);

        fs::remove_dir_all(&proc_root).expect("failed to remove fake proc");

        let bytes = result.expect("cmdline reading should succeed");

        assert_eq!(bytes, expected);
    }
}
