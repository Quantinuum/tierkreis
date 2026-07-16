use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaggedResultValue {
    Int(i64),
    Bool(bool),
    Float(f64),
    IntArr(Vec<i64>),
    BoolArr(Vec<bool>),
    FloatArr(Vec<f64>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaggedResult(pub String, pub TaggedResultValue);

#[derive(Debug, Serialize, Deserialize)]
pub struct QSysResultAttributes {
    results: Vec<Vec<TaggedResult>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QSysResultData {
    attributes: QSysResultAttributes,
}

impl QSysResultData {
    pub fn results_ref(&self) -> &Vec<Vec<TaggedResult>> {
        &self.attributes.results
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QSysResult {
    data: QSysResultData,
}

impl QSysResult {
    pub fn data(self) -> QSysResultData {
        self.data
    }
}
