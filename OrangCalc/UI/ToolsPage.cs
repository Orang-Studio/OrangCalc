using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Windows.Forms;
using System.Security.Cryptography;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class ToolsPage : Panel, IPage
    {
        readonly MainForm main;
        readonly SegmentBar tabs = new SegmentBar { Dock = DockStyle.Top, Compact = true };
        readonly Panel body = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window, Padding = new Padding(0, 14, 0, 0) };
        readonly Control[] views;

        public ToolsPage(MainForm m)
        {
            main = m;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);
            var header = Ui.Header("Tools");
            header.Dock = DockStyle.Top;
            tabs.Items.AddRange(new[] { "Percentage Calculation", "Calendar Conversion", "Floating Point Conversion (IEEE 754)", "Password Generator" });
            tabs.SelectedChanged += (s, e) => Show(tabs.SelectedIndex);
            views = new Control[] { BuildPercentage(), BuildCalendar(), BuildFloat(), BuildPasswords() };
            Controls.Add(body);
            Controls.Add(tabs);
            Controls.Add(header);
            Show(0);
        }

        public void Select(int i) { tabs.SelectedIndex = i; Show(i); }

        void Show(int i)
        {
            body.Controls.Clear();
            body.Controls.Add(views[i]);
        }

        static TableLayoutPanel Form(int labelWidth = 220)
        {
            var t = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 3, BackColor = Theme.Window, Padding = new Padding(4) };
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, labelWidth));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 320));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            return t;
        }

        static TextBox Field(TableLayoutPanel t, string label, bool readOnly = false)
        {
            int r = t.RowCount++;
            t.Controls.Add(new Label { Text = label, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, r);
            var b = Ui.Box("", 300);
            b.ReadOnly = readOnly;
            if (readOnly) b.BackColor = Theme.Panel;
            b.Margin = new Padding(0, 4, 8, 4);
            b.Font = Theme.MonoFont;
            t.Controls.Add(b, 1, r);
            return b;
        }

        double? Value(string s)
        {
            s = s.Trim();
            if (s.Length == 0) return null;
            try
            {
                var e = main.Svc.Run(() => main.Calc.EvalText(s.TrimEnd('%'))).Result;
                if (e is NumE n && n.V.IsReal) return n.V.Re.ToDouble();
            }
            catch (AggregateException) { }
            return double.NaN;
        }

        static string F(double v) => double.IsNaN(v) || double.IsInfinity(v) ? "-" : v.ToString("0.##########", CultureInfo.InvariantCulture);

        Control BuildPercentage()
        {
            var t = Form();
            var intro = new Label { Text = "Enter two values, of which at most one is a percentage, and the others will be calculated for you.", AutoSize = true, MaximumSize = new Size(700, 0), ForeColor = Theme.Text, Margin = new Padding(0, 0, 0, 10) };
            t.Controls.Add(intro, 0, t.RowCount++); t.SetColumnSpan(intro, 3);
            var v1 = Field(t, "Value 1");
            var v2 = Field(t, "Value 2");
            var pct = Field(t, "Change from 1 to 2 (%)");
            var sep = new Label { Height = 10, AutoSize = false };
            t.Controls.Add(sep, 0, t.RowCount++);
            var diff = Field(t, "Change from 1 to 2", true);
            var back = Field(t, "Change from 2 to 1 (%)", true);
            var cmp21 = Field(t, "2 compared to 1 (%)", true);
            var cmp12 = Field(t, "1 compared to 2 (%)", true);
            var pctOf = Field(t, "Value 1 is what % of value 2", true);
            var buttons = Ui.Row(new FlatButton("Calculate", null, true), new FlatButton("Clear", null));
            ((FlatButton)buttons.Controls[0]).Pressed += (s, e) => Calc();
            ((FlatButton)buttons.Controls[1]).Pressed += (s, e) => { foreach (var b in new[] { v1, v2, pct, diff, back, cmp21, cmp12, pctOf }) b.Text = ""; };
            buttons.Controls[1].Margin = new Padding(8, 0, 0, 0);
            buttons.Margin = new Padding(0, 12, 0, 0);
            t.Controls.Add(buttons, 1, t.RowCount++);
            foreach (var b in new[] { v1, v2, pct }) b.KeyDown += (s, e) => { if (e.KeyCode == Keys.Enter) { Calc(); e.SuppressKeyPress = true; } };

            void Calc()
            {
                double? a = Value(v1.Text), b = Value(v2.Text), p = Value(pct.Text);
                if (a.HasValue && b.HasValue) p = (b - a) / a * 100;
                else if (a.HasValue && p.HasValue) b = a * (1 + p / 100);
                else if (b.HasValue && p.HasValue) a = b / (1 + p / 100);
                else return;
                v1.Text = F(a.Value); v2.Text = F(b.Value); pct.Text = F(p.Value);
                diff.Text = F(b.Value - a.Value);
                back.Text = F((a.Value - b.Value) / b.Value * 100);
                cmp21.Text = F(b.Value / a.Value * 100);
                cmp12.Text = F(a.Value / b.Value * 100);
                pctOf.Text = F(a.Value / b.Value * 100);
            }
            var wrap = new Panel { Dock = DockStyle.Fill, AutoScroll = true, BackColor = Theme.Window };
            wrap.Controls.Add(t);
            return wrap;
        }

        Control BuildCalendar()
        {
            var t = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 5, BackColor = Theme.Window };
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 230));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 80));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 200));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 100));
            t.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            var rows = new List<(NumericUpDown d, DarkCombo m, NumericUpDown y, Label err)>();
            bool updating = false;
            for (int i = 0; i < Dates.CalendarNames.Length; i++)
            {
                int ci = i;
                var kind = (CalendarKind)i;
                t.Controls.Add(new Label { Text = Dates.CalendarNames[i], AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, i);
                var d = Ui.Number(1, 1, 31, 70);
                var mo = Ui.Combo(Dates.MonthNames(kind), 0, 190);
                var y = Ui.Number(2000, -9999, 99999, 90);
                var err = new Label { AutoSize = true, ForeColor = Theme.TextFaint, Margin = new Padding(6, 8, 0, 0) };
                t.Controls.Add(d, 1, i); t.Controls.Add(mo, 2, i); t.Controls.Add(y, 3, i); t.Controls.Add(err, 4, i);
                rows.Add((d, mo, y, err));
                EventHandler changed = (s, e) =>
                {
                    if (updating) return;
                    try
                    {
                        var g = Dates.ToGregorian(kind, (int)y.Value, mo.SelectedIndex + 1, (int)d.Value);
                        SetAll(g, ci);
                        err.Text = "";
                    }
                    catch (Exception) { err.Text = "Calendar conversion failed"; }
                };
                d.ValueChanged += changed; mo.SelectedIndexChanged += changed; y.ValueChanged += changed;
            }
            void SetAll(DateTime g, int except)
            {
                updating = true;
                try
                {
                    for (int i = 0; i < rows.Count; i++)
                    {
                        var (d, mo, y, err) = rows[i];
                        if (i == except) continue;
                        try
                        {
                            Dates.FromGregorian((CalendarKind)i, g, out int yy, out int mm, out int dd);
                            y.Value = Math.Max(y.Minimum, Math.Min(y.Maximum, yy));
                            if (mm - 1 < mo.Items.Count) mo.SelectedIndex = mm - 1;
                            d.Value = Math.Max(1, Math.Min(31, dd));
                            err.Text = (CalendarKind)i == CalendarKind.Chinese ? Dates.Describe(CalendarKind.Chinese, g).Split(',')[0] : "";
                        }
                        catch (Exception) { err.Text = "out of range"; }
                    }
                }
                finally { updating = false; }
                current = g;
            }
            var bar = Ui.Row(new FlatButton("Today", (s, e) => SetAll(DateTime.Today, -1)), new FlatButton("Insert date", (s, e) => main.InsertIntoExpression(current.ToString("yyyy-MM-dd")), true));
            bar.Controls[1].Margin = new Padding(8, 0, 0, 0);
            bar.Margin = new Padding(0, 14, 0, 0);
            t.Controls.Add(bar, 1, rows.Count); t.SetColumnSpan(bar, 3);
            SetAll(DateTime.Today, -1);
            var wrap = new Panel { Dock = DockStyle.Fill, AutoScroll = true, BackColor = Theme.Window };
            wrap.Controls.Add(t);
            return wrap;
        }

        DateTime current = DateTime.Today;

        Control BuildFloat()
        {
            var t = Form(220);
            int r0 = t.RowCount++;
            t.Controls.Add(new Label { Text = "Format", AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, r0);
            var fmt = Ui.Combo(FloatFormats.All.Select(f => f.Name), 1, 300);
            t.Controls.Add(fmt, 1, r0);
            var value = Field(t, "Value");
            var bin = Field(t, "Binary representation");
            var hex = Field(t, "Hexadecimal representation");
            var fpv = Field(t, "Floating point value", true);
            var err = Field(t, "Conversion error", true);
            var parts = Field(t, "Sign | exponent | significand", true);
            bin.Multiline = true; bin.Height = 44; bin.WordWrap = true;
            bool busy = false;
            FloatFormat F() => FloatFormats.All[Math.Max(0, fmt.SelectedIndex)];

            void FromBits(BigInteger bits)
            {
                var f = F();
                busy = true;
                try
                {
                    bin.Text = FloatFormats.ToBits(bits, f);
                    hex.Text = FloatFormats.ToHex(bits, f);
                    var s = FloatFormats.ToBits(bits, f);
                    parts.Text = f.Microchip ? s.Substring(0, f.ExpBits) + " | " + s.Substring(f.ExpBits, 1) + " | " + s.Substring(f.ExpBits + 1) : s.Substring(0, 1) + " | " + s.Substring(1, f.ExpBits) + " | " + s.Substring(1 + f.ExpBits);
                    try
                    {
                        var dv = FloatFormats.Decode(bits, f);
                        var o = main.Calc.Opts.Clone(); o.Precision = Math.Max(o.Precision, 40); o.Fraction = FractionMode.Decimal;
                        fpv.Text = Formatter.Format(new NumE(new Cx(dv)), o, out _);
                        return;
                    }
                    catch (CalcException ex) { fpv.Text = ex.Message; }
                }
                finally { busy = false; }
            }

            void FromValue()
            {
                if (busy) return;
                var text = value.Text.Trim();
                if (text.Length == 0) return;
                try
                {
                    var e = main.Svc.Run(() => main.Calc.EvalText(text)).Result;
                    if (!(e is NumE n) || !n.V.IsReal) { err.Text = "Conversion error"; return; }
                    var f = F();
                    var bits = FloatFormats.Encode(n.V.Re, f);
                    FromBits(bits);
                    var back = FloatFormats.Decode(bits, f);
                    var o = main.Calc.Opts.Clone(); o.Precision = Math.Max(o.Precision, 20);
                    err.Text = n.V.Re.IsInfinite ? "0" : Formatter.Format(new NumE(new Cx(back - n.V.Re)), o, out _);
                }
                catch (AggregateException ex) { err.Text = ex.InnerException?.Message ?? "Conversion error"; }
                catch (CalcException ex) { err.Text = ex.Message; }
            }

            value.TextChanged += (s, e) => FromValue();
            fmt.SelectedIndexChanged += (s, e) => FromValue();
            bin.TextChanged += (s, e) =>
            {
                if (busy) return;
                var b = bin.Text.Replace(" ", "").Replace("\r", "").Replace("\n", "");
                if (b.Length == 0 || b.Any(ch => ch != '0' && ch != '1')) return;
                BigInteger v = 0;
                foreach (char ch in b) v = v * 2 + (ch - '0');
                busy = true; value.Text = ""; busy = false;
                FromBits(v); err.Text = "";
            };
            hex.TextChanged += (s, e) =>
            {
                if (busy) return;
                var h = hex.Text.Trim();
                if (h.StartsWith("0x", StringComparison.OrdinalIgnoreCase)) h = h.Substring(2);
                if (h.Length == 0 || !h.All(Uri.IsHexDigit)) return;
                var v = BigInteger.Parse("0" + h, NumberStyles.HexNumber);
                busy = true; value.Text = ""; busy = false;
                FromBits(v); err.Text = "";
            };
            value.Text = "0.1";
            var wrap = new Panel { Dock = DockStyle.Fill, AutoScroll = true, BackColor = Theme.Window };
            wrap.Controls.Add(t);
            return wrap;
        }

        const string PwLower = "abcdefghijklmnopqrstuvwxyz";
        const string PwUpper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const string PwDigits = "0123456789";
        const string PwSymbols = "!@#$%^&*()-_=+[]{}<>?/;:,.";
        const string PwAmbiguous = "0Ooil1LI";

        Control BuildPasswords()
        {
            var t = Form(220);
            var intro = new Label { Text = "Generates random passwords from a cryptographically secure random source. Each password contains at least one character from every selected set.", AutoSize = true, MaximumSize = new Size(700, 0), ForeColor = Theme.Text, Margin = new Padding(0, 0, 0, 10) };
            t.Controls.Add(intro, 0, t.RowCount++); t.SetColumnSpan(intro, 3);
            int r1 = t.RowCount++;
            t.Controls.Add(new Label { Text = "Length", AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, r1);
            var len = Ui.Number(16, 4, 256, 90);
            t.Controls.Add(len, 1, r1);
            int r2 = t.RowCount++;
            t.Controls.Add(new Label { Text = "Quantity", AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, r2);
            var qty = Ui.Number(5, 1, 20, 90);
            t.Controls.Add(qty, 1, r2);
            int r3 = t.RowCount++;
            t.Controls.Add(new Label { Text = "Character sets", AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 8, 4) }, 0, r3);
            var sets = new FlowLayoutPanel { FlowDirection = FlowDirection.TopDown, AutoSize = true, WrapContents = false, Margin = new Padding(0, 4, 0, 0), BackColor = Color.Transparent };
            var lower = Ui.Check("Lowercase letters (a-z)", true);
            var upper = Ui.Check("Uppercase letters (A-Z)", true);
            var digits = Ui.Check("Digits (0-9)", true);
            var symbols = Ui.Check("Symbols (!@#$...)", true);
            var noAmb = Ui.Check("Exclude ambiguous characters (0, O, o, 1, l, I, ...)", true);
            sets.Controls.AddRange(new Control[] { lower, upper, digits, symbols, noAmb });
            t.Controls.Add(sets, 1, r3); t.SetColumnSpan(sets, 2);
            var entropy = new Label { AutoSize = true, ForeColor = Theme.TextFaint, Margin = new Padding(0, 0, 0, 0) };
            var output = Field(t, "Password(s)", true);
            output.Multiline = true;
            output.ScrollBars = ScrollBars.Both;
            output.WordWrap = false;
            output.Height = 96;
            output.Width = 420;
            t.SetColumnSpan(output, 2);
            var buttons = Ui.Row(new FlatButton("Generate", null, true), new FlatButton("Copy", null));
            ((FlatButton)buttons.Controls[0]).Pressed += (s, e) => Generate();
            ((FlatButton)buttons.Controls[1]).Pressed += (s, e) => { if (output.Text.Length > 0) Clipboard.SetText(output.Text); };
            buttons.Controls[1].Margin = new Padding(8, 0, 0, 0);
            buttons.Margin = new Padding(0, 12, 0, 0);
            t.Controls.Add(buttons, 1, t.RowCount++);
            entropy.Text = "";
            t.Controls.Add(entropy, 1, t.RowCount++);

            void Generate()
            {
                var chosen = new List<string>();
                if (lower.Checked) chosen.Add(PwLower);
                if (upper.Checked) chosen.Add(PwUpper);
                if (digits.Checked) chosen.Add(PwDigits);
                if (symbols.Checked) chosen.Add(PwSymbols);
                if (noAmb.Checked) chosen = chosen.Select(s => new string(s.Where(c => !PwAmbiguous.Contains(c)).ToArray())).Where(s => s.Length > 0).ToList();
                if (chosen.Count == 0) { entropy.Text = "Select at least one character set."; output.Text = ""; return; }
                var all = string.Concat(chosen);
                var lines = new List<string>();
                using (var rng = RandomNumberGenerator.Create())
                {
                    for (int k = 0; k < (int)qty.Value; k++)
                    {
                        var chars = new char[(int)len.Value];
                        int filled = 0;
                        foreach (var s in chosen)
                        {
                            if (filled >= chars.Length) break;
                            chars[filled++] = NextChar(rng, s);
                        }
                        for (int i = filled; i < chars.Length; i++) chars[i] = NextChar(rng, all);
                        for (int i = chars.Length - 1; i > 0; i--)
                        {
                            int j = NextIndex(rng, (uint)(i + 1));
                            char tmp = chars[i]; chars[i] = chars[j]; chars[j] = tmp;
                        }
                        lines.Add(new string(chars));
                    }
                }
                output.Text = string.Join("\r\n", lines);
                entropy.Text = all.Length + " possible characters, ~" + (Math.Log(all.Length, 2) * (int)len.Value).ToString("0.#") + " bits of entropy per password";
            }

            var wrap = new Panel { Dock = DockStyle.Fill, AutoScroll = true, BackColor = Theme.Window };
            wrap.Controls.Add(t);
            return wrap;
        }

        static char NextChar(RandomNumberGenerator rng, string set) => set[NextIndex(rng, (uint)set.Length)];

        static int NextIndex(RandomNumberGenerator rng, uint n)
        {
            if (n <= 1) return 0;
            ulong threshold = (1UL << 32) / n * n;
            var b = new byte[4];
            uint v;
            do { rng.GetBytes(b); v = BitConverter.ToUInt32(b, 0); } while (v >= threshold);
            return (int)(v % n);
        }

        public void Activated() { }
        public void DefinitionsChanged() { }
        public void OptionsChanged() { }
    }
}
