#[derive(Debug)]
pub enum EncodeError {
    DateRangeLevelTooLarge,
    TimeResolutionTooLarge,
    ZoneLevelTooLarge,
    AccuracyTooLarge,
    LeapCounterLengthTooLarge,
    LslStatusTooLarge,
    InvalidLslStatus,
}

#[derive(Debug)]
pub enum DecodeError {
    EmptyInput,
    UnexpectedEnd,
    NotMomentHeader,
    InvalidAccuracyBits,
    NonZeroReservedBits,
    InvalidHeaderCategory,
    InvalidLslStatus,
    CoreHeaderAfterExtension,
    UnsupportedAdditionalExtensionHeaders,
}
