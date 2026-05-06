// Contains model semantic definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MomentHeader {
    pub time_resolution_level: u8,
    pub date_range_level: u8,
    pub zone_level: u8,
    pub accuracy: Accuracy,
    pub leap_counter_length: u8,
    pub has_uncertainty: bool,
    pub lsl_status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    pub header: MomentHeader,
    pub time_value: Option<u128>, // u128 is also limitation on payload sizes, future work
    pub date_value: u128, // time value optional, date must exist for moment, hence differing types
    pub zone_value: Option<u128>,

    pub positive_leap_seconds: Option<u128>,
    pub negative_leap_seconds: Option<u128>,

    pub uncertainty_offset: Option<u128>,
    pub lsl_jdn: Option<u64>,
}

// Period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Positive,
    Negative,
}

impl Sign {
    pub fn to_binary(self) -> u8 {
        match self {
            Sign::Positive => 0,
            Sign::Negative => 1,
        }
    }

    pub fn from_binary(bit: u8) -> Option<Self> {
        match bit {
            0 => Some(Sign::Positive),
            1 => Some(Sign::Negative),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodHeader {
    pub sign: Sign,
    pub time_resolution_level: u8,
    pub date_range_level: u8,
    pub leap_counter_length: u8,
    pub has_uncertainty: bool,
    pub lsl_status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub header: PeriodHeader,
    pub time_duration: Option<u128>,
    pub date_duration: u128,

    pub positive_leap_seconds: Option<u128>,
    pub negative_leap_seconds: Option<u128>,

    pub uncertainty_offset: Option<u128>,
    pub lsl_jdn: Option<u64>,
}
