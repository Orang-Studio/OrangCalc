using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Numerics;
using System.Threading.Tasks;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class CalcPage : Panel, IKeypadHost
    {
        readonly MainForm main;
        readonly Calculator calc;
        readonly CalcService svc;
        readonly AppSettings settings;
        public readonly ExpressionBox Expr;
        public readonly ResultView Result;
        readonly Label status;
        readonly LinkLabel abortLink;
        readonly FlowLayoutPanel modeStrip;
        public readonly KeypadPanel Keypad;
        readonly SplitContainer split;
        readonly SegmentBar sideTabs;
        readonly Panel sideHost;
        public readonly HistoryPanel History;
        public readonly RpnPanel Rpn;
        public readonly BasesPanel Bases;
        readonly TableLayoutPanel column;
        public readonly List<Expr> Stack = new List<Expr>();
        Expr lastX;
        public Display Last;
        string lastCalcText;
        bool justCalculated;
        readonly Timer previewTimer = new Timer();
        int previewGen;
        readonly Timer busyTimer = new Timer { Interval = 250 };
        readonly DarkToolTip tip = new DarkToolTip();
        readonly Dictionary<string, FlatKey> chips = new Dictionary<string, FlatKey>();

        public CalcPage(MainForm m, CalcService s, AppSettings st, List<HistoryEntry> history)
        {
            main = m; svc = s; calc = s.Calc; settings = st;
            BackColor = Theme.Window;
            Dock = DockStyle.Fill;

            Expr = new ExpressionBox(calc, settings) { Dock = DockStyle.Fill, Margin = new Padding(12, 12, 12, 0) };
            Expr.CalculateRequested += (o, e) => Calculate();
            Expr.RpnActive = () => settings.RpnMode;
            Expr.RpnOperator += ch => RpnOp(ch.ToString());
            Expr.InsertDateRequested += (o, e) => InsertDate();
            Expr.InsertMatrixRequested += (o, e) => OpenMatrixEditor();
            Expr.CalculateSelectionRequested += (o, e) => CalculateSelection();
            Expr.HistoryProvider = () => History.Entries.AsEnumerable().Reverse().Select(h => h.Expression).Distinct().ToList();
            Expr.Box.TextChanged += (o, e) => OnExpressionChanged();
            Expr.Box.KeyPress += (o, e) =>
            {
                if (e.Handled || !justCalculated || settings.RpnMode) return;
                if ("+-*/^×÷".IndexOf(e.KeyChar) >= 0 && Expr.Box.SelectionLength == Expr.Box.TextLength && Expr.Box.TextLength > 0)
                {
                    Expr.SetText("ans");
                }
            };
            Expr.Box.KeyDown += (o, e) =>
            {
                if (e.KeyCode == Keys.Escape)
                {
                    if (Expr.Box.TextLength > 0) { Expr.SetText(""); e.Handled = e.SuppressKeyPress = true; }
                    else if (settings.CloseWithEscape) main.Close();
                }
            };

            var statusRow = new Panel { Dock = DockStyle.Fill, Margin = new Padding(14, 0, 14, 0), BackColor = Theme.Window };
            status = new Label { Dock = DockStyle.Fill, ForeColor = Theme.TextFaint, Font = Theme.StatusFont, TextAlign = ContentAlignment.MiddleLeft, AutoEllipsis = true, UseMnemonic = false };
            abortLink = new LinkLabel { Text = "Abort", Dock = DockStyle.Right, Width = 60, TextAlign = ContentAlignment.MiddleRight, LinkColor = Theme.Accent, ActiveLinkColor = Theme.Accent, Visible = false, LinkBehavior = LinkBehavior.HoverUnderline };
            abortLink.LinkClicked += (o, e) => svc.Abort();
            statusRow.Controls.Add(status);
            statusRow.Controls.Add(abortLink);

            Result = new ResultView { Dock = DockStyle.Fill, Margin = new Padding(0) };
            Result.ContextMenuStrip = BuildResultMenu();
            Result.DoubleClick += (o, e) => CopyResult(false);

            modeStrip = new FlowLayoutPanel { Dock = DockStyle.Fill, BackColor = Theme.Window, Padding = new Padding(10, 3, 10, 3), WrapContents = true, Margin = new Padding(0) };
            BuildModeStrip();

            Keypad = new KeypadPanel(this, settings) { Dock = DockStyle.Fill };
            Keypad.CustomButtonsChanged += (o, e) => main.SaveAll();

            column = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = 1, RowCount = 5, BackColor = Theme.Window, Margin = new Padding(0), Padding = new Padding(0) };
            column.RowStyles.Add(new RowStyle(SizeType.Absolute, 104));
            column.RowStyles.Add(new RowStyle(SizeType.Absolute, 24));
            column.RowStyles.Add(new RowStyle(SizeType.Absolute, 120));
            column.RowStyles.Add(new RowStyle(SizeType.Absolute, 36));
            column.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
            column.Controls.Add(Expr, 0, 0);
            column.Controls.Add(statusRow, 0, 1);
            column.Controls.Add(Result, 0, 2);
            column.Controls.Add(modeStrip, 0, 3);
            column.Controls.Add(Keypad, 0, 4);
            column.Resize += (o, e) => FitModeStrip();

            History = new HistoryPanel(history, settings) { Dock = DockStyle.Fill };
            History.InsertText += t => Insert(t);
            History.InsertValue += h => InsertHistoryValue(h);
            History.Changed += (o, e) => main.MarkDirty();
            Rpn = new RpnPanel(Stack, x => FormatShort(x)) { Dock = DockStyle.Fill };
            Rpn.Operation += op => StackOp(op);
            Rpn.EditRegister += (i, t) => EditRegister(i, t);
            Bases = new BasesPanel { Dock = DockStyle.Fill };
            Bases.ValueEdited += v =>
            {
                Expr.SetText(FormatInputBase(v));
                Calculate();
            };

            sideTabs = new SegmentBar { Dock = DockStyle.Top, BackColor = Theme.Panel };
            sideTabs.Items.AddRange(new[] { "History", "Stack", "Bases" });
            sideTabs.SelectedChanged += (o, e) => ShowSide(sideTabs.SelectedIndex);
            sideHost = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Panel };
            var side = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Panel };
            side.Controls.Add(sideHost);
            side.Controls.Add(sideTabs);

            split = new SplitContainer { Dock = DockStyle.Fill, BackColor = Theme.Line, SplitterWidth = 4, Orientation = Orientation.Vertical };
            split.Panel1.BackColor = Theme.Window; split.Panel2.BackColor = Theme.Panel;
            if (settings.SideLeft) { split.Panel1.Controls.Add(side); split.Panel2.Controls.Add(column); split.FixedPanel = FixedPanel.Panel1; }
            else { split.Panel1.Controls.Add(column); split.Panel2.Controls.Add(side); split.FixedPanel = FixedPanel.Panel2; }
            Controls.Add(split);
            split.SplitterMoved += (o, e) => settings.SideWidth = settings.SideLeft ? split.SplitterDistance : split.Width - split.SplitterDistance - split.SplitterWidth;
            HandleCreated += (o, e) => BeginInvoke((Action)ApplyLayout);

            previewTimer.Tick += (o, e) => { previewTimer.Stop(); RunPreview(); };
            busyTimer.Tick += (o, e) => { busyTimer.Stop(); if (svc.Busy) { Result.ShowBusy(true); abortLink.Visible = true; } };
            ShowSide(Math.Max(0, Math.Min(2, settings.SideTab)));
            sideTabs.SelectedIndex = Math.Max(0, Math.Min(2, settings.SideTab));
            ApplyFonts();
        }

        public void ApplyLayout()
        {
            try
            {
                int sw = Math.Max(220, Math.Min(Width - 420, settings.SideWidth));
                if (settings.SideLeft) { split.Panel1Collapsed = !settings.SideVisible; if (settings.SideVisible) split.SplitterDistance = sw; }
                else { split.Panel2Collapsed = !settings.SideVisible; if (settings.SideVisible && split.Width > sw + 200) split.SplitterDistance = split.Width - sw - split.SplitterWidth; }
            }
            catch (InvalidOperationException) { }
            bool kp = settings.KeypadVisible && settings.Keypad != KeypadMode.None;
            Keypad.Visible = kp;
            column.RowStyles[2] = kp ? new RowStyle(SizeType.Absolute, 120) : new RowStyle(SizeType.Percent, 100);
            column.RowStyles[4] = kp ? new RowStyle(SizeType.Percent, 100) : new RowStyle(SizeType.Absolute, 0);
            Keypad.ApplyNumpadVisibility();
            RefreshModes();
            FitModeStrip();
        }

        // the mode buttons wrap to a second line in narrow windows
        void FitModeStrip()
        {
            if (column.Width <= 0) return;
            int h = Math.Max(36, modeStrip.GetPreferredSize(new Size(column.ClientSize.Width, 0)).Height);
            if ((int)column.RowStyles[3].Height != h) column.RowStyles[3].Height = h;
        }

        public void ApplyFonts()
        {
            Expr.Box.Font = AppSettings.ParseFont(settings.ExpressionFont, Theme.ExpressionFont);
            Result.ResultFont = AppSettings.ParseFont(settings.ResultFont, Theme.ResultFont);
            History.EntryFont = AppSettings.ParseFont(settings.HistoryFont, Theme.HistoryFont);
            status.Font = AppSettings.ParseFont(settings.StatusFont, Theme.StatusFont);
            Keypad.SetKeyFont(AppSettings.ParseFont(settings.KeypadFont, Theme.KeypadFont));
            Result.Colorize = settings.ColorizeResult;
            History.Colorize = settings.ColorizeResult;
            status.Visible = settings.ShowStatus;
            Result.Invalidate();
            History.Refill();
        }

        void ShowSide(int i)
        {
            settings.SideTab = i;
            sideHost.Controls.Clear();
            sideHost.Controls.Add(i == 0 ? (Control)History : i == 1 ? Rpn : Bases);
            if (i == 2 && Last != null) Bases.SetValue(Last.Value);
        }

        public void SelectSide(int i) { settings.SideVisible = true; ApplyLayout(); sideTabs.SelectedIndex = i; }

        public void FocusExpression() { Expr.Box.Focus(); }

        string FormatShort(Expr e)
        {
            try { return Formatter.Format(e, calc.Opts, out _); } catch (Exception) { return "?"; }
        }

        string FormatInputBase(BigInteger v)
        {
            int b = calc.Opts.InputBase;
            if (b == 10 || b < 2 || b > 36) return v.ToString();
            var o = calc.Opts.Clone(); o.OutputBase = b; o.TwosOut = TwosComplement.Off; o.BinaryLetterSpacing = 0;
            return Formatter.Format(new NumE(new Cx(Real.Int(v))), o, out _).Replace("−", "-");
        }

        // mode strip with quick dropdown chips
        void BuildModeStrip()
        {
            FlatKey Chip(string id, string tipText)
            {
                var k = new FlatKey { Label = id, Width = 74, Height = 28, Font = Theme.UiFont, LabelFont = Theme.UiFont, KeyStyle = KeyStyle.Muted, Margin = new Padding(0, 0, 6, 0) };
                k.Primary = () => ShowChipMenu(id, k);
                k.Secondary = k.Primary;
                tip.SetToolTip(k, tipText);
                chips[id] = k;
                modeStrip.Controls.Add(k);
                return k;
            }
            Chip("angle", "Angle unit");
            Chip("approx", "Approximation");
            Chip("display", "Number display");
            Chip("base", "Result base");
            Chip("fraction", "Fractions and decimals");
            Chip("mode", "Normal, RPN or chain mode");
            var spacer = new Panel { Width = 8, Height = 1, BackColor = Theme.Window };
            modeStrip.Controls.Add(spacer);
            var kp = Chip("keys", "Show or hide the keypad");
            kp.Toggle = true;
            kp.Primary = () => { settings.KeypadVisible = !settings.KeypadVisible; if (settings.Keypad == KeypadMode.None) settings.Keypad = KeypadMode.General; ApplyLayout(); };
            kp.Secondary = () => ShowChipMenu("keys", kp);
            var sp = Chip("panel", "Show or hide the side panel");
            sp.Toggle = true;
            sp.Primary = () => { settings.SideVisible = !settings.SideVisible; ApplyLayout(); };
            RefreshModes();
        }

        public void RefreshModes()
        {
            if (chips.Count == 0) return;
            var o = calc.Opts;
            chips["angle"].Label = o.Angle == AngleMode.Degrees ? "DEG" : o.Angle == AngleMode.Gradians ? "GRA" : o.Angle == AngleMode.Radians ? "RAD" : "ANG –";
            chips["approx"].Label = o.Approx == ApproxMode.Auto ? "AUTO" : o.Approx == ApproxMode.Dual ? "DUAL" : o.Approx == ApproxMode.Exact ? "EXACT" : "APPROX";
            chips["display"].Label = o.Display == NumberDisplay.Scientific ? "SCI" : o.Display == NumberDisplay.Engineering ? "ENG" : o.Display == NumberDisplay.Simple ? "SIMPLE" : o.Display == NumberDisplay.PureScientific ? "PURE" : "NORM";
            chips["display"].Hint = o.Precision.ToString();
            chips["base"].Label = BaseLabel(o.OutputBase);
            chips["base"].Hint = o.InputBase != 10 ? "in " + BaseLabel(o.InputBase) : "";
            chips["fraction"].Label = o.Fraction == FractionMode.Fraction ? "a/b" : o.Fraction == FractionMode.Combined ? "a b/c" : o.Fraction == FractionMode.Percent ? "%" : o.Fraction == FractionMode.Permille ? "‰" : o.Fraction == FractionMode.Permyriad ? "‱" : o.Fraction == FractionMode.Decimal ? "0.1" : "0.1…";
            chips["mode"].Label = settings.RpnMode ? "RPN" : o.Parse == ParseMode.Chain ? "CHAIN" : "NORMAL";
            chips["keys"].Label = "Keypad"; chips["keys"].Checked = settings.KeypadVisible;
            chips["panel"].Label = "Panel"; chips["panel"].Checked = settings.SideVisible;
            foreach (var c in chips.Values) c.Invalidate();
            Keypad?.RefreshBaseKeys();
        }

        public static string BaseLabel(int b)
        {
            switch (b)
            {
                case 2: return "BIN";
                case 8: return "OCT";
                case 10: return "DEC";
                case 12: return "DUO";
                case 16: return "HEX";
                case Engine.Bases.Roman: return "ROMAN";
                case Engine.Bases.Time: return "TIME";
                case Engine.Bases.Sexagesimal: return "SEXA";
                case Engine.Bases.Bijective26: return "BIJ26";
                case Engine.Bases.Bcd: return "BCD";
                case Engine.Bases.Unicode: return "UNI";
                case Engine.Bases.Golden: return "φ";
                case Engine.Bases.Supergolden: return "ψ";
                case Engine.Bases.PiBase: return "π";
                case Engine.Bases.EBase: return "e";
                case Engine.Bases.Sqrt2: return "√2";
                case Engine.Bases.Fp32: return "FP32";
                case Engine.Bases.Fp64: return "FP64";
                case Engine.Bases.Fp16: return "FP16";
                case Engine.Bases.Fp80: return "FP80";
                case Engine.Bases.Fp128: return "FP128";
                case Engine.Bases.Custom: return "CUSTOM";
                default: return "B" + b;
            }
        }

        void ShowChipMenu(string id, Control anchor)
        {
            var m = main.BuildModeMenu(id);
            if (m == null) return;
            m.Show(anchor, new Point(0, anchor.Height));
        }

        ContextMenuStrip BuildResultMenu()
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            m.Items.Add("Copy", null, (s, e) => CopyResult(false));
            m.Items.Add("Copy Unformatted ASCII", null, (s, e) => CopyResult(true));
            m.Items.Add("Copy Without Units", null, (s, e) => { if (Last != null) HistoryPanel.SafeClipboard(Last.AsciiNoUnits); });
            m.Items.Add("Copy Expression and Result", null, (s, e) => { if (Last != null) HistoryPanel.SafeClipboard(Last.Input + " = " + Last.Text); });
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Store as Variable…", null, (s, e) => StoreResult());
            m.Items.Add("Export as CSV…", null, (s, e) => main.RunAction("csvexport"));
            m.Items.Add("Plot", null, (s, e) => main.RunAction("plotexpr"));
            m.Items.Add(new ToolStripSeparator());
            var conv = new ToolStripMenuItem("Convert");
            m.Items.Add(conv);
            m.Opening += (s, e) =>
            {
                conv.DropDownItems.Clear();
                foreach (ToolStripItem it in main.BuildConversionMenu(t => ConvertTo(t)).Items.Cast<ToolStripItem>().ToList()) conv.DropDownItems.Add(it);
            };
            return m;
        }

        public void CopyResult(bool ascii)
        {
            if (Last == null || Last.Error != null) return;
            bool useAscii = ascii || settings.CopyAsciiDefault;
            var t = useAscii ? (settings.CopyAsciiNoUnits ? Last.AsciiNoUnits : Last.Ascii) : Last.Text;
            HistoryPanel.SafeClipboard(t);
            SetStatus("Copied " + t, false);
        }

        void SetStatus(string text, bool error)
        {
            status.Text = text;
            status.ForeColor = error ? Theme.Error : Theme.TextFaint;
        }

        void OnExpressionChanged()
        {
            justCalculated = false;
            if (!settings.CalcAsYouType || settings.RpnMode) { previewTimer.Stop(); return; }
            int delay = settings.AsYouTypeDelay;
            if (settings.AdaptiveDelay) delay = Expr.Box.TextLength < 12 ? Math.Min(delay, 120) : delay;
            previewTimer.Interval = Math.Max(30, delay);
            previewTimer.Stop();
            previewTimer.Start();
            previewGen++;
        }

        void RunPreview()
        {
            var text = Expr.Box.Text.Trim();
            int gen = previewGen;
            if (text.Length == 0 || text == lastCalcText && justCalculated) { SetStatus("", false); return; }
            if (svc.Busy) { svc.Abort(); }
            svc.Run(() => Display.From(calc.Calculate(text, 1500, true), calc)).ContinueWith(t =>
            {
                if (t.IsFaulted || gen != previewGen || IsDisposed) return;
                var d = t.Result;
                BeginInvoke((Action)(() =>
                {
                    if (gen != previewGen) return;
                    if (d.Error != null) SetStatus(d.ParsedText.Length > 0 ? d.ParsedText + "   " + d.Error : d.Error, false);
                    else if (d.Empty) SetStatus("", false);
                    else
                    {
                        var res = (d.Approx ? "≈ " : "= ") + (d.Text.Length > 200 ? d.Text.Substring(0, 200) + "…" : d.Text);
                        var parsed = d.ParsedText;
                        SetStatus(parsed.Length > 0 && parsed != text ? parsed + "   " + res : res, false);
                    }
                }));
            });
        }

        public void Calculate() => Calculate(Expr.Box.Text);

        public void Calculate(string input, bool addHistory = true)
        {
            var text = (input ?? "").Trim();
            previewTimer.Stop();
            previewGen++;
            if (settings.RpnMode) { RpnEnter(); return; }
            if (text.Length == 0) return;
            lastCalcText = text;
            busyTimer.Stop(); busyTimer.Start();
            svc.Run(() => Display.From(calc.Calculate(text), calc)).ContinueWith(t =>
            {
                if (IsDisposed) return;
                BeginInvoke((Action)(() =>
                {
                    busyTimer.Stop();
                    abortLink.Visible = false;
                    if (t.IsFaulted) { Result.ShowError(text, t.Exception?.InnerException?.Message ?? "Error"); return; }
                    ShowDisplay(t.Result, addHistory);
                }));
            });
        }

        void ShowDisplay(Display d, bool addHistory)
        {
            Last = d;
            if (d.Error != null)
            {
                Result.ShowError(d.ParsedText, d.Error);
                SetStatus(d.Error, true);
                if (addHistory) History.Add(new HistoryEntry { Expression = d.Input, Parsed = d.ParsedText, Result = d.Error, IsError = true });
                main.MarkDirty();
                return;
            }
            if (d.Empty) { Result.Clear(); SetStatus("", false); return; }
            Result.ShowResult(d.ParsedText, d.Tokens, d.Approx, d.ExactTokens, d.Matrix);
            SetStatus(d.Warnings.Count > 0 ? string.Join("; ", d.Warnings) : d.Comment, d.Warnings.Count > 0);
            if (addHistory)
            {
                History.Add(new HistoryEntry
                {
                    Expression = d.Input, Parsed = d.ParsedText, Result = d.Text, ResultAscii = d.Ascii, ExactText = d.ExactText, Tokens = d.Tokens,
                    Approx = d.Approx, Value = d.Value, Comment = d.Comment, IsMessage = d.Value is StrE
                });
            }
            if (sideTabs.SelectedIndex == 2 || settings.Keypad == KeypadMode.Programming) Bases.SetValue(d.Value);
            if (settings.AutoCopyResult) CopyResult(false);
            if (d.Assigned != null) main.DefinitionsChanged();
            main.UpdateTitle(d.Text);
            main.MarkDirty();
            switch (settings.ExpressionAfter)
            {
                case AfterCalc.Clear: Expr.SetText(""); break;
                case AfterCalc.ReplaceWithResult: if (!(d.Value is StrE)) Expr.SetText(d.Ascii, true); break;
                case AfterCalc.ReplaceIfShorter: if (!(d.Value is StrE) && d.Ascii.Length < d.Input.Length) Expr.SetText(d.Ascii, true); else Expr.Box.SelectAll(); break;
                default: Expr.Box.SelectAll(); break;
            }
            justCalculated = true;
            previewGen++;
            Expr.Box.Focus();
        }

        void CalculateSelection()
        {
            var sel = Expr.Box.SelectedText;
            if (sel.Trim().Length == 0) return;
            svc.Run(() => Display.From(calc.Calculate(sel, 3000, true), calc)).ContinueWith(t =>
            {
                if (t.IsFaulted) return;
                var d = t.Result;
                BeginInvoke((Action)(() =>
                {
                    if (d.Error != null) SetStatus(d.Error, true);
                    else if (!d.Empty)
                    {
                        int start = Expr.Box.SelectionStart;
                        Expr.Box.SelectedText = NeedsParens(d.Ascii) ? "(" + d.Ascii + ")" : d.Ascii;
                        Expr.Box.SelectionStart = start;
                    }
                }));
            });
        }

        static bool NeedsParens(string s) => s.IndexOfAny(new[] { ' ', '+', '-', '*', '/', '^' }) > 0;

        void InsertHistoryValue(HistoryEntry h)
        {
            if (h.IsError || h.IsMessage) { Insert(h.Expression); return; }
            var t = h.ResultAscii.Length > 0 ? h.ResultAscii : h.Result;
            if (Expr.Box.TextLength > 0 && NeedsParens(t)) t = "(" + t + ")";
            Insert(t);
        }

        public void ConvertTo(string target)
        {
            var text = Expr.Box.Text.Trim();
            if (text.Length == 0) { if (Last == null) return; text = "ans"; }
            int to = Calculator.FindTopLevel(text, " to ");
            if (to > 0 && justCalculated) text = text.Substring(0, to);
            Expr.SetText(text + " to " + target);
            Calculate();
        }

        public void Insert(string text, int caretBack = 0)
        {
            if (justCalculated && Expr.Box.SelectionLength == Expr.Box.TextLength && Expr.Box.TextLength > 0 && settings.ExpressionAfter == AfterCalc.Keep)
            {
                if (text.Length > 0 && (char.IsDigit(text[0]) || char.IsLetter(text[0]) || text[0] == '(' || text[0] == '.')) Expr.SetText("");
                else Expr.Box.SelectionStart = Expr.Box.TextLength;
            }
            justCalculated = false;
            Expr.Insert(text, caretBack);
        }

        public void InsertOperator(string op)
        {
            if (settings.RpnMode && (op == "+" || op == "-" || op == "*" || op == "/" || op == "^" || op == "^2" || op == "^3" || op == "^-1" || op == "!" || op == "&" || op == "|" || op == "<<" || op == ">>" || op == " mod " || op == "//" || op == " xor "))
            {
                RpnOp(op);
                return;
            }
            if (justCalculated && Expr.Box.SelectionLength == Expr.Box.TextLength && Expr.Box.TextLength > 0)
            {
                Expr.SetText("ans");
            }
            justCalculated = false;
            Expr.Insert(op);
        }

        public void InsertFunction(string name, string extra = "")
        {
            if (settings.RpnMode && name.Length > 0 && Stack.Count > 0 && Expr.Box.TextLength == 0)
            {
                RpnFunction(name, extra);
                return;
            }
            bool wrapAll = justCalculated && Expr.Box.SelectionLength == Expr.Box.TextLength && Expr.Box.TextLength > 0;
            justCalculated = false;
            Expr.InsertFunction(name, wrapAll, extra);
            if (name.Length > 0) main.AddRecent(settings.RecentFunctions, name);
        }

        public void ClearExpression() { Expr.SetText(""); Result.Clear(); SetStatus("", false); Expr.Box.Focus(); }
        public void Backspace() => Expr.Backspace();
        public void DeleteForward() => Expr.DeleteForward();
        public void MoveCaret(int delta, bool absolute = false) => Expr.MoveCaret(delta, absolute);

        public void Negate()
        {
            if (settings.RpnMode && Expr.Box.TextLength == 0 && Stack.Count > 0) { StackOp("negate"); return; }
            var t = Expr.Box.Text;
            if (t.StartsWith("-(") && t.EndsWith(")")) Expr.SetText(t.Substring(2, t.Length - 3));
            else if (t.Length > 0) Expr.SetText("-(" + t + ")");
            else Expr.Insert("-");
            Expr.Box.Focus();
        }

        Expr CurrentValue() => Last?.Value;

        public void MemoryStore()
        {
            var v = CurrentValue();
            if (v == null) return;
            svc.Run(() => { calc.Memory = v; return true; });
            SetStatus("Stored in memory", false);
        }

        public void MemoryRecall() => Insert("MR");
        public void MemoryClear() { svc.Run(() => { calc.Memory = null; return true; }); SetStatus("Memory cleared", false); }

        public void MemoryAdd(bool subtract)
        {
            var v = CurrentValue();
            if (v == null) return;
            svc.Run(() =>
            {
                var cur = calc.Memory ?? Engine.Expr.Zero;
                calc.Memory = calc.Run(subtract ? Ops.Sub(cur, v) : Ops.Add(cur, v));
                return true;
            });
            SetStatus(subtract ? "Subtracted from memory" : "Added to memory", false);
        }

        public void StoreResult()
        {
            var v = CurrentValue();
            if (v == null) { main.RunAction("newvariable"); return; }
            var name = InputDialog.Ask(main, "Store Result", "Variable name:", "");
            if (string.IsNullOrWhiteSpace(name)) return;
            name = name.Trim();
            if (!Lexer.IsIdentStart(name[0]) || !name.All(Lexer.IsIdentPart)) { Ui.Message(main, "Invalid variable name."); return; }
            svc.Run(() =>
            {
                var var = new Variable { Name = name, Names = { name }, Value = v, ValueText = Formatter.Ascii(v, calc.Opts), IsUser = true, Category = "User variables", Title = name };
                calc.AddVariable(var);
                return true;
            }).ContinueWith(t => BeginInvoke((Action)(() => { main.DefinitionsChanged(); SetStatus("Stored as " + name, false); })));
        }

        public void ShowUnitsMenu(Control anchor) => main.BuildUnitsMenu(u => { settings.LastUnitButton = u; Keypad.RefreshUnitKey(); Insert(u); }).Show(anchor, new Point(0, anchor.Height));
        public void ShowFunctionsMenu(Control anchor) => main.BuildFunctionsMenu().Show(anchor, new Point(0, anchor.Height));
        public void ShowVariablesMenu(Control anchor) => main.BuildVariablesMenu(v => Insert(v)).Show(anchor, new Point(0, anchor.Height));
        public void ShowConversionMenu(Control anchor) => main.BuildConversionMenu(t => ConvertTo(t)).Show(anchor, new Point(0, anchor.Height));
        public string LastUnit => string.IsNullOrEmpty(settings.LastUnitButton) ? "m" : settings.LastUnitButton;

        public int InputBase => calc.Opts.InputBase;
        public int OutputBase => calc.Opts.OutputBase;
        public void SetInputBase(int b) { calc.Opts.InputBase = b; main.OptionsChanged(); }
        public void SetOutputBase(int b) { calc.Opts.OutputBase = b; main.OptionsChanged(); if (Last != null && Expr.Box.TextLength > 0) Calculate(lastCalcText ?? Expr.Box.Text, false); }

        public void OpenMatrixEditor()
        {
            using (var d = new MatrixEditDialog(Last?.Value as MatE))
                if (d.ShowDialog(main) == DialogResult.OK) Insert(d.ResultText);
        }

        public void InsertDate()
        {
            using (var d = new DateDialog("Insert Date", DateTime.Now, true))
                if (d.ShowDialog(main) == DialogResult.OK)
                    Insert(d.Value.TimeOfDay.Ticks == 0 ? d.Value.ToString("yyyy-MM-dd") : d.Value.ToString("yyyy-MM-ddTHH:mm:ss"));
        }

        public void RunAction(string id) => main.RunAction(id);

        public void ClearStack() { StackOp("clear"); }

        // rpn stack operations
        void RpnEnter()
        {
            var text = Expr.Box.Text.Trim();
            if (text.Length == 0)
            {
                if (Stack.Count > 0) StackOp("copy");
                return;
            }
            svc.Run(() => Display.From(calc.Calculate(text), calc)).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                var d = t.Result;
                if (d.Error != null) { SetStatus(d.Error, true); Result.ShowError(d.ParsedText, d.Error); return; }
                if (d.Value != null) { Stack.Add(d.Value); Rpn.Refill(); }
                ShowDisplay(d, true);
                Expr.SetText("");
                SelectSide(1);
            })));
        }

        void RpnOp(string op)
        {
            var pending = Expr.Box.Text.Trim();
            var st = Stack.ToList();
            svc.Run(() =>
            {
                if (pending.Length > 0)
                {
                    var r = calc.Calculate(pending);
                    if (r.Error != null) return Display.From(r, calc);
                    st.Add(Funcs.Unhold(r.Result));
                }
                int needs = (op == "^2" || op == "^3" || op == "^-1" || op == "!") ? 1 : 2;
                if (st.Count < needs) return new Display { Error = "Not enough values on the RPN stack" };
                Expr b = st[st.Count - 1];
                Expr res;
                if (needs == 1)
                {
                    st.RemoveAt(st.Count - 1);
                    lastX = b;
                    Expr raw = op == "!" ? new CallE(calc.Fn("factorial"), b) : Ops.Pow(b, op == "^2" ? Engine.Expr.Two : op == "^3" ? Engine.Expr.N(3) : Engine.Expr.MinusOne);
                    res = calc.Run(raw);
                }
                else
                {
                    var a = st[st.Count - 2];
                    st.RemoveRange(st.Count - 2, 2);
                    lastX = b;
                    Expr raw;
                    switch (op)
                    {
                        case "+": raw = Ops.Add(a, b); break;
                        case "-": raw = Ops.Sub(a, b); break;
                        case "*": raw = Ops.Mul(a, b); break;
                        case "/": raw = Ops.Div(a, b); break;
                        case "^": raw = Ops.Pow(a, b); break;
                        case "&": raw = new CallE(calc.Fn("bitand"), a, b); break;
                        case "|": raw = new CallE(calc.Fn("bitor"), a, b); break;
                        case " xor ": raw = new CallE(calc.Fn("xor"), a, b); break;
                        case "<<": raw = new CallE(calc.Fn("shift"), a, b); break;
                        case ">>": raw = new CallE(calc.Fn("shift"), a, Ops.Neg(b)); break;
                        case " mod ": raw = new CallE(calc.Fn("mod"), a, b); break;
                        case "//": raw = new CallE(calc.Fn("div"), a, b); break;
                        default: raw = Ops.Add(a, b); break;
                    }
                    try { res = calc.Run(raw); }
                    catch (CalcException ex) { st.Add(a); st.Add(b); return new Display { Error = ex.Message }; }
                }
                st.Add(Funcs.Unhold(res));
                calc.Answers.Insert(0, res);
                var cr = new CalcResult { Input = "RPN " + op.Trim(), Result = res, Print = calc.Opts.Clone() };
                return Display.From(cr, calc);
            }).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                var d = t.Result;
                Expr.SetText("");
                Stack.Clear(); Stack.AddRange(st);
                Rpn.Refill();
                if (d.Error != null) { SetStatus(d.Error, true); return; }
                Last = d;
                Result.ShowResult(d.ParsedText, d.Tokens, d.Approx, d.ExactTokens, d.Matrix);
                SelectSide(1);
            })));
        }

        void RpnFunction(string name, string extra)
        {
            var f = calc.FindFunction(name);
            if (f == null) return;
            var st = Stack.ToList();
            svc.Run(() =>
            {
                int n = Math.Max(1, Math.Min(f.MinArgs, st.Count));
                if (st.Count < n) return new Display { Error = "Not enough values on the RPN stack" };
                var args = st.Skip(st.Count - n).ToList();
                st.RemoveRange(st.Count - n, n);
                lastX = args.Last();
                try
                {
                    var res = Funcs.Unhold(calc.Run(new CallE(f, args)));
                    st.Add(res);
                    return Display.From(new CalcResult { Input = name, Result = res, Print = calc.Opts.Clone() }, calc);
                }
                catch (CalcException ex) { st.AddRange(args); return new Display { Error = ex.Message }; }
            }).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                Stack.Clear(); Stack.AddRange(st);
                Rpn.Refill();
                var d = t.Result;
                if (d.Error != null) { SetStatus(d.Error, true); return; }
                Last = d;
                Result.ShowResult(d.ParsedText, d.Tokens, d.Approx, null, d.Matrix);
            })));
        }

        public void StackOp(string op)
        {
            int sel = Rpn.SelectedRegister;
            int n = Stack.Count;
            switch (op)
            {
                case "clear": Stack.Clear(); break;
                case "copy":
                    if (n == 0) return;
                    Stack.Add(sel >= 0 ? Stack[n - 1 - sel] : Stack[n - 1]); break;
                case "delete":
                    if (n == 0) return;
                    Stack.RemoveAt(sel >= 0 ? n - 1 - sel : n - 1); break;
                case "swap":
                    if (n < 2) return;
                    if (sel > 0) { var t = Stack[n - 1 - sel]; Stack.RemoveAt(n - 1 - sel); Stack.Add(t); }
                    else { var t = Stack[n - 1]; Stack[n - 1] = Stack[n - 2]; Stack[n - 2] = t; }
                    break;
                case "up":
                    if (n < 2) return;
                    if (sel >= 0 && sel < n - 1) { int i = n - 1 - sel; var t = Stack[i]; Stack[i] = Stack[i - 1]; Stack[i - 1] = t; }
                    else { var t = Stack[0]; Stack.RemoveAt(0); Stack.Add(t); }
                    break;
                case "down":
                    if (n < 2) return;
                    if (sel > 0) { int i = n - 1 - sel; var t = Stack[i]; Stack[i] = Stack[i + 1]; Stack[i + 1] = t; }
                    else { var t = Stack[n - 1]; Stack.RemoveAt(n - 1); Stack.Insert(0, t); }
                    break;
                case "lastx": if (lastX != null) Stack.Add(lastX); break;
                case "negate":
                    if (n == 0) return;
                    {
                        var v = Stack[n - 1];
                        svc.Run(() => calc.Run(Ops.Neg(v))).ContinueWith(t => BeginInvoke((Action)(() => { if (!t.IsFaulted) { Stack[Stack.Count - 1] = t.Result; Rpn.Refill(); } })));
                        return;
                    }
            }
            Rpn.Refill();
        }

        void EditRegister(int idx, string text)
        {
            svc.Run(() => calc.Calculate(text)).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                var r = t.Result;
                if (r.Error != null) { SetStatus(r.Error, true); return; }
                int pos = Stack.Count - 1 - idx;
                if (pos >= 0 && pos < Stack.Count) Stack[pos] = Funcs.Unhold(r.Result);
                Rpn.Refill();
            })));
}   }   } 