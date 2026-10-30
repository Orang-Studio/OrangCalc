using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;

namespace OrangCalc.Engine
{
    public enum CalendarKind { Gregorian, Hebrew, Islamic, Persian, Indian, Chinese, Julian, RevisedJulian, Coptic, Ethiopian }

    public static class Dates
    {
        public static DateTime Get(Expr e) => ((DateE)AsDate(e)).T;

        public static Expr AsDate(Expr e)
        {
            if (e is DateE) return e;
            if (e is StrE s)
            {
                var d = Funcs.ParseDate(s.S, out bool ht, out bool u);
                return new DateE(d, ht, u);
            }
            throw new CalcException("A date is required");
        }

        public static Expr AddSeconds(DateE d, Real seconds)
        {
            double s = seconds.ToDouble();
            var t = d.T.AddTicks((long)Math.Round(s * TimeSpan.TicksPerSecond));
            bool hasTime = d.HasTime || Math.Abs(s % 86400) > 1e-6;
            return new DateE(t, hasTime, d.Utc);
        }

        public static int IsoWeek(DateTime d)
        {
            var day = CultureInfo.InvariantCulture.Calendar.GetDayOfWeek(d);
            if (day >= DayOfWeek.Monday && day <= DayOfWeek.Wednesday) d = d.AddDays(3);
            return CultureInfo.InvariantCulture.Calendar.GetWeekOfYear(d, CalendarWeekRule.FirstFourDayWeek, DayOfWeek.Monday);
        }

        const double Synodic = 29.530588853;
        static readonly DateTime KnownNewMoon = new DateTime(2000, 1, 6, 18, 14, 0, DateTimeKind.Utc);

        public static double LunarPhase(DateTime d)
        {
            var days = (d.ToUniversalTime() - KnownNewMoon).TotalDays;
            var ph = (days / Synodic) % 1.0;
            if (ph < 0) ph += 1;
            return Math.Round(ph, 4);
        }

        public static DateTime NextLunarPhase(DateTime from, double phase)
        {
            var cur = LunarPhase(from);
            var delta = phase - cur;
            if (delta <= 0) delta += 1;
            return from.AddDays(delta * Synodic);
        }

        public static DateTime Easter(int y)
        {
            int a = y % 19, b = y / 100, c = y % 100, d = b / 4, e = b % 4, f = (b + 8) / 25, g = (b - f + 1) / 3;
            int h = (19 * a + b - d - g + 15) % 30, i = c / 4, k = c % 4, l = (32 + 2 * e + 2 * i - h - k) % 7;
            int m = (a + 11 * h + 22 * l) / 451;
            int month = (h + l - 7 * m + 114) / 31, day = (h + l - 7 * m + 114) % 31 + 1;
            return new DateTime(y, month, day);
        }

        public static Expr ToUtc(Expr e, TimeSpan offset)
        {
            if (!(e is DateE d)) throw new CalcException("UTC conversion requires a date");
            var utc = d.Utc ? d.T : d.T.ToUniversalTime();
            if (offset == TimeSpan.Zero) return new DateE(utc, true, true);
            var t = utc + offset;
            string sign = offset < TimeSpan.Zero ? "-" : "+";
            var ab = offset.Duration();
            return new StrE(t.ToString("yyyy-MM-dd'T'HH:mm:ss", CultureInfo.InvariantCulture) + sign + ab.Hours.ToString("00") + ":" + ab.Minutes.ToString("00"));
        }

        public static long ToJdn(DateTime g)
        {
            int a = (14 - g.Month) / 12, y = g.Year + 4800 - a, m = g.Month + 12 * a - 3;
            return g.Day + (153 * m + 2) / 5 + 365L * y + y / 4 - y / 100 + y / 400 - 32045;
        }

