use super::RoundingMode;
macro_rules! indexed_enum {
    ($name:ident { $($v:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($v),* }
        #[allow(dead_code)]
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v),*];
            pub fn index(self) -> usize { Self::ALL.iter().position(|x| *x == self).unwrap() }
            pub fn from_index(i: usize) -> $name { Self::ALL[i.min(Self::ALL.len() - 1)] }
            pub fn name(self) -> &'static str { match self { $($name::$v => stringify!($v)),* } }
            pub fn parse(s: &str) -> Option<$name> { Self::ALL.iter().copied().find(|x| x.name() == s) }
        }
    };
}

indexed_enum!(ApproxMode { Auto, Dual, Exact, Approximate });
indexed_enum!(AngleMode { Radians, Degrees, Gradians, None });
indexed_enum!(ParseMode { Conventional, ImplicitFirst, Adaptive, Chain, Rpn });
indexed_enum!(StructMode { Simplify, Factorize, Expand, None });
indexed_enum!(NumberDisplay { Normal, Scientific, Engineering, Simple, PureScientific });
indexed_enum!(FractionMode { Decimal, DecimalExact, Fraction, Combined, Percent, Permille, Permyriad });
indexed_enum!(ComplexForm { Rectangular, Exponential, Polar, Cis, Angle });
indexed_enum!(IntervalDisplay { Adaptive, SignificantDigits, Interval, PlusMinus, Midpoint, Lower, Upper, Concise, Relative });
indexed_enum!(IntervalCalc { VarianceFormula, IntervalArithmetic, Simple });
indexed_enum!(DigitGrouping { None, Standard, Locale });
indexed_enum!(RepeatingMode { None, Ellipsis, Overline });
indexed_enum!(UnitConversion { None, OptimalSI, Optimal, Base });
indexed_enum!(PrefixMode { None, Some, All, AllWithCurrencies });
indexed_enum!(TemperatureMode { Hybrid, Absolute, Relative });
indexed_enum!(AssumeType { Number, Real, Rational, Integer, Boolean });
indexed_enum!(AssumeSign { Unknown, NonZero, Positive, NonNegative, Negative, NonPositive });
indexed_enum!(MultSign { Dot, Times, Asterisk, MiddleDot });
indexed_enum!(DivSign { Slash, DivisionSlash, Division });
indexed_enum!(TwosComplement { Off, Binary, BinaryHex });

impl RoundingMode {
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|x| *x == self).unwrap()
    }
    pub fn from_index(i: usize) -> RoundingMode {
        Self::ALL[i.min(Self::ALL.len() - 1)]
    }
}

// special output bases beyond plain integers
pub mod bases {
    pub const ROMAN: i32 = -1000;
    pub const TIME: i32 = -1001;
    pub const SEXAGESIMAL: i32 = -1002;
    pub const BIJECTIVE26: i32 = -1003;
    pub const BCD: i32 = -1004;
    pub const UNICODE: i32 = -1005;
    pub const GOLDEN: i32 = -1006;
    pub const SUPERGOLDEN: i32 = -1007;
    pub const PI_BASE: i32 = -1008;
    pub const E_BASE: i32 = -1009;
    pub const SQRT2: i32 = -1010;
    pub const FP32: i32 = -1011;
    pub const FP64: i32 = -1012;
    pub const FP16: i32 = -1013;
    pub const FP80: i32 = -1014;
    pub const FP128: i32 = -1015;
    pub const LATITUDE: i32 = -1016;
    pub const LONGITUDE: i32 = -1017;
    pub const CUSTOM: i32 = -1018;
}

