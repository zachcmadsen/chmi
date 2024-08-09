#[derive(Debug)]
pub struct VcpCode {
    pub code: u8,
    pub values: Vec<u8>,
}

#[derive(Debug)]
pub struct Capabilities {
    pub vcp: Option<Vec<VcpCode>>,
}
