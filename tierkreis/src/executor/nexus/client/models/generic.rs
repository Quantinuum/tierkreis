use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct NewRelationshipData {
    id: Uuid,
    r#type: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NewRelationship {
    data: NewRelationshipData,
}

impl NewRelationship {
    pub fn new(id: Uuid, r#type: &'static str) -> Self {
        NewRelationship {
            data: NewRelationshipData { id, r#type },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Data {
    id: Uuid,
}

impl Data {
    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Debug, Deserialize)]
pub struct CollectionDocument {
    data: Vec<Data>,
}

impl CollectionDocument {
    pub fn last_data(mut self) -> Option<Data> {
        self.data.pop()
    }
}

#[derive(Debug, Deserialize)]
pub struct Document {
    data: Data,
}

impl Document {
    pub fn data(self) -> Data {
        self.data
    }
}
