use crate::cap::{Capabilities, Input};

pub trait Monitor {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &Capabilities;
    fn input(&self) -> anyhow::Result<Input>;
    fn set_input(&mut self, input: Input) -> anyhow::Result<()>;
}
