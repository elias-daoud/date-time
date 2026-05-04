// Contains model semantic definitions
#[derive(Debug, Clone, Copy)]
pub enum Accuracy {
    Start,
    Whole,
    End,
}

impl Accuracy {
    pub fn to_binary(self) -> u8 {
        match self {
            Accuracy::Start => 0b00,
            Accuracy::Whole => 0b01,
            Accuracy::End => 0b10,
        }
    }

    pub fn from_binary(bits: u8) -> Option<Self> {
        match bits {
            0b00 => Some(Accuracy::Start),
            0b01 => Some(Accuracy::Whole),
            0b10 => Some(Accuracy::End),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct MomentHeader {
    pub time_resolution_level: u8,
    pub date_range_level: u8,
    pub zone_level: u8,
    pub accuracy: Accuracy,
    pub leap_counter_length: u8,
    pub has_uncertainty: bool,
    pub lsl_status: u8,
}