        public static DateTime FromJdn(long jdn)
        {
            long a = jdn + 32044, b = (4 * a + 3) / 146097, c = a - 146097 * b / 4;
            long d = (4 * c + 3) / 1461, e = c - 1461 * d / 4, m = (5 * e + 2) / 153;
            int day = (int)(e - (153 * m + 2) / 5 + 1), month = (int)(m + 3 - 12 * (m / 10)), year = (int)(100 * b + d - 4800 + m / 10);
            return new DateTime(year, month, day);
        }

        static long JulianToJdn(int y, int m, int d)
        {
            int a = (14 - m) / 12, yy = y + 4800 - a, mm = m + 12 * a - 3;
            return d + (153 * mm + 2) / 5 + 365L * yy + yy / 4 - 32083;
        }

        static void JdnToJulian(long jdn, out int y, out int m, out int d)
        {
            long c = jdn + 32082, dd = (4 * c + 3) / 1461, e = c - 1461 * dd / 4, mm = (5 * e + 2) / 153;
            d = (int)(e - (153 * mm + 2) / 5 + 1); m = (int)(mm + 3 - 12 * (mm / 10)); y = (int)(dd - 4800 + mm / 10);
        }

        static bool RjLeap(long y) => (y % 4 == 0 && y % 100 != 0) || (((y % 900) + 900) % 900 == 200 || ((y % 900) + 900) % 900 == 600);

        static long RjLeapsBefore(long y)
        {
            long n = y - 1;
            long c = FloorDiv(n, 4) - FloorDiv(n, 100) + FloorDiv(n - 200, 900) + 1 + FloorDiv(n - 600, 900) + 1;
            return c;
        }

        static long FloorDiv(long a, long b) => a >= 0 ? a / b : -((-a + b - 1) / b);

        static readonly int[] MonthDays = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
        static long rjOffset = long.MinValue;

        static long RjRaw(int y, int m, int d)
        {
            long days = 365L * (y - 1) + RjLeapsBefore(y);
            for (int i = 1; i < m; i++) days += MonthDays[i - 1] + (i == 2 && RjLeap(y) ? 1 : 0);
            return days + d;
        }

        static long RevisedJulianToJdn(int y, int m, int d)
        {
            if (rjOffset == long.MinValue) rjOffset = ToJdn(new DateTime(2000, 1, 1)) - RjRaw(2000, 1, 1);
            return RjRaw(y, m, d) + rjOffset;
        }

        static void JdnToRevisedJulian(long jdn, out int y, out int m, out int d)
        {
            var g = FromJdn(jdn);
            y = g.Year + 1;
            while (RevisedJulianToJdn(y, 1, 1) > jdn) y--;
            m = 12;
            while (RevisedJulianToJdn(y, m, 1) > jdn) m--;
            d = (int)(jdn - RevisedJulianToJdn(y, m, 1)) + 1;
        }

        static long CopticLike(long epoch, int y, int m, int d) => epoch - 1 + 365L * (y - 1) + y / 4 + 30 * (m - 1) + d;

        static void FromCopticLike(long epoch, long jdn, out int y, out int m, out int d)
        {
            y = (int)((4 * (jdn - epoch) + 1463) / 1461);
            m = (int)((jdn - CopticLike(epoch, y, 1, 1)) / 30) + 1;
            d = (int)(jdn + 1 - CopticLike(epoch, y, m, 1));
        }

        const long CopticEpoch = 1825030, EthiopianEpoch = 1724221;

        static void ToIndian(DateTime g, out int y, out int m, out int d)
        {
            int gy = g.Year;
            bool leap = DateTime.IsLeapYear(gy);
            var start = new DateTime(gy, 3, leap ? 21 : 22);
            int sy = gy - 78;
            if (g < start) { sy--; leap = DateTime.IsLeapYear(gy - 1); start = new DateTime(gy - 1, 3, leap ? 21 : 22); }
            int dayOfYear = (int)(g - start).TotalDays;
            int[] len = { leap ? 31 : 30, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30 };
            m = 0;
            while (dayOfYear >= len[m]) { dayOfYear -= len[m]; m++; }
            y = sy; m += 1; d = dayOfYear + 1;
        }

