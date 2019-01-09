use crate::spec::Object;

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct VerifyRequest {
    #[serde(flatten)]
    pub object: Object,
}
