use serde::Serialize;

#[derive(Serialize)]
struct ProjectRelationships {}

#[derive(Serialize)]
struct ProjectProperties {}

#[derive(Serialize)]
struct NewProjectAttributes<'a> {
    name: &'a str,
    description: Option<&'a str>,
    properties: ProjectProperties,
}

#[derive(Serialize)]
struct NewProjectData<'a> {
    attributes: NewProjectAttributes<'a>,
    relationships: ProjectRelationships,
    r#type: &'static str,
}

#[derive(Serialize)]
pub struct NewProject<'a> {
    data: NewProjectData<'a>,
}

impl NewProject<'_> {
    pub fn new<'a>(name: &'a str, description: Option<&'a str>) -> NewProject<'a> {
        NewProject {
            data: NewProjectData {
                attributes: NewProjectAttributes {
                    name,
                    description,
                    properties: ProjectProperties {},
                },
                relationships: ProjectRelationships {},
                r#type: "project",
            },
        }
    }
}
