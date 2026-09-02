// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const POINTER: &str = "library.json";
const DATABASE: &str = "ampello.db";

// Where a shared library goes by default. `C:\Users\Public` is writable by
// every account on the machine without any permission work, which ProgramData
// is not: a folder created there by one user is not writable by the next.
const PUBLIC_FOLDER: &str = "Ampello";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Pointer {
    #[serde(default)]
    shared_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub dir: PathBuf,
    pub personal_dir: PathBuf,
    pub shared: bool,
    // Set when a shared library was configured but could not be used, so the
    // interface can say why it is looking at the personal one instead.
    pub problem: Option<String>,
}

impl Resolved {
    pub fn database_path(&self) -> PathBuf {
        self.dir.join(DATABASE)
    }
}

pub fn default_shared_dir() -> PathBuf {
    let public = std::env::var_os("PUBLIC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"));
    public.join(PUBLIC_FOLDER)
}

/// Decide which library this account should open.
///
/// A missing or unreadable pointer means the personal library, which is what
/// every account gets until someone deliberately points it elsewhere. A
/// configured shared library that cannot be reached - an external drive that is
/// not plugged in, a folder someone deleted - falls back to personal rather
/// than refusing to start, and says so.
pub fn resolve(personal_dir: &Path) -> Resolved {
    let personal = Resolved {
        dir: personal_dir.to_path_buf(),
        personal_dir: personal_dir.to_path_buf(),
        shared: false,
        problem: None,
    };

    let raw = match std::fs::read_to_string(personal_dir.join(POINTER)) {
        Ok(raw) => raw,
        Err(_) => return personal,
    };
    let pointer: Pointer = match serde_json::from_str(&raw) {
        Ok(pointer) => pointer,
        Err(error) => {
            log::warn!("the library pointer could not be read: {error}");
            return personal;
        }
    };
    let Some(path) = pointer.shared_path.filter(|p| !p.trim().is_empty()) else {
        return personal;
    };

    let dir = PathBuf::from(path);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        return Resolved {
            problem: Some(format!("{} is not reachable: {error}", dir.display())),
            ..personal
        };
    }
    if let Err(error) = writable(&dir) {
        return Resolved {
            problem: Some(format!("{} cannot be written to: {error}", dir.display())),
            ..personal
        };
    }

    Resolved {
        dir,
        personal_dir: personal_dir.to_path_buf(),
        shared: true,
        problem: None,
    }
}

/// Point this account at a shared library, or back at its own.
pub fn set(personal_dir: &Path, shared: Option<&Path>) -> std::io::Result<()> {
    if let Some(dir) = shared {
        std::fs::create_dir_all(dir)?;
        writable(dir)?;
    }
    let pointer = Pointer {
        shared_path: shared.map(|dir| dir.to_string_lossy().into_owned()),
    };
    std::fs::create_dir_all(personal_dir)?;
    let body = serde_json::to_string_pretty(&pointer)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(personal_dir.join(POINTER), body)
}

// Checked by writing rather than by reading permissions: an account can hold
// rights it cannot use, and a folder on a disconnected drive reports nothing
// useful at all. A shared library that turns out to be read-only at the first
// edit is worse than one refused here.
fn writable(dir: &Path) -> std::io::Result<()> {
    let probe = dir.join(format!(".ampello-{}.tmp", std::process::id()));
    std::fs::write(&probe, b"")?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ampello-library-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_account_gets_its_own_library_until_it_says_otherwise() {
        let personal = temp("default");
        let resolved = resolve(&personal);
        assert!(!resolved.shared);
        assert_eq!(resolved.dir, personal);
        assert!(resolved.problem.is_none());
    }

    #[test]
    fn pointing_at_a_shared_folder_moves_the_library_there() {
        let personal = temp("share-personal");
        let shared = temp("share-shared");

        set(&personal, Some(&shared)).unwrap();
        let resolved = resolve(&personal);

        assert!(resolved.shared);
        assert_eq!(resolved.dir, shared);
        assert_eq!(resolved.database_path(), shared.join("ampello.db"));
        // The personal directory is still where the pointer lives, so the
        // account can always find its way back.
        assert_eq!(resolved.personal_dir, personal);
    }

    #[test]
    fn reverting_returns_to_the_personal_library_and_leaves_the_shared_one() {
        let personal = temp("revert-personal");
        let shared = temp("revert-shared");
        std::fs::write(shared.join("ampello.db"), b"pretend").unwrap();

        set(&personal, Some(&shared)).unwrap();
        set(&personal, None).unwrap();

        let resolved = resolve(&personal);
        assert!(!resolved.shared);
        assert_eq!(resolved.dir, personal);
        assert!(shared.join("ampello.db").is_file());
    }

    #[test]
    fn an_unreachable_shared_library_falls_back_rather_than_failing() {
        // An external drive that is not plugged in, or a folder someone
        // deleted. Refusing to start would leave the user with no way to
        // change the setting that is blocking them.
        let personal = temp("missing-personal");

        // A regular file cannot contain a directory, so this stands in for any
        // path that cannot be created: an unplugged drive, a deleted folder.
        let blocker = personal.join("blocker");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let gone = blocker.join("library");

        std::fs::write(
            personal.join(POINTER),
            serde_json::to_string(&Pointer {
                shared_path: Some(gone.to_string_lossy().into_owned()),
            })
            .unwrap(),
        )
        .unwrap();

        let resolved = resolve(&personal);
        assert!(!resolved.shared);
        assert_eq!(resolved.dir, personal);
        assert!(resolved.problem.is_some(), "the reason must be reported");
    }

    #[test]
    fn a_corrupt_pointer_is_ignored_rather_than_fatal() {
        let personal = temp("corrupt");
        std::fs::write(personal.join(POINTER), b"{ this is not json").unwrap();

        let resolved = resolve(&personal);
        assert!(!resolved.shared);
        assert_eq!(resolved.dir, personal);
    }

    #[test]
    fn two_accounts_pointing_at_one_folder_resolve_to_the_same_database() {
        let alice = temp("alice");
        let bob = temp("bob");
        let shared = temp("shared-both");

        set(&alice, Some(&shared)).unwrap();
        set(&bob, Some(&shared)).unwrap();

        assert_eq!(
            resolve(&alice).database_path(),
            resolve(&bob).database_path()
        );
        // And each still keeps its own pointer, so either can leave alone.
        set(&bob, None).unwrap();
        assert!(resolve(&alice).shared);
        assert!(!resolve(&bob).shared);
    }
}
