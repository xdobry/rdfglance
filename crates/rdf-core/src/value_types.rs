bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ValueTypes: u32 {
        const STRING       = 0b1;
        const SHORT_STRING = 0b10;
        const LANG_STRING  = 0b100;
        const INTEGER      = 0b1000;
        const DOUBLE       = 0b10000;
        const XML          = 0b100000;
        const JSON         = 0b1000000;
        const DATE_TIME    = 0b10000000;
        const DATE         = 0b100000000;
        const TIME         = 0b1000000000;
        const DURATION     = 0b10000000000;
        const BOOLEAN      = 0b100000000000;
        const UNKNOWN      = 0b1000000000000;
    }
}
