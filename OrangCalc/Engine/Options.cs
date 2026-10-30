namespace OrangCalc.Engine
{
    public enum ApproxMode { Auto, Dual, Exact, Approximate }
    public enum AngleMode { Radians, Degrees, Gradians, None }
    public enum ParseMode { Conventional, ImplicitFirst, Adaptive, Chain, Rpn }
    public enum StructMode { Simplify, Factorize, Expand, None }
    public enum NumberDisplay { Normal, Scientific, Engineering, Simple, PureScientific }
    public enum FractionMode { Decimal, DecimalExact, Fraction, Combined, Percent, Permille, Permyriad }
    public enum ComplexForm { Rectangular, Exponential, Polar, Cis, Angle }
    public enum IntervalDisplay { Adaptive, SignificantDigits, Interval, PlusMinus, Midpoint, Lower, Upper, Concise, Relative }
    public enum IntervalCalc { VarianceFormula, IntervalArithmetic, Simple }
    public enum DigitGrouping { None, Standard, Locale }
    public enum RepeatingMode { None, Ellipsis, Overline }
    public enum UnitConversion { None, OptimalSI, Optimal, Base }
    public enum PrefixMode { None, Some, All, AllWithCurrencies }
    public enum TemperatureMode { Hybrid, Absolute, Relative }
    public enum AssumeType { Number, Real, Rational, Integer, Boolean }
    public enum AssumeSign { Unknown, NonZero, Positive, NonNegative, Negative, NonPositive }
    public enum MultSign { Dot, Times, Asterisk, MiddleDot }
    public enum DivSign { Slash, DivisionSlash, Division }
    public enum TwosComplement { Off, Binary, BinaryHex }

    // special output bases beyond plain integers
    public static class Bases
    {
        public const int Roman = -1000;
        public const int Time = -1001;
        public const int Sexagesimal = -1002;
        public const int Bijective26 = -1003;
        public const int Bcd = -1004;
        public const int Unicode = -1005;
        public const int Golden = -1006;
        public const int Supergolden = -1007;
        public const int PiBase = -1008;
        public const int EBase = -1009;
        public const int Sqrt2 = -1010;
        public const int Fp32 = -1011;
        public const int Fp64 = -1012;
        public const int Fp16 = -1013;
        public const int Fp80 = -1014;
        public const int Fp128 = -1015;
        public const int Latitude = -1016;
        public const int Longitude = -1017;
        public const int Custom = -1018;
    }

    public sealed class CalcOptions
    {
        public ApproxMode Approx = ApproxMode.Auto;
        public AngleMode Angle = AngleMode.Radians;
        public ParseMode Parse = ParseMode.Adaptive;
        public StructMode Structure = StructMode.Simplify;
        public bool UnitsEnabled = true;
        public bool VariablesEnabled = true;
        public bool FunctionsEnabled = true;
        public bool UnitsInConstants = true;
        public bool UnknownAsVariables = true;
        public bool ComplexEnabled = true;
        public bool InfiniteEnabled = true;
        public bool RealRoots = true;
        public bool ReadPrecision = false;
        public bool LimitImplicitMult = false;
        public bool CaretXor = false;
        public bool SimplifiedPercent = true;
        public bool ConciseUncertaintyInput = false;
        public bool IgnoreComma = false;
        public bool IgnoreDots = false;
        public bool CommaDecimal = false;
        public bool UseJ = false;
        public IntervalCalc Interval = IntervalCalc.VarianceFormula;
        public TemperatureMode Temperature = TemperatureMode.Hybrid;
        public UnitConversion AutoConvert = UnitConversion.Optimal;
        public PrefixMode Prefixes = PrefixMode.Some;
        public bool DenominatorPrefixes = false;
        public bool AllSiPrefixes = false;
        public bool CurrencyPrefixes = false;
        public bool BinaryPrefixes = false;
        public bool MixedUnits = true;
        public AssumeType DefaultType = AssumeType.Real;
        public AssumeSign DefaultSign = AssumeSign.Unknown;
        public int InputBase = 10;
        public string InputBaseCustom = "";
        public bool TwosComplementInput = false;
        public bool HexTwosComplementInput = false;
        public int Precision = 10;
        public RoundingMode Rounding = RoundingMode.HalfAwayFromZero;

        public NumberDisplay Display = NumberDisplay.Normal;
        public int OutputBase = 10;
        public string OutputBaseCustom = "";
        public int MinDecimals = 0;
        public int MaxDecimals = -1;
        public FractionMode Fraction = FractionMode.DecimalExact;
        public ComplexForm Complex = ComplexForm.Rectangular;
        public IntervalDisplay IntervalOut = IntervalDisplay.Adaptive;
        public DigitGrouping Grouping = DigitGrouping.None;
        public RepeatingMode Repeating = RepeatingMode.None;
        public bool ENotation = false;
        public bool LowerCaseE = false;
        public bool LowerCaseHex = false;
        public bool DuodecimalSymbols = false;
        public bool Unicode = true;
        public MultSign Mult = MultSign.Times;
        public DivSign Div = DivSign.Slash;
        public bool UnicodeDivision = false;
        public bool SpellOutLogical = true;
        public bool AbbreviateNames = true;
        public TwosComplement TwosOut = TwosComplement.Binary;
        public int BinaryBits = 0;
        public int BinaryLetterSpacing = 4;
        public bool ImplicitMultOutput = true;
        public bool ExcessiveParentheses = false;
        public bool ShowEndingZeroes = false;

        public CalcOptions Clone() => (CalcOptions)MemberwiseClone();
    }
}
