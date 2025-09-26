use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subject {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Filter {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub filter_type: String,
    pub list: Vec<Subject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelResponse {
    pub errno: i32,
    pub errmsg: String,
    pub data: LabelData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelData {
    pub filter: Vec<Filter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskItem {
    #[serde(rename = "taskID")]
    pub task_id: i32,
    #[serde(rename = "clueID")]
    pub clue_id: i32,
    pub brief: String,
    pub step: i32,
    pub subject: i32,
    pub state: i32,
    #[serde(rename = "stepName")]
    pub step_name: String,
    #[serde(rename = "subjectName")]
    pub subject_name: String,
    #[serde(rename = "clueType")]
    pub clue_type: i32,
    #[serde(rename = "clueTypeName")]
    pub clue_type_name: String,
    #[serde(rename = "stateName")]
    pub state_name: String,
    #[serde(rename = "createTime")]
    pub create_time: String,
    #[serde(rename = "dispatchTime", default)]
    pub dispatch_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListData {
    pub total: i32,
    pub list: Vec<TaskItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListResponse {
    pub errno: i32,
    pub errmsg: String,
    pub data: TaskListData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaimResponse {
    pub errno: i32,
    pub errmsg: String,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoData {
    #[serde(rename = "roleLinks")]
    pub role_links: Vec<String>,
    #[serde(rename = "roleNames")]
    pub role_names: Vec<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub avatar: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub errno: i32,
    pub errmsg: String,
    pub data: UserInfoData,
}
