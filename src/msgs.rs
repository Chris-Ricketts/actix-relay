use actix_broker::BrokerMsg;
use bytes::Bytes;
use serde::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialize, deserialize};

use std::any::TypeId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub trait RelayMessage: BrokerMsg + Serialize + DeserializeOwned {
    fn tag() -> u64;
    fn into_relay_data(self) -> RelayData;
    fn from_byte_slice(bs: &[u8]) -> Option<Self>;
}

impl<M> RelayMessage for M
where 
    M: BrokerMsg + Serialize + DeserializeOwned
{
    fn tag() -> u64 {
        let type_id = TypeId::of::<M>();
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        hasher.finish()
    }

    fn into_relay_data(self) -> RelayData {
        let payload = serialize(&self).unwrap();
        let tag = Self::tag();
        let rd = TaggedData {
            tag, 
            data: payload
        };
        let serialized = serialize(&rd).unwrap();
        RelayData(Bytes::from(serialized))
    }

    fn from_byte_slice(bs: &[u8]) -> Option<M> {
        deserialize::<M>(bs).ok()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaggedData {
    pub tag: u64, 
    pub data: Vec<u8>,
}

impl TaggedData {
    pub fn from_relay_data(rd: &RelayData) -> Option<TaggedData> {
        deserialize::<Self>(&rd.0).ok()
    }
}

// TODO Change result type to io::Result<()> 
// Possibly just implement Message for Bytes? 
#[derive(Message)]
pub struct RelayData(pub Bytes);

impl From<Bytes> for RelayData {
    fn from(bytes: Bytes) -> RelayData {
        RelayData(bytes)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Clone, PartialEq,  Message, Serialize, Deserialize)]
    enum TestMessage {
        Op1,
        Op2,
        Op3,
    }

    #[test]
    fn relay_message_to_relay_data_and_back() {
        let msg = TestMessage::Op1;

        let relay_data = msg.into_relay_data();

        let tagged_data = TaggedData::from_relay_data(&relay_data).unwrap();

        assert_eq!(tagged_data.tag, TestMessage::tag());

        let reconstructed = TestMessage::from_byte_slice(&tagged_data.data);

        assert_eq!(reconstructed, Some(TestMessage::Op1));
    }
}
