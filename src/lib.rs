mod grandpa;
mod babe;

use sp_runtime::traits::{Block as BlockT};

fn ingest_finalized_header<B: BlockT>(finalized_header: &B::Header) -> Result<(), &str> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
