mod grandpa;
mod babe;

use sp_runtime::traits::{Block as BlockT};

fn ingest_finalized_header<B: BlockT>(finalized_header: &B::Header) -> Result<(), &str> {
    let _scheduled_authority_change = grandpa::find_scheduled_change::<B>(finalized_header);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