        static DateTime FromIndian(int y, int m, int d)
        {
            int gy = y + 78;
            bool leap = DateTime.IsLeapYear(gy);
            var start = new DateTime(gy, 3, leap ? 21 : 22);
            int[] len = { leap ? 31 : 30, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30 };
            int days = 0;
            for (int i = 0; i < m - 1; i++) days += len[i];
            return start.AddDays(days + d - 1);
        }

        public static readonly string[] CalendarNames = { "Gregorian", "Hebrew", "Islamic (Hijri)", "Persian (Solar Hijri)", "Indian (National)", "Chinese", "Julian", "Revised Julian (Milanković)", "Coptic", "Ethiopian" };

        public static string[] MonthNames(CalendarKind k)
        {
            switch (k)
            {
                case CalendarKind.Hebrew: return new[] { "Tishrei", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar I", "Adar II", "Nisan", "Iyar", "Sivan", "Tammuz", "Av", "Elul" };
                case CalendarKind.Islamic: return new[] { "Muharram", "Safar", "Rabi' al-awwal", "Rabi' al-thani", "Jumada al-awwal", "Jumada al-thani", "Rajab", "Sha'ban", "Ramadan", "Shawwal", "Dhu al-Qi'dah", "Dhu al-Hijjah" };
                case CalendarKind.Persian: return new[] { "Farvardin", "Ordibehesht", "Khordad", "Tir", "Mordad", "Shahrivar", "Mehr", "Aban", "Azar", "Dey", "Bahman", "Esfand" };
                case CalendarKind.Indian: return new[] { "Chaitra", "Vaishakha", "Jyeshtha", "Ashadha", "Shravana", "Bhadra", "Ashvin", "Kartika", "Agrahayana", "Pausha", "Magha", "Phalguna" };
                case CalendarKind.Coptic: return new[] { "Thout", "Paopi", "Hathor", "Koiak", "Tobi", "Meshir", "Paremhat", "Parmouti", "Pashons", "Paoni", "Epip", "Mesori", "Pi Kogi Enavot" };
                case CalendarKind.Ethiopian: return new[] { "Meskerem", "Tikimt", "Hidar", "Tahsas", "Tir", "Yekatit", "Megabit", "Miazia", "Genbot", "Sene", "Hamle", "Nehase", "Pagume" };
                case CalendarKind.Chinese: return Enumerable.Range(1, 13).Select(i => "Month " + i).ToArray();
                default: return CultureInfo.InvariantCulture.DateTimeFormat.MonthNames.Where(s => s.Length > 0).ToArray();
            }
        }

        public static DateTime ToGregorian(CalendarKind k, int y, int m, int d)
        {
            switch (k)
            {
                case CalendarKind.Gregorian: return new DateTime(y, m, d);
                case CalendarKind.Hebrew: return new HebrewCalendar().ToDateTime(y, m, d, 0, 0, 0, 0);
                case CalendarKind.Islamic: return new HijriCalendar().ToDateTime(y, m, d, 0, 0, 0, 0);
                case CalendarKind.Persian: return new PersianCalendar().ToDateTime(y, m, d, 0, 0, 0, 0);
                case CalendarKind.Chinese: return new ChineseLunisolarCalendar().ToDateTime(y, m, d, 0, 0, 0, 0);
                case CalendarKind.Indian: return FromIndian(y, m, d);
                case CalendarKind.Julian: return FromJdn(JulianToJdn(y, m, d));
                case CalendarKind.RevisedJulian: return FromJdn(RevisedJulianToJdn(y, m, d));
                case CalendarKind.Coptic: return FromJdn(CopticLike(CopticEpoch, y, m, d));
                case CalendarKind.Ethiopian: return FromJdn(CopticLike(EthiopianEpoch, y, m, d));
            }
            throw new CalcException("Unknown calendar");
        }

        public static void FromGregorian(CalendarKind k, DateTime g, out int y, out int m, out int d)
        {
            long jdn = ToJdn(g);
            switch (k)
            {
                case CalendarKind.Gregorian: y = g.Year; m = g.Month; d = g.Day; return;
                case CalendarKind.Hebrew: { var c = new HebrewCalendar(); y = c.GetYear(g); m = c.GetMonth(g); d = c.GetDayOfMonth(g); return; }
                case CalendarKind.Islamic: { var c = new HijriCalendar(); y = c.GetYear(g); m = c.GetMonth(g); d = c.GetDayOfMonth(g); return; }
                case CalendarKind.Persian: { var c = new PersianCalendar(); y = c.GetYear(g); m = c.GetMonth(g); d = c.GetDayOfMonth(g); return; }
                case CalendarKind.Chinese: { var c = new ChineseLunisolarCalendar(); y = c.GetYear(g); m = c.GetMonth(g); d = c.GetDayOfMonth(g); return; }
                case CalendarKind.Indian: ToIndian(g, out y, out m, out d); return;
                case CalendarKind.Julian: JdnToJulian(jdn, out y, out m, out d); return;
                case CalendarKind.RevisedJulian: JdnToRevisedJulian(jdn, out y, out m, out d); return;
                case CalendarKind.Coptic: FromCopticLike(CopticEpoch, jdn, out y, out m, out d); return;
                case CalendarKind.Ethiopian: FromCopticLike(EthiopianEpoch, jdn, out y, out m, out d); return;
            }
            throw new CalcException("Unknown calendar");
        }

        static readonly string[] Stems = { "Jiă", "Yĭ", "Bĭng", "Dīng", "Wù", "Jĭ", "Gēng", "Xīn", "Rén", "Guĭ" };
        static readonly string[] Branches = { "Zĭ (Rat)", "Chŏu (Ox)", "Yín (Tiger)", "Măo (Rabbit)", "Chén (Dragon)", "Sì (Snake)", "Wŭ (Horse)", "Wèi (Goat)", "Shēn (Monkey)", "Yŏu (Rooster)", "Xū (Dog)", "Hài (Pig)" };

        public static string Describe(CalendarKind k, DateTime g)
        {
            try
            {
                FromGregorian(k, g, out int y, out int m, out int d);
                if (k == CalendarKind.Chinese)
                {
                    var c = new ChineseLunisolarCalendar();
                    int sex = c.GetSexagenaryYear(g);
                    int stem = c.GetCelestialStem(sex) - 1, branch = c.GetTerrestrialBranch(sex) - 1;
                    int leap = c.GetLeapMonth(y);
                    string month = leap > 0 && m >= leap ? (m == leap ? "Leap month " + (m - 1) : "Month " + (m - 1)) : "Month " + m;
                    return Stems[stem] + " " + Branches[branch] + ", " + month + ", day " + d;
                }
                var names = MonthNames(k);
                string mn = m - 1 < names.Length ? names[m - 1] : m.ToString();
                if (k == CalendarKind.Hebrew)
                {
                    var hc = new HebrewCalendar();
                    if (!hc.IsLeapYear(y))
                    {
                        string[] nl = { "Tishrei", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar", "Nisan", "Iyar", "Sivan", "Tammuz", "Av", "Elul" };
                        mn = nl[m - 1];
                    }
                }
                return d + " " + mn + " " + y;
            }
            catch (ArgumentOutOfRangeException) { return "out of range"; }
            catch (ArgumentException) { return "out of range"; }
        }

        public static Expr CalendarsText(Expr e)
        {
            var d = Get(e);
            var sb = new StringBuilder();
            for (int i = 0; i < CalendarNames.Length; i++)
            {
                if (i > 0) sb.Append("\n");
                sb.Append(CalendarNames[i]).Append(": ").Append(Describe((CalendarKind)i, d.Date));
            }
            return new StrE(sb.ToString());
        }
    }
}
