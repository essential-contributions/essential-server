// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use essential_types::{intent::Intent, ContentAddress};
use placeholder::Signed;
use storage::Storage;

#[cfg(test)]
mod tests;

pub async fn deploy<S>(storage: &S, intent: Signed<Vec<Intent>>) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    todo!()
}
