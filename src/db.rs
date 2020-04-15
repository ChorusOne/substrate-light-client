use parity_scale_codec::alloc::collections::{HashMap, BTreeMap};
use parity_scale_codec::{Error, Input, Output, Encode, Decode};
use kvdb::{DBValue, KeyValueDB, DBTransaction, DBOp};
use std::io;
use parity_util_mem::MallocSizeOf;
use parking_lot::RwLock;

#[derive(Default, MallocSizeOf)]
pub struct IBCDB {
    columns: RwLock<HashMap<u32, BTreeMap<Vec<u8>, DBValue>>>
}


pub fn create(num_cols: u32) -> IBCDB {
    let mut cols = HashMap::new();

    for idx in 0..num_cols {
        cols.insert(idx, BTreeMap::new());
    }

    IBCDB { columns: RwLock::new(cols) }
}

impl KeyValueDB for IBCDB {
    fn get(&self, col: u32, key: &[u8]) -> io::Result<Option<DBValue>> {
        let columns = self.columns.read();
        match columns.get(&col) {
            None => Err(io::Error::new(io::ErrorKind::Other, format!("No such column family: {:?}", col))),
            Some(map) => Ok(map.get(key).cloned()),
        }
    }

    fn get_by_prefix(&self, col: u32, prefix: &[u8]) -> Option<Box<[u8]>> {
        let columns = self.columns.read();
        match columns.get(&col) {
            None => None,
            Some(map) => {
                map.iter().find(|&(ref k, _)| k.starts_with(prefix)).map(|(_, v)| v.to_vec().into_boxed_slice())
            }
        }
    }

    fn write_buffered(&self, transaction: DBTransaction) {
        let mut columns = self.columns.write();
        let ops = transaction.ops;
        for op in ops {
            match op {
                DBOp::Insert { col, key, value } => {
                    if let Some(col) = columns.get_mut(&col) {
                        col.insert(key.into_vec(), value);
                    }
                }
                DBOp::Delete { col, key } => {
                    if let Some(col) = columns.get_mut(&col) {
                        col.remove(&*key);
                    }
                }
            }
        }
    }

    fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    fn iter<'a>(&'a self, col: u32) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        match self.columns.read().get(&col) {
            Some(map) => Box::new(
                // TODO: worth optimizing at all?
                map.clone().into_iter().map(|(k, v)| (k.into_boxed_slice(), v.into_boxed_slice())),
            ),
            None => Box::new(None.into_iter()),
        }
    }

    fn iter_from_prefix<'a>(
        &'a self,
        col: u32,
        prefix: &'a [u8],
    ) -> Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a> {
        match self.columns.read().get(&col) {
            Some(map) => Box::new(
                map.clone()
                    .into_iter()
                    .filter(move |&(ref k, _)| k.starts_with(prefix))
                    .map(|(k, v)| (k.into_boxed_slice(), v.into_boxed_slice())),
            ),
            None => Box::new(None.into_iter()),
        }
    }

    fn restore(&self, _new_db: &str) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "Attempted to restore in-memory database"))
    }
}

impl Encode for IBCDB {
    fn encode_to<T: Output>(&self, dest: &mut T) {
        let columns = self.columns.read();
        let column_length = columns.len() as u32;
        column_length.encode_to(dest);
        for i in 0..column_length {
            let column = columns.get(&i).unwrap();
            column.encode_to(dest);
        }
    }
}

impl Decode for IBCDB {
    fn decode<I: Input>(value: &mut I) -> Result<Self, Error> {
        let length = u32::decode(value)?;

        let mut ibcdb = IBCDB::default();
        let mut map : HashMap<u32, BTreeMap<Vec<u8>, DBValue>> = HashMap::new();

        for i in 0..length {
            let v: BTreeMap<Vec<u8>, DBValue> = BTreeMap::decode(value)?;
            map.insert(i, v);
        }

        ibcdb.columns = RwLock::new(map);

        return Ok(ibcdb)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{IBCDB, create};
    use kvdb::KeyValueDB;
    use parity_scale_codec::{Encode, Decode};

    #[test]
    fn encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        let encoded_db = db.encode();
        assert!(encoded_db.len() > 0);
        let decoded_db = IBCDB::decode(&mut encoded_db.as_slice()).unwrap();

        assert_eq!(decoded_db.get(0, b"key1").unwrap().unwrap(), b"horse");
        assert_eq!(decoded_db.get(1, b"key2").unwrap().unwrap(), b"pigeon");
        assert_eq!(decoded_db.get(1, b"key3").unwrap().unwrap(), b"cat");
    }

    #[test]
    fn deterministic_encode_decode() {
        let db = create(2);
        let mut transaction = db.transaction();
        transaction.put(0, b"key1", b"horse");
        transaction.put(1, b"key2", b"pigeon");
        transaction.put(1, b"key3", b"cat");
        assert!(db.write(transaction).is_ok());

        // First test: If two IBCDB instance are identical, their
        // deserialization need to produce same binary data.
        for _i in 0..100 {
            // Serialization
            let encoded_db = db.encode();
            assert!(encoded_db.len() > 0);
            // Deserialization
            let decoded_db = IBCDB::decode(&mut encoded_db.as_slice()).unwrap();
            // Deserialization need to produce same data every time
            assert_eq!(encoded_db.as_slice(), decoded_db.encode().as_slice());
        }

        // Second test: If two instances of IBCDB are created from same binary blob,
        // and if we insert same data on both instance, then
        // both instance should produce same binary blob
        let encoded_db = db.encode();
        let decoded_db = IBCDB::decode(&mut encoded_db.as_slice()).unwrap();

        let mut transaction = db.transaction();
        transaction.put(0, b"another_format", b"pikachu");
        let duplicate_transaction = transaction.clone();
        // Insert into original db
        assert!(db.write(transaction).is_ok());
        // Insert into an instance created from previous state of original db
        assert!(decoded_db.write(duplicate_transaction).is_ok());

        assert_eq!(db.encode().as_slice(), decoded_db.encode().as_slice());
    }
}
