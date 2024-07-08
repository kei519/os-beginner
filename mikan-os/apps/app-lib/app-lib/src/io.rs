use crate::fs::File;

pub fn stdin() -> File {
    File(0)
}

pub fn stdout() -> File {
    File(1)
}

pub fn stderr() -> File {
    File(2)
}
