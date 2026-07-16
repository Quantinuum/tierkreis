use hugr::{envelope::serde_with::AsBinaryEnvelope, package::Package};
use serde::Serialize;
use serde_with::serde_as;
use uuid::Uuid;

use crate::executor::nexus::client::models::NewRelationship;

#[derive(Serialize)]
struct NewHugrRelationships {
    project: NewRelationship,
}

impl NewHugrRelationships {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project: NewRelationship::new(project_id, "project"),
        }
    }
}

#[derive(Serialize)]
struct HugrProperties {}

#[serde_as]
#[derive(Serialize)]
struct NewHugrAttributes<'a> {
    name: &'a str,
    description: Option<&'a str>,
    properties: HugrProperties,
    #[serde_as(as = "AsBinaryEnvelope")]
    contents: Package,
}

#[derive(Serialize)]
struct NewHugrData<'a> {
    attributes: NewHugrAttributes<'a>,
    relationships: NewHugrRelationships,
    r#type: &'static str,
}

#[derive(Serialize)]
pub struct NewHugr<'a> {
    data: NewHugrData<'a>,
}

impl NewHugr<'_> {
    pub fn new<'a>(
        name: &'a str,
        description: Option<&'a str>,
        project_id: Uuid,
        package: Package,
    ) -> NewHugr<'a> {
        NewHugr {
            data: NewHugrData {
                attributes: NewHugrAttributes {
                    name,
                    description,
                    properties: HugrProperties {},
                    contents: package,
                },
                relationships: NewHugrRelationships::new(project_id),
                r#type: "hugr",
            },
        }
    }
}
