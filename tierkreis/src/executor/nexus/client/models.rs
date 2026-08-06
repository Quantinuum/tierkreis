pub mod generic;
pub mod hugr;
pub mod jobs;
pub mod projects;
pub mod results;

pub use generic::{CollectionDocument, Data, Document, NewRelationship};
pub use hugr::NewHugr;
pub use jobs::{NewExecuteJobItem, NewJob, NewJobDefinition};
pub use projects::NewProject;
