//! **Accounts** (#143): `<data-dir>/accounts.toml`, each account's name and the character guids it
//! owns. An unknown name registers on its first login; the join password is the server's only
//! secret, so an account is a name and nothing more.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use benilla_protocol::SessionEvent;
use twilight_realm::RealmCommand;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct File {
    #[serde(default)]
    accounts: BTreeMap<String, Account>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
struct Account {
    #[serde(default)]
    characters: Vec<u64>,
}

pub(crate) struct Accounts {
    path: PathBuf,
    file: File,
}

impl Accounts {
    /// Reads `<data_dir>/accounts.toml`; a missing file is no accounts yet.
    pub(crate) fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("accounts.toml");
        let file = match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                panic!("twilight-server: {} does not parse: {e}", path.display())
            }),
            Err(_) => File::default(),
        };
        Self { path, file }
    }

    /// Normalizes a login name: accounts are case-blind, as 1.12's are.
    pub(crate) fn key(name: &str) -> String {
        name.trim().to_uppercase()
    }

    /// Registers `account` if it is new.
    pub(crate) fn register(&mut self, account: &str) {
        if !self.file.accounts.contains_key(account) {
            self.file
                .accounts
                .insert(account.to_string(), Account::default());
            eprintln!("twilight-server: registered account {account}");
            self.write();
        }
    }

    fn owns(&self, account: &str, guid: u64) -> bool {
        self.file
            .accounts
            .get(account)
            .is_some_and(|a| a.characters.contains(&guid))
    }

    fn owner(&self, guid: u64) -> Option<&str> {
        self.file
            .accounts
            .iter()
            .find(|(_, a)| a.characters.contains(&guid))
            .map(|(n, _)| n.as_str())
    }

    /// Gives a just-saved `guid` no account holds to `account`: the character it just created.
    pub(crate) fn claim(&mut self, account: &str, guid: u64) {
        if self.owner(guid).is_some() {
            return;
        }
        if let Some(a) = self.file.accounts.get_mut(account) {
            a.characters.push(guid);
            self.write();
        }
    }

    /// Forgets a deleted character.
    pub(crate) fn release(&mut self, guid: u64) {
        for a in self.file.accounts.values_mut() {
            a.characters.retain(|&g| g != guid);
        }
        self.write();
    }

    /// Whether `account` may send `command`: a character it names must be its own.
    pub(crate) fn allows(&self, account: &str, command: &RealmCommand) -> bool {
        match command {
            RealmCommand::EnterWorld { guid } | RealmCommand::DeleteCharacter { guid } => {
                self.owns(account, *guid)
            }
            _ => true,
        }
    }

    /// `event` as `account` sees it: a roster lists only its own characters.
    pub(crate) fn filter(&self, account: &str, mut event: SessionEvent) -> SessionEvent {
        if let SessionEvent::CharacterList { characters, .. } = &mut event {
            characters.retain(|c| self.owns(account, c.guid));
        }
        event
    }

    fn write(&self) {
        let text = toml::to_string(&self.file).expect("accounts serialize");
        if let Err(e) = std::fs::create_dir_all(self.path.parent().unwrap_or(Path::new(".")))
            .and_then(|()| std::fs::write(&self.path, text))
        {
            eprintln!("twilight-server: cannot write {}: {e}", self.path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roster_lists_only_the_accounts_own_and_a_claim_sticks() {
        let dir = std::env::temp_dir().join(format!("twilight-accounts-{}", std::process::id()));
        let mut a = Accounts::load(&dir);
        a.register("ANNA");
        a.register("BO");
        a.claim("ANNA", 7);
        a.claim("BO", 7);
        assert!(a.allows("ANNA", &RealmCommand::EnterWorld { guid: 7 }));
        assert!(!a.allows("BO", &RealmCommand::EnterWorld { guid: 7 }));
        let reloaded = Accounts::load(&dir);
        assert!(reloaded.owns("ANNA", 7));
        a.release(7);
        assert!(!a.owns("ANNA", 7));
        std::fs::remove_dir_all(&dir).ok();
    }
}
