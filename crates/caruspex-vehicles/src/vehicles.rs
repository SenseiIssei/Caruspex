//! The bus a car talks on at its OBD socket.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BusKind {
    /// ISO 9141-2 / ISO 14230, older European and Japanese cars.
    KLine,
    /// ISO 15765-4, mandatory in Europe from about 2004.
    Can,
    /// SAE J1850 PWM, older Ford USA.
    Pwm,
    /// SAE J1850 VPW, older GM USA.
    Vpw,
    /// Transition period, either is possible.
    #[default]
    Mixed,
}
