use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Code {
    Success = 0,
    InvalidCli = 2,
    InvalidInput = 3,
    ParityFail = 4,
    TargetFail = 5,
    RunFail = 6,
    AggregateFail = 7,
    PublishFail = 8,
    GateFail = 9,
    NoBaseline = 10,
    Canceled = 11,
}

impl From<Code> for ExitCode {
    fn from(value: Code) -> Self {
        ExitCode::from(value as u8)
    }
}

#[derive(Debug)]
pub struct Fail {
    pub code: Code,
    pub msg: String,
}

impl Fail {
    pub fn new(code: Code, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
        }
    }
}
