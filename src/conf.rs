use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Child {
    pub pid: u32,
    pub socket: u16,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Conf {
    pub children: Vec<Child>,
}
