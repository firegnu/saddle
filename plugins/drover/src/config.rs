use std::path::PathBuf;
pub fn expand_home(path: &str) -> PathBuf {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(path.strip_prefix("~/").unwrap_or(""));
    }
    PathBuf::from(path)
}
