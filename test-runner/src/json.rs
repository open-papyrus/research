use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TestDefinition {
    pub description: String,
    pub tests: Vec<Test>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Test {
    pub name: String,
    pub description: String,
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScriptResult {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Script {
    pub file: String,

    #[serde(rename = "expectedResult")]
    pub expected_result: ScriptResult,
}
