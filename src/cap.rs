use std::fmt;

#[derive(Debug, PartialEq)]
pub struct VcpCode {
    pub code: u8,
    pub values: Vec<u8>,
}

#[derive(Debug)]
pub struct Capabilities {
    pub vcp: Option<Vec<VcpCode>>,
}

#[derive(Debug, PartialEq)]
pub enum Input {
    DisplayPort1,
    DisplayPort2,
    Hdmi1,
    Hdmi2,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Input::DisplayPort1 => write!(f, "DisplayPort 1"),
            Input::DisplayPort2 => write!(f, "DisplayPort 2"),
            Input::Hdmi1 => write!(f, "HDMI 1"),
            Input::Hdmi2 => write!(f, "HDMI 2"),
        }
    }
}

impl Capabilities {
    pub fn supports_input_select(&self) -> bool {
        self.vcp.as_ref().is_some_and(|vcp_codes| {
            vcp_codes.iter().any(|vcp_code| vcp_code.code == 0x60)
        })
    }

    pub fn supported_inputs(&self) -> Vec<Input> {
        let mut inputs = Vec::new();

        if let Some(vcp_codes) = self.vcp.as_ref() {
            if let Some(vcp_code) =
                vcp_codes.iter().find(|vcp_code| vcp_code.code == 0x60)
            {
                for &value in vcp_code.values.iter() {
                    match value {
                        0x0F => inputs.push(Input::DisplayPort1),
                        0x10 => inputs.push(Input::DisplayPort2),
                        0x11 => inputs.push(Input::Hdmi1),
                        0x12 => inputs.push(Input::Hdmi2),
                        _ => (),
                    }
                }
            }
        }

        inputs
    }
}
