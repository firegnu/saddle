use crate::drover::Task;
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Target {
    /// A task by project and public identity: its id, or its text when unnumbered.
    Task {
        project: String,
        id: Option<String>,
        title: String,
        body: String,
    },
    /// A project whose read failed; opening shows its Tasks.
    Project(String),
    Source(String),
}

impl Target {
    pub(crate) fn task(project: &str, task: &Task) -> Self {
        Self::Task {
            project: project.into(),
            id: task.id.clone(),
            title: task.title.clone(),
            body: task.body.clone(),
        }
    }
}
pub fn project_name(path: &str) -> &str {
    let path = path.trim_end_matches('/');
    path.rsplit('/').next().unwrap_or(path)
}
