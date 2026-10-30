using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class SettingsPage : Panel, IPage
    {
        readonly MainForm main;
        readonly NavRail sections = new NavRail { Dock = DockStyle.Left, Width = 190 };
        readonly Panel body = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window, AutoScroll = true, Padding = new Padding(24, 4, 12, 12) };
        TableLayoutPanel form;
        AppSettings S => main.Settings;
        CalcOptions O => main.Calc.Opts;

        static readonly string[] Names = { "Look & Feel", "Behavior", "Numbers & Operators", "Result Display", "Calculation", "Units & Currencies", "Keyboard Shortcuts", "Fonts" };

        public SettingsPage(MainForm m)
        {
            main = m;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);
            var header = Ui.Header("Settings");
            header.Dock = DockStyle.Top;
            sections.Items.AddRange(Names);
            sections.SelectedChanged += (s, e) => Build(sections.SelectedIndex);
            Theme.Style(body);
            Controls.Add(body);
            Controls.Add(sections);
            Controls.Add(header);
            Build(0);
        }

        public void ShowSection(int i) { sections.SelectedIndex = i; Build(i); }

        void Changed(bool options)
        {
            if (options) main.OptionsChanged();
            else main.MarkDirty();
        }

        void Begin(string title)
        {
            body.Controls.Clear();
            form = new TableLayoutPanel { Location = new Point(body.Padding.Left, body.Padding.Top), AutoSize = true, ColumnCount = 2, BackColor = Theme.Window };
            form.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            form.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            body.Controls.Add(form);
            var h = new Label { Text = title, AutoSize = true, Font = Theme.HeaderFont, ForeColor = Theme.Text, Margin = new Padding(0, 0, 0, 10), UseMnemonic = false };
            Add(h, true);
        }

        void Add(Control c, bool span = false, string label = null)
        {
            int r = form.RowCount++;
            if (label != null)
            {
                form.Controls.Add(new Label { Text = label, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 8, 10, 4) }, 0, r);
                c.Margin = new Padding(0, 4, 0, 4);
                form.Controls.Add(c, 1, r);
            }
            else
            {
                form.Controls.Add(c, 0, r);
                if (span) form.SetColumnSpan(c, 2);
            }
        }

        void Group(string text)
        {
            var l = new Label { Text = text, AutoSize = true, ForeColor = Theme.Accent, Font = Theme.UiBold, Margin = new Padding(0, 16, 0, 4) };
            Add(l, true);
        }

        void Note(string text) => Add(new Label { Text = text, AutoSize = true, MaximumSize = new Size(640, 0), ForeColor = Theme.TextFaint, Margin = new Padding(0, 0, 0, 6) }, true);

        void Check(string text, Func<bool> get, Action<bool> set, bool options = true)
        {
            var c = Ui.Check(text, get());
            c.CheckedChanged += (s, e) => { set(c.Checked); Changed(options); };
            Add(c, true);
        }

        void Choice(string label, string[] items, Func<int> get, Action<int> set, bool options = true, int width = 260)
        {
            var c = Ui.Combo(items, get(), width);
            c.SelectedIndexChanged += (s, e) => { set(c.SelectedIndex); Changed(options); };
            Add(c, false, label);
        }

        void EnumChoice<T>(string label, Func<T> get, Action<T> set, bool options = true, string[] names = null) where T : struct
        {
            var vals = Enum.GetValues(typeof(T)).Cast<T>().ToList();
            var items = names ?? vals.Select(v => ExpressionBox.Words(v.ToString())).ToArray();
            Choice(label, items, () => vals.IndexOf(get()), i => set(vals[i]), options);
        }

        void Num(string label, int min, int max, Func<int> get, Action<int> set, bool options = true)
        {
            var n = Ui.Number(get(), min, max, 90);
            n.ValueChanged += (s, e) => { set((int)n.Value); Changed(options); };
            Add(n, false, label);
        }

        void TextField(string label, Func<string> get, Action<string> set, bool options = true)
        {
            var t = Ui.Box(get(), 260);
            t.Leave += (s, e) => { set(t.Text); Changed(options); };
            Add(t, false, label);
        }

        void Build(int i)
        {
            body.SuspendLayout();
            switch (i)
            {
                case 0: LookAndFeel(); break;
                case 1: Behavior(); break;
                case 2: Parsing(); break;
                case 3: Display(); break;
                case 4: Calculation(); break;
                case 5: UnitsSection(); break;
                case 6: Shortcuts(); break;
                default: Fonts(); break;
            }
            body.ResumeLayout();
            foreach (Control c in form.Controls) Theme.Style(c);
        }

        void LookAndFeel()
        {
            Begin("Look & Feel");
            EnumChoice("Window title:", () => S.Title, v => { S.Title = v; main.UpdateTitle(""); }, false, new[] { "Application name", "Application name + result", "Application name + workspace", "Result only" });
            AccentPicker();
            Check("Keep above other windows", () => S.AlwaysOnTop, v => { S.AlwaysOnTop = v; main.TopMost = v; }, false);
            Check("Show status line (expression status and messages)", () => S.ShowStatus, v => { S.ShowStatus = v; main.CalcView.ApplyFonts(); }, false);
            Check("Colorize result", () => S.ColorizeResult, v => { S.ColorizeResult = v; main.CalcView.ApplyFonts(); }, false);
            Check("Show tooltips on keypad buttons", () => S.Tooltips, v => S.Tooltips = v, false);
            Group("Layout");
            Check("Show keypad", () => S.KeypadVisible, v => { S.KeypadVisible = v; main.CalcView.ApplyLayout(); }, false);
            Check("Always show number pad", () => S.NumpadVisible, v => { S.NumpadVisible = v; main.CalcView.ApplyLayout(); }, false);
            Check("Show side panel (history, RPN stack, number bases)", () => S.SideVisible, v => { S.SideVisible = v; main.CalcView.ApplyLayout(); }, false);
            Check("Place side panel on the left (requires restart)", () => S.SideLeft, v => S.SideLeft = v, false);
            Note("The window position and size are remembered between sessions.");
        }

        static readonly Color[] Accents =
        {
            Theme.DefaultAccent, Color.FromArgb(226, 92, 76), Color.FromArgb(214, 170, 60), Color.FromArgb(104, 178, 98),
            Color.FromArgb(64, 168, 160), Color.FromArgb(78, 140, 220), Color.FromArgb(150, 118, 220), Color.FromArgb(210, 100, 160)
        };

        // swatches plus a custom picker; also colors the title bar gradient
        void AccentPicker()
        {
            var row = Ui.Row();
            foreach (var c in Accents)
            {
                var col = c;
                var sw = new Panel { Width = 26, Height = 26, Margin = new Padding(0, 0, 6, 0), BackColor = Theme.Window, Cursor = Cursors.Hand };
                sw.Paint += (s, e) =>
                {
                    bool on = Theme.Accent.ToArgb() == col.ToArgb();
                    if (on) using (var p = new Pen(Theme.Text, 2f)) e.Graphics.DrawRectangle(p, 1, 1, sw.Width - 3, sw.Height - 3);
                    using (var b = new SolidBrush(col)) e.Graphics.FillRectangle(b, 4, 4, sw.Width - 8, sw.Height - 8);
                };
                sw.Click += (s, e) => SetAccent(col);
                row.Controls.Add(sw);
            }
            row.Controls.Add(new FlatButton("Custom…", (s, e) =>
            {
                using (var d = new ColorDialog { Color = Theme.Accent, FullOpen = true })
                    if (d.ShowDialog(main) == DialogResult.OK) SetAccent(d.Color);
            }) { Margin = new Padding(4, 0, 0, 0) });
            Add(row, false, "Accent color:");
        }

        void SetAccent(Color c)
        {
            S.AccentColor = AppSettings.ColorHex(c);
            Theme.SetAccent(c);
            Changed(false);
        }

        void Behavior()
        {
            Begin("Behavior");
            Group("Expression");
            EnumChoice("Expression after calculation:", () => S.ExpressionAfter, v => S.ExpressionAfter = v, false, new[] { "Keep expression", "Clear expression", "Replace with result", "Replace with result if shorter" });
            Check("Calculate as you type (shows the result in the status line)", () => S.CalcAsYouType, v => S.CalcAsYouType = v, false);
            Num("Calculate as you type delay (ms):", 0, 5000, () => S.AsYouTypeDelay, v => S.AsYouTypeDelay = v, false);
            Check("Adaptive delay", () => S.AdaptiveDelay, v => S.AdaptiveDelay = v, false);
            Check("Automatically calculate selection", () => S.AutoCalculateSelection, v => S.AutoCalculateSelection = v, false);
            EnumChoice("Completion:", () => S.Completion, v => S.Completion = v, false, new[] { "No completion", "Full completion", "Strict completion", "Limited full completion", "Limited strict completion" });
            Num("Minimum characters before completion:", 1, 10, () => S.CompletionMinChars, v => S.CompletionMinChars = v, false);
            Check("Close application with Escape key (when the expression is empty)", () => S.CloseWithEscape, v => S.CloseWithEscape = v, false);
            Group("Result");
            Check("Automatically copy result", () => S.AutoCopyResult, v => S.AutoCopyResult = v, false);
            Check("Copy unformatted ASCII by default", () => S.CopyAsciiDefault, v => S.CopyAsciiDefault = v, false);
            Check("Copy unformatted ASCII without units", () => S.CopyAsciiNoUnits, v => S.CopyAsciiNoUnits = v, false);
            Group("History");
            Num("Max history lines saved:", 10, 100000, () => S.MaxHistory, v => S.MaxHistory = v, false);
            Check("Clear history on exit (protected entries are kept)", () => S.ClearHistoryOnExit, v => S.ClearHistoryOnExit = v, false);
            EnumChoice("Expression in history:", () => S.HistoryExpression, v => { S.HistoryExpression = v; main.CalcView.History.Refill(); }, false, new[] { "Entered", "Parsed", "Entered + parsed" });
            Group("RPN and workspaces");
            Check("Use keyboard keys for RPN operations", () => S.RpnKeys, v => S.RpnKeys = v, false);
            Check("Ask to save the workspace on exit", () => S.AskSaveWorkspace, v => S.AskSaveWorkspace = v, false);
            Check("Allow multiple instances", () => S.MultipleInstances, v => S.MultipleInstances = v, false);
        }

        void Parsing()
        {
            Begin("Numbers & Operators");
            EnumChoice("Parsing mode:", () => O.Parse, v => O.Parse = v, true, new[] { "Conventional", "Implicit multiplication first", "Adaptive", "Chain", "RPN syntax" });
            Note("Adaptive: implicit multiplication binds tighter unless separated by a space, so 1/2x = 1/(2x) but 1/2 x = x/2.");
            Check("Limit implicit multiplication", () => O.LimitImplicitMult, v => O.LimitImplicitMult = v);
            Check("Interpret unrecognized symbols as unknown variables", () => O.UnknownAsVariables, v => O.UnknownAsVariables = v);
            Check("Read precision (decimal input is approximate)", () => O.ReadPrecision, v => O.ReadPrecision = v);
            Check("Allow concise uncertainty input, e.g. 1.23(4)", () => O.ConciseUncertaintyInput, v => O.ConciseUncertaintyInput = v);
            Check("Simplified percentage calculation (100 + 10% = 110)", () => O.SimplifiedPercent, v => O.SimplifiedPercent = v);
            Group("Separators");
            Check("Use comma as decimal separator (semicolon separates arguments)", () => O.CommaDecimal, v => O.CommaDecimal = v);
            Check("Ignore comma in numbers", () => O.IgnoreComma, v => O.IgnoreComma = v);
            Check("Ignore dots in numbers", () => O.IgnoreDots, v => O.IgnoreDots = v);
            Group("Operators");
            Check("Use caret for bitwise XOR", () => O.CaretXor, v => O.CaretXor = v);
            Check("Use 'j' as imaginary unit", () => O.UseJ, v => O.UseJ = v);
            Group("Number base");
            Choice("Expression base:", new[] { "Decimal", "Binary", "Octal", "Duodecimal", "Hexadecimal" }, () => Array.IndexOf(new[] { 10, 2, 8, 12, 16 }, O.InputBase) < 0 ? 0 : Array.IndexOf(new[] { 10, 2, 8, 12, 16 }, O.InputBase), i => O.InputBase = new[] { 10, 2, 8, 12, 16 }[i]);
            Check("Two's complement input for binary numbers", () => O.TwosComplementInput, v => O.TwosComplementInput = v);
            Check("Two's complement input for hexadecimal numbers", () => O.HexTwosComplementInput, v => O.HexTwosComplementInput = v);
        }

        void Display()
        {
            Begin("Result Display");
            EnumChoice("Number display:", () => O.Display, v => O.Display = v, true, new[] { "Normal", "Scientific", "Engineering", "Simple", "Purely scientific" });
            Num("Precision (significant digits):", 2, 5000, () => O.Precision, v => O.Precision = v);
            Num("Min decimals:", 0, 100, () => O.MinDecimals, v => O.MinDecimals = v);
            Num("Max decimals (−1 = off):", -1, 100, () => O.MaxDecimals, v => O.MaxDecimals = v);
            EnumChoice("Rounding:", () => O.Rounding, v => O.Rounding = v, true, new[] { "Round halfway numbers away from zero", "Round halfway numbers to even", "Round halfway numbers to odd", "Round halfway numbers toward zero", "Round halfway numbers to random", "Round halfway numbers up", "Round halfway numbers down", "Round toward zero", "Round away from zero", "Round up", "Round down" });
            EnumChoice("Fractions:", () => O.Fraction, v => O.Fraction = v, true, new[] { "Decimal", "Exact decimal", "Fraction", "Mixed fraction", "Percent", "Per mille", "Per myriad" });
            EnumChoice("Indicate repeating decimals:", () => O.Repeating, v => O.Repeating = v, true, new[] { "Off", "Ellipsis", "Overline" });
            EnumChoice("Digit grouping:", () => O.Grouping, v => O.Grouping = v, true, new[] { "Off", "Standard (thin space)", "Local (comma)" });
            EnumChoice("Complex number form:", () => O.Complex, v => O.Complex = v, true, new[] { "Rectangular", "Exponential", "Polar", "Cis", "Angle/phasor" });
            EnumChoice("Interval display:", () => O.IntervalOut, v => O.IntervalOut = v, true, new[] { "Adaptive", "Significant digits", "Interval", "Plus/minus", "Midpoint", "Lower", "Upper", "Concise", "Relative" });
            Group("Notation");
            Check("Use E-notation instead of 10^n", () => O.ENotation, v => O.ENotation = v);
            Check("Use lower case e in E-notation", () => O.LowerCaseE, v => O.LowerCaseE = v);
            Check("Use lower case letters in non-decimal numbers", () => O.LowerCaseHex, v => O.LowerCaseHex = v);
            Check("Use special duodecimal symbols (↊ ↋)", () => O.DuodecimalSymbols, v => O.DuodecimalSymbols = v);
            Check("Show ending zeroes of approximate numbers", () => O.ShowEndingZeroes, v => O.ShowEndingZeroes = v);
            Check("Use Unicode symbols (×, −, √, superscripts)", () => O.Unicode, v => O.Unicode = v);
            EnumChoice("Multiplication sign:", () => O.Mult, v => O.Mult = v, true, new[] { "Dot operator (⋅)", "Multiplication sign (×)", "Asterisk (*)", "Middle dot (·)" });
            Check("Use Unicode division slash in output", () => O.UnicodeDivision, v => O.UnicodeDivision = v);
            Check("Implicit multiplication in output (2x, 3π)", () => O.ImplicitMultOutput, v => O.ImplicitMultOutput = v);
            Check("Spell out logical operators", () => O.SpellOutLogical, v => O.SpellOutLogical = v);
            Check("Abbreviate names (unit symbols instead of names)", () => O.AbbreviateNames, v => O.AbbreviateNames = v);
            Group("Number bases");
            EnumChoice("Two's complement output:", () => O.TwosOut, v => O.TwosOut = v, true, new[] { "Off", "Binary", "Binary and hexadecimal" });
            Num("Binary bits (0 = automatic):", 0, 4096, () => O.BinaryBits, v => O.BinaryBits = v);
            Num("Binary letter spacing:", 0, 16, () => O.BinaryLetterSpacing, v => O.BinaryLetterSpacing = v);
        }

        void Calculation()
        {
            Begin("Calculation");
            EnumChoice("Approximation:", () => O.Approx, v => O.Approx = v, true, new[] { "Automatic", "Dual (exact and approximate)", "Exact", "Approximate" });
            EnumChoice("Angle unit:", () => O.Angle, v => O.Angle = v, true, new[] { "Radians", "Degrees", "Gradians", "None" });
            EnumChoice("Result structure:", () => O.Structure, v => O.Structure = v, true, new[] { "Simplify", "Factorize", "Expand", "None" });
            EnumChoice("Interval calculation:", () => O.Interval, v => O.Interval = v, true, new[] { "Variance formula", "Interval arithmetic", "Simple" });
            Check("Allow complex numbers", () => O.ComplexEnabled, v => O.ComplexEnabled = v);
            Check("Allow infinite numbers", () => O.InfiniteEnabled, v => O.InfiniteEnabled = v);
            Check("Use real roots for odd roots of negative numbers", () => O.RealRoots, v => O.RealRoots = v);
            Check("Enable variables and constants", () => O.VariablesEnabled, v => O.VariablesEnabled = v);
            Check("Enable functions", () => O.FunctionsEnabled, v => O.FunctionsEnabled = v);
            Group("Default assumptions for unknown variables");
            EnumChoice("Type:", () => O.DefaultType, v => O.DefaultType = v);
            EnumChoice("Sign:", () => O.DefaultSign, v => O.DefaultSign = v);
        }

        void UnitsSection()
        {
            Begin("Units & Currencies");
            Check("Enable units", () => O.UnitsEnabled, v => O.UnitsEnabled = v);
            Check("Enable units in physical constants", () => O.UnitsInConstants, v => O.UnitsInConstants = v);
            EnumChoice("Automatic unit conversion:", () => O.AutoConvert, v => O.AutoConvert = v, true, new[] { "No conversion", "Optimal SI units", "Optimal units", "Base units" });
            Check("Convert to mixed units (1.5 h = 1 h + 30 min)", () => O.MixedUnits, v => O.MixedUnits = v);
            EnumChoice("Automatic unit prefixes:", () => O.Prefixes, v => O.Prefixes = v, true, new[] { "No prefixes", "Prefixes for some units", "Prefixes for all units", "Prefixes also for currencies" });
            Check("Enable denominator prefixes", () => O.DenominatorPrefixes, v => O.DenominatorPrefixes = v);
            Check("Enable all SI prefixes", () => O.AllSiPrefixes, v => O.AllSiPrefixes = v);
            Check("Prefixes also for currencies", () => O.CurrencyPrefixes, v => O.CurrencyPrefixes = v);
            Check("Use binary prefixes for information units", () => O.BinaryPrefixes, v => O.BinaryPrefixes = v);
            EnumChoice("Temperature calculation:", () => O.Temperature, v => O.Temperature = v, true, new[] { "Hybrid", "Absolute", "Relative" });
            Group("Currencies");
            Choice("Exchange rates updates:", new[] { "Never", "Daily", "Weekly", "Monthly" }, () => S.RatesUpdateDays <= 0 ? 0 : S.RatesUpdateDays == 1 ? 1 : S.RatesUpdateDays <= 7 ? 2 : 3, i => S.RatesUpdateDays = new[] { 0, 1, 7, 30 }[i], false);
            var units = main.Calc.Units.Where(u => u.IsCurrency).Select(u => u.Name).OrderBy(x => x).ToArray();
            Choice("Default currency:", units, () => Math.Max(0, Array.IndexOf(units, S.DefaultCurrency)), i => S.DefaultCurrency = units[i], false);
            var b = new FlatButton("Update exchange rates now", (s, e) => main.UpdateRates(true), true) { Width = 220, Margin = new Padding(0, 8, 0, 0) };
            Add(b, true);
            Note(string.IsNullOrEmpty(S.LastRatesUpdate) ? "Exchange rates have not been downloaded yet; built-in approximate rates are used." : "Last update: " + S.LastRatesUpdate.Replace("T", " ") + ". Rates come from the European Central Bank.");
        }

        void Shortcuts()
        {
            Begin("Keyboard Shortcuts");
            var grid = new DataGridView { Width = 640, Height = 460, ReadOnly = true, AllowUserToAddRows = false, SelectionMode = DataGridViewSelectionMode.FullRowSelect, MultiSelect = false, AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill };
            Theme.StyleGrid(grid);
            grid.Columns.Add("a", "Action");
            grid.Columns.Add("k", "Key combination");
            void Fill()
            {
                grid.Rows.Clear();
                foreach (var a in main.Actions) grid.Rows.Add(a.Title, main.ShortcutFor(a));
            }
            Fill();
            Add(grid, true);
            void EditRow()
            {
                if (grid.CurrentRow == null) return;
                var a = main.Actions[grid.CurrentRow.Index];
                using (var d = new ShortcutDialog(a.Title, main.ShortcutFor(a)))
                {
                    if (d.ShowDialog(main) != DialogResult.OK) return;
                    var other = main.Actions.FirstOrDefault(x => x != a && d.Value.Length > 0 && main.ShortcutFor(x) == d.Value);
                    if (other != null)
                    {
                        if (!Ui.Confirm(main, "The key combination is already in use.\nDo you wish to replace the current action (" + other.Title + ")?")) return;
                        S.Shortcuts[other.Id] = "";
                    }
                    S.Shortcuts[a.Id] = d.Value;
                }
                main.ApplyShortcuts();
                Fill();
                Changed(false);
            }
            grid.CellDoubleClick += (s, e) => EditRow();
            var row = Ui.Row(new FlatButton("Edit…", (s, e) => EditRow(), true), new FlatButton("Reset", (s, e) =>
            {
                if (grid.CurrentRow == null) return;
                S.Shortcuts.Remove(main.Actions[grid.CurrentRow.Index].Id);
                main.ApplyShortcuts(); Fill(); Changed(false);
            }), new FlatButton("Reset all", (s, e) => { S.Shortcuts.Clear(); main.ApplyShortcuts(); Fill(); Changed(false); }));
            foreach (Control c in row.Controls) c.Margin = new Padding(0, 8, 8, 0);
            Add(row, true);
        }

        void Fonts()
        {
            Begin("Fonts");
            void FontRow(string label, Func<string> get, Action<string> set, Font def)
            {
                var cur = AppSettings.ParseFont(get(), def);
                var l = new Label { Text = cur.Name + ", " + cur.SizeInPoints + " pt", AutoSize = true, ForeColor = Theme.Text, Font = cur, Margin = new Padding(0, 6, 12, 4), MaximumSize = new Size(300, 0) };
                var choose = new FlatButton("Choose…", (s, e) =>
                {
                    using (var fd = new FontDialog { Font = AppSettings.ParseFont(get(), def), ShowEffects = false })
                    {
                        if (fd.ShowDialog(main) != DialogResult.OK) return;
                        set(AppSettings.FontSpec(fd.Font));
                        main.CalcView.ApplyFonts();
                        Changed(false);
                        Build(7);
                    }
                });
                var reset = new FlatButton("Default", (s, e) => { set(""); main.CalcView.ApplyFonts(); Changed(false); Build(7); });
                var r = Ui.Row(l, choose, reset);
                choose.Margin = new Padding(0, 0, 6, 0);
                Add(r, false, label);
            }
            FontRow("Expression font:", () => S.ExpressionFont, v => S.ExpressionFont = v, Theme.ExpressionFont);
            FontRow("Result font:", () => S.ResultFont, v => S.ResultFont = v, Theme.ResultFont);
            FontRow("History font:", () => S.HistoryFont, v => S.HistoryFont = v, Theme.HistoryFont);
            FontRow("Keypad font:", () => S.KeypadFont, v => S.KeypadFont = v, Theme.KeypadFont);
            FontRow("Status font:", () => S.StatusFont, v => S.StatusFont = v, Theme.StatusFont);
            FontRow("Application font (restart):", () => S.AppFont, v => S.AppFont = v, Theme.UiFont);
        }

        public void Activated() { Build(sections.SelectedIndex); }
        public void DefinitionsChanged() { }
        public void OptionsChanged() { }
    }
}