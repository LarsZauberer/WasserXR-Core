use wasserxr::Uuid;

#[derive(Default)]
pub struct ConsoleData {
    entities: Vec<(Uuid, String)>,
}

impl ConsoleData {
    pub(crate) fn new(entities: Vec<(Uuid, String)>) -> Self {
        Self { entities }
    }
}
