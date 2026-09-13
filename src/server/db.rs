use native_db::*;
use once_cell::sync::Lazy;
use std::ops::{Deref, DerefMut};

use crate::models::{Account, LiveRoom};

pub struct Db<'a> {
    pub inner: native_db::Database<'a>,
}

impl<'a> Deref for Db<'a> {
    type Target = native_db::Database<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for Db<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<crate::models::Account>().unwrap();
    models.define::<crate::models::LiveRoom>().unwrap();
    models
});

pub async fn init_db() -> anyhow::Result<Db<'static>> {
    Ok(Db {
        inner: Builder::new().create(&MODELS, "./data/db")?,
    })
}

impl Db<'_> {
    pub fn find_account(&self, username: &str) -> anyhow::Result<Option<Account>> {
        let transaction = self.r_transaction()?;
        Ok(transaction.get().primary::<Account>(username)?)
    }

    pub fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let transaction = self.r_transaction()?;
        let accounts = transaction
            .scan()
            .primary::<Account>()?
            .all()?
            .collect::<native_db::db_type::Result<Vec<_>>>()?;
        Ok(accounts)
    }

    pub fn insert_account(&self, account: Account) -> anyhow::Result<()> {
        let transaction = self.rw_transaction()?;
        transaction.insert(account)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn upsert_account(&self, mut account: Account) -> anyhow::Result<()> {
        let transaction = self.rw_transaction()?;
        account.updated_at = chrono::Utc::now();
        transaction.upsert(account)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn find_room(&self, stream_key: &str) -> anyhow::Result<Option<LiveRoom>> {
        let transaction = self.r_transaction()?;
        Ok(transaction.get().primary::<LiveRoom>(stream_key)?)
    }

    pub fn list_rooms(&self) -> anyhow::Result<Vec<LiveRoom>> {
        let transaction = self.r_transaction()?;
        let rooms = transaction
            .scan()
            .primary::<LiveRoom>()?
            .all()?
            .collect::<native_db::db_type::Result<Vec<_>>>()?;
        Ok(rooms)
    }

    pub fn insert_room(&self, room: LiveRoom) -> anyhow::Result<()> {
        let transaction = self.rw_transaction()?;
        transaction.insert(room)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn upsert_room(&self, mut room: LiveRoom) -> anyhow::Result<()> {
        let transaction = self.rw_transaction()?;
        room.updated_at = chrono::Utc::now();
        transaction.upsert(room)?;
        transaction.commit()?;
        Ok(())
    }
}
