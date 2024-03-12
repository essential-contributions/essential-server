use placeholder::{EoaPermit, Signed};

#[cfg(test)]
mod tests;

pub async fn submit_permit<S>(storage: &S, permit: Signed<EoaPermit>) -> anyhow::Result<()> {
    todo!()
}
