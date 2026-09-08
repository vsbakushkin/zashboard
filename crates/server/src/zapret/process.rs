use std::{fs, io, path::Path};

const NFQWS2_PROCESS_NAME: &str = "nfqws2";

pub fn find_nfqws2_pid() -> io::Result<Option<u32>> {
    find_process_pid(Path::new("/proc"), NFQWS2_PROCESS_NAME)
}

pub fn is_nfqws2_running() -> io::Result<bool> {
    find_nfqws2_pid().map(|pid| pid.is_some())
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
}