#[derive(Clone, Debug)]
pub struct CalcOptions {
    pub approx: ApproxMode,
    pub angle: AngleMode,
    pub parse: ParseMode,
    pub structure: StructMode,
    pub units_enabled: bool,
    pub variables_enabled: bool,
    pub functions_enabled: bool,
    pub units_in_constants: bool,
    pub unknown_as_variables: bool,
    pub complex_enabled: bool,
    pub infinite_enabled: bool,
    pub real_roots: bool,
    pub read_precision: bool,
    pub limit_implicit_mult: bool,
    pub caret_xor: bool,
    pub simplified_percent: bool,
    pub concise_uncertainty_input: bool,
    pub ignore_comma: bool,
    pub ignore_dots: bool,
    pub comma_decimal: bool,
    pub use_j: bool,
    pub interval: IntervalCalc,
    pub temperature: TemperatureMode,
    pub auto_convert: UnitConversion,
    pub prefixes: PrefixMode,
    pub denominator_prefixes: bool,
    pub all_si_prefixes: bool,
    pub currency_prefixes: bool,
    pub binary_prefixes: bool,
    pub mixed_units: bool,
    pub default_type: AssumeType,
    pub default_sign: AssumeSign,
    pub input_base: i32,
    pub input_base_custom: String,
    pub twos_complement_input: bool,
    pub hex_twos_complement_input: bool,
    pub precision: i32,
    pub rounding: RoundingMode,

    pub display: NumberDisplay,
    pub output_base: i32,
    pub output_base_custom: String,
    pub min_decimals: i32,
    pub max_decimals: i32,
    pub fraction: FractionMode,
    pub complex: ComplexForm,
    pub interval_out: IntervalDisplay,
    pub grouping: DigitGrouping,
    pub repeating: RepeatingMode,
    pub e_notation: bool,
    pub lower_case_e: bool,
    pub lower_case_hex: bool,
    pub duodecimal_symbols: bool,
    pub unicode: bool,
    pub mult: MultSign,
    pub div: DivSign,
    pub unicode_division: bool,
    pub spell_out_logical: bool,
    pub abbreviate_names: bool,
    pub twos_out: TwosComplement,
    pub binary_bits: i32,
    pub binary_letter_spacing: i32,
    pub implicit_mult_output: bool,
    pub excessive_parentheses: bool,
    pub show_ending_zeroes: bool,
}

impl Default for CalcOptions {
    fn default() -> Self {
        CalcOptions {
            approx: ApproxMode::Auto,
            angle: AngleMode::Radians,
            parse: ParseMode::Adaptive,
            structure: StructMode::Simplify,
            units_enabled: true,
            variables_enabled: true,
            functions_enabled: true,
            units_in_constants: true,
            unknown_as_variables: true,
            complex_enabled: true,
            infinite_enabled: true,
            real_roots: true,
            read_precision: false,
            limit_implicit_mult: false,
            caret_xor: false,
            simplified_percent: true,
            concise_uncertainty_input: false,
            ignore_comma: false,
            ignore_dots: false,
            comma_decimal: false,
            use_j: false,
            interval: IntervalCalc::VarianceFormula,
            temperature: TemperatureMode::Hybrid,
            auto_convert: UnitConversion::Optimal,
            prefixes: PrefixMode::Some,
            denominator_prefixes: false,
            all_si_prefixes: false,
            currency_prefixes: false,
            binary_prefixes: false,
            mixed_units: true,
            default_type: AssumeType::Real,
            default_sign: AssumeSign::Unknown,
            input_base: 10,
            input_base_custom: String::new(),
            twos_complement_input: false,
            hex_twos_complement_input: false,
            precision: 10,
            rounding: RoundingMode::HalfAwayFromZero,
            display: NumberDisplay::Normal,
            output_base: 10,
            output_base_custom: String::new(),
            min_decimals: 0,
            max_decimals: -1,
            fraction: FractionMode::DecimalExact,
            complex: ComplexForm::Rectangular,
            interval_out: IntervalDisplay::Adaptive,
            grouping: DigitGrouping::None,
            repeating: RepeatingMode::None,
            e_notation: false,
            lower_case_e: false,
            lower_case_hex: false,
            duodecimal_symbols: false,
            unicode: true,
            mult: MultSign::Times,
            div: DivSign::Slash,
            unicode_division: false,
            spell_out_logical: true,
            abbreviate_names: true,
            twos_out: TwosComplement::Binary,
            binary_bits: 0,
            binary_letter_spacing: 4,
            implicit_mult_output: true,
            excessive_parentheses: false,
            show_ending_zeroes: false,
        }
    }
}