use serde_derive::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Child {
    pub pid: u32,
    pub socket: u16,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Conf {
    pub children: Vec<Child>,
}
