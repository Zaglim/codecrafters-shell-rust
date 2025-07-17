use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub trait Executable {
    fn is_executable_file(&self) -> bool;
    fn first_executable_match_in_path(&self) -> Option<Box<Path>>;
}

impl<S> Executable for S
where
    S: AsRef<OsStr>,
{
    fn is_executable_file(&self) -> bool {
        let Ok(metadata) = Path::new(self).metadata() else {
            return false;
        };
        let permission_bits = metadata.mode();
        metadata.is_file() && permission_bits & 0o000_000_001 != 0
    }

    fn first_executable_match_in_path(&self) -> Option<Box<Path>> {
        for path_str in std::env::var("PATH").unwrap().split(':') {
            let path_buf = Path::new(path_str).join(self.as_ref());
            let exe = path_buf.is_executable_file();
            if exe {
                return Some(path_buf.into_boxed_path());
            }
        }
        None
    }
}
