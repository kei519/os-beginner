use core::fmt::{self, Display};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub enum Code {
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotID,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
    NoPCIMSI,
    UnknownPixelFormat,
    NoSuchTask,
    InvalidFormat,
    FrameTooSmall,
    InvalidFile,
    IsDirectory,
    NoSuchEntry,
    FreeTypeError,
    EndpointNotInCharge,
}

impl Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::Empty => write!(f, "Empty"),
            Self::NoEnoughMemory => write!(f, "NoEnoughMemory"),
            Self::IndexOutOfRange => write!(f, "IndexOutOfRange"),
            Self::HostControllerNotHalted => write!(f, "HostControllerNotHalted"),
            Self::InvalidSlotID => write!(f, "InvalidSlotID"),
            Self::PortNotConnected => write!(f, "PortNotConnected"),
            Self::InvalidEndpointNumber => write!(f, "InvalidEndpointNumber"),
            Self::TransferRingNotSet => write!(f, "TransferRingNotSet"),
            Self::AlreadyAllocated => write!(f, "AlreadyAllocated"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::InvalidDescriptor => write!(f, "InvalidDescriptor"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
            Self::UnknownDevice => write!(f, "UnknownDevice"),
            Self::NoCorrespondingSetupStage => write!(f, "NoCorrespondingSetupStage"),
            Self::TransferFailed => write!(f, "TransferFailed"),
            Self::InvalidPhase => write!(f, "InvalidPhase"),
            Self::UnknownXHCISpeedID => write!(f, "UnknownXHCISpeedID"),
            Self::NoWaiter => write!(f, "NoWaiter"),
            Self::NoPCIMSI => write!(f, "NoPCIMSI"),
            Self::UnknownPixelFormat => write!(f, "UnknownPixelFormat"),
            Self::NoSuchTask => write!(f, "NoSuchTask"),
            Self::InvalidFormat => write!(f, "InvalidFormat"),
            Self::FrameTooSmall => write!(f, "FrameTooSmall"),
            Self::InvalidFile => write!(f, "InvalidFile"),
            Self::IsDirectory => write!(f, "IsDirectory"),
            Self::NoSuchEntry => write!(f, "NoSuchEntry"),
            Self::FreeTypeError => write!(f, "FreeTypeError"),
            Self::EndpointNotInCharge => write!(f, "EndpointNotInCharge"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Error {
    code: Code,
    msg: &'static str,
    line: u32,
    file: &'static str,
}

impl Error {
    pub const fn new(code: Code, msg: &'static str, file: &'static str, line: u32) -> Self {
        Self {
            code,
            msg,
            line,
            file,
        }
    }

    pub const fn cause(&self) -> Code {
        self.code
    }

    pub const fn file(&self) -> &str {
        self.file
    }

    pub const fn line(&self) -> u32 {
        self.line
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in {} at {}", self.code, self.file, self.line)?;
        if !self.msg.is_empty() {
            write!(f, ":\n    {}", self.msg)?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! make_error {
    ($code:expr, $msg:expr) => {
        $crate::error::Error::new($code, $msg, file!(), line!())
    };
    ($code:expr) => {
        make_error!($code, "")
    };
}
