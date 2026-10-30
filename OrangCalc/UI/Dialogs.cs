using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.Linq;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public class DialogBase : Form
    {
        protected readonly TableLayoutPanel Body;
        protected readonly FlowLayoutPanel Buttons;

        public DialogBase(string title, int width = 460)
        {
            Text = title;
            Theme.PrepareForm(this);
            FormBorderStyle = FormBorderStyle.FixedDialog;
            MaximizeBox = false; MinimizeBox = false;
            ShowInTaskbar = false;
            StartPosition = FormStartPosition.CenterParent;
            Width = width;
            AutoSize = true;
            AutoSizeMode = AutoSizeMode.GrowAndShrink;
            KeyPreview = true;
            Icon = Theme.AppIcon;
            var stack = new TableLayoutPanel { AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink, ColumnCount = 1, Location = Point.Empty, Margin = new Padding(0), BackColor = Theme.Window };
            Body = new TableLayoutPanel { AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink, ColumnCount = 2, Padding = new Padding(16, 14, 16, 6), Margin = new Padding(0), BackColor = Theme.Window };
            Body.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            Body.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, width - 170));
            Buttons = new FlowLayoutPanel { AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink, WrapContents = false, FlowDirection = FlowDirection.RightToLeft, Padding = new Padding(12, 6, 12, 12), Margin = new Padding(0), BackColor = Theme.Window, Anchor = AnchorStyles.Right };
            stack.Controls.Add(Body, 0, 0);
            stack.Controls.Add(Buttons, 0, 1);
            Controls.Add(stack);
            KeyDown += (s, e) => { if (e.KeyCode == Keys.Escape) { DialogResult = DialogResult.Cancel; Close(); } };
        }

        protected int row;

        protected T AddRow<T>(string label, T c) where T : Control
        {
            if (label != null)
            {
                Body.Controls.Add(new Label { Text = label, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 7, 12, 3) }, 0, row);
                c.Dock = DockStyle.Fill;
                c.Margin = new Padding(0, 3, 0, 3);
                Body.Controls.Add(c, 1, row);
            }
            else
            {
                Body.Controls.Add(c, 0, row);
                Body.SetColumnSpan(c, 2);
            }
            row++;
            return c;
        }

        protected FlatButton AddButton(string text, Action onClick, bool primary = false)
        {
            var b = new FlatButton(text, (s, e) => onClick(), primary) { Margin = new Padding(6, 0, 0, 0) };
            Buttons.Controls.Add(b);
            return b;
        }

        protected void OkCancel(Func<bool> onOk, string okText = "OK")
        {
            AddButton("Cancel", () => { DialogResult = DialogResult.Cancel; Close(); });
            AddButton(okText, () => { if (onOk()) { DialogResult = DialogResult.OK; Close(); } }, true);
        }

        protected void Fail(string msg) => Ui.Message(this, msg, "Error");
    }

    public sealed class MessageDialog : DialogBase
    {
        MessageDialog(string text, string title, bool question) : base(title, 420)
        {
            var l = new Label { Text = text, AutoSize = true, MaximumSize = new Size(380, 0), ForeColor = Theme.Text, Margin = new Padding(0, 4, 0, 10) };
            AddRow(null, l);
            if (question)
            {
                AddButton("No", () => { DialogResult = DialogResult.No; Close(); });
                AddButton("Yes", () => { DialogResult = DialogResult.Yes; Close(); }, true);
            }
            else AddButton("Close", () => { DialogResult = DialogResult.OK; Close(); }, true);
        }

        public static bool Show(IWin32Window owner, string text, string title, bool question)
        {
            using (var d = new MessageDialog(text, title, question)) return d.ShowDialog(owner) == DialogResult.Yes;
        }
    }

    public sealed class InputDialog : DialogBase
    {
        readonly TextBox box;
        InputDialog(string title, string prompt, string value, bool multiline) : base(title, 460)
        {
            AddRow(null, new Label { Text = prompt, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 2, 0, 4) });
            box = Ui.Box(value, 420);
            if (multiline) { box.Multiline = true; box.Height = 80; box.ScrollBars = ScrollBars.Vertical; }
            AddRow(null, box);
            box.Width = 420;
            OkCancel(() => true);
            AcceptButton = null;
            box.KeyDown += (s, e) => { if (e.KeyCode == Keys.Enter && (!multiline || e.Control)) { DialogResult = DialogResult.OK; Close(); } };
            Shown += (s, e) => { box.Focus(); box.SelectAll(); };
        }

        public static string Ask(IWin32Window owner, string title, string prompt, string value = "", bool multiline = false)
        {
            using (var d = new InputDialog(title, prompt, value, multiline)) return d.ShowDialog(owner) == DialogResult.OK ? d.box.Text : null;
        }
    }

    public sealed class DateDialog : DialogBase
    {
        readonly MonthCalendar cal;
        readonly DateTimePicker time;
        public DateTime Value { get; private set; }

        public DateDialog(string title, DateTime initial, bool withTime) : base(title, 300)
        {
            cal = new MonthCalendar { MaxSelectionCount = 1, SelectionStart = initial, BackColor = Theme.Raised, ForeColor = Theme.Text, TitleBackColor = Theme.Accent, TitleForeColor = Theme.AccentText, TrailingForeColor = Theme.TextFaint };
            Theme.Unthemed(cal);
            AddRow(null, cal);
            if (withTime)
            {
                time = new DateTimePicker { Format = DateTimePickerFormat.Time, ShowUpDown = true, Value = initial };
                AddRow("Time:", time);
            }
            OkCancel(() =>
            {
                Value = cal.SelectionStart.Date + (time != null ? time.Value.TimeOfDay : TimeSpan.Zero);
                return true;
            });
        }
    }

    public sealed class CustomButtonDialog : DialogBase
    {
        readonly TextBox label, a1, a2, a3;
        public string[] Values;

        public CustomButtonDialog(string[] cur) : base("Button Action", 480)
        {
            label = AddRow("Label:", Ui.Box(cur.Length > 0 ? cur[0] : ""));
            a1 = AddRow("Click:", Ui.Box(cur.Length > 1 ? cur[1] : ""));
            a2 = AddRow("Right-click:", Ui.Box(cur.Length > 2 ? cur[2] : ""));
            a3 = AddRow("Middle-click:", Ui.Box(cur.Length > 3 ? cur[3] : ""));
            AddRow(null, new Label { Text = "Text is inserted into the expression. Use name() to insert a function, or @action for an action\n(for example @calculate, @clear, @plot, @periodic, @copy).", AutoSize = true, MaximumSize = new Size(440, 0), ForeColor = Theme.TextFaint, Margin = new Padding(0, 6, 0, 4) });
            AddButton("Cancel", () => { DialogResult = DialogResult.Cancel; Close(); });
            AddButton("Remove", () => { DialogResult = DialogResult.Abort; Close(); });
            AddButton("Save", () =>
            {
                if (label.Text.Trim().Length == 0 || a1.Text.Length == 0) { Fail("A label and a click action are required."); return; }
                Values = new[] { label.Text.Trim(), a1.Text, a2.Text, a3.Text };
                DialogResult = DialogResult.OK; Close();
            }, true);
        }
    }

    public sealed class FunctionEditDialog : DialogBase
    {
        readonly Calculator calc;
        readonly TextBox name, title, category, formula, args, description, condition, example;
        public MathFunction Result;

        public FunctionEditDialog(Calculator c, MathFunction f, string initialFormula = "") : base(f == null ? "New Function" : "Edit Function", 560)
        {
            calc = c;
            name = AddRow("Name:", Ui.Box(f?.Names[0] ?? ""));
            title = AddRow("Title:", Ui.Box(f?.Title ?? ""));
            category = AddRow("Category:", Ui.Box(f?.Category ?? "User functions"));
            formula = AddRow("Expression:", Ui.Box(f?.Formula ?? initialFormula));
            formula.Font = Theme.MonoFont;
            args = AddRow("Argument names:", Ui.Box(f != null ? string.Join(", ", f.Args.Select(a => a.Name)) : "x"));
            condition = AddRow("Condition:", Ui.Box(f?.Condition ?? ""));
            example = AddRow("Example:", Ui.Box(f?.Example ?? ""));
            description = AddRow("Description:", Ui.Box(f?.Description ?? ""));
            description.Multiline = true; description.Height = 64;
            AddRow(null, new Label { Text = "Refer to arguments with \\x, \\y, \\z, then \\a, \\b … in the expression, e.g. \\x^2 + \\y.", AutoSize = true, ForeColor = Theme.TextFaint, Margin = new Padding(0, 4, 0, 2) });
            if (f != null && !f.IsUser) { formula.ReadOnly = true; name.ReadOnly = true; }
            OkCancel(() =>
            {
                var n = name.Text.Trim();
                if (n.Length == 0 || !Lexer.IsIdentStart(n[0]) || !n.All(Lexer.IsIdentPart)) { Fail("Invalid name."); return false; }
                if (f != null && !f.IsUser) { f.Title = title.Text; f.Category = category.Text; f.Description = description.Text; Result = f; return true; }
                try
                {
                    var an = args.Text.Split(',').Select(a => a.Trim()).Where(a => a.Length > 0).ToList();
                    var nf = UserItems.MakeFunction(calc, n, formula.Text.Trim(), an);
                    nf.Title = title.Text.Trim().Length > 0 ? title.Text.Trim() : n;
                    nf.Category = category.Text.Trim().Length > 0 ? category.Text.Trim() : "User functions";
                    nf.Description = description.Text; nf.Condition = condition.Text; nf.Example = example.Text;
                    if (f != null && f.IsUser) calc.RemoveFunction(f);
                    calc.AddFunction(nf);
                    Result = nf;
                    return true;
                }
                catch (CalcException ex) { Fail(ex.Message); return false; }
            });
        }
    }

    public sealed class VariableEditDialog : DialogBase
    {
        public Variable Result;

        public VariableEditDialog(Calculator c, Variable v, string initial = "") : base(v == null ? "New Variable" : "Edit Variable", 520)
        {
            var name = AddRow("Name:", Ui.Box(v?.Names[0] ?? ""));
            var value = AddRow("Value:", Ui.Box(v != null ? (v.ValueText.Length > 0 ? v.ValueText : (v.Value != null ? Formatter.Ascii(v.Value, c.Opts) : "")) : initial));
            value.Font = Theme.MonoFont;
            var title = AddRow("Title:", Ui.Box(v?.Title ?? ""));
            var category = AddRow("Category:", Ui.Box(v?.Category ?? "User variables"));
            var description = AddRow("Description:", Ui.Box(v?.Description ?? ""));
            if (v != null && !v.IsUser) { name.ReadOnly = true; value.ReadOnly = true; }
            OkCancel(() =>
            {
                var n = name.Text.Trim();
                if (n.Length == 0 || !Lexer.IsIdentStart(n[0]) || !n.All(Lexer.IsIdentPart)) { Fail("Invalid name."); return false; }
                if (v != null && !v.IsUser) { v.Title = title.Text; v.Category = category.Text; v.Description = description.Text; Result = v; return true; }
                try
                {
                    var nv = UserItems.MakeVariable(c, n, value.Text.Trim(), title.Text.Trim(), category.Text.Trim().Length > 0 ? category.Text.Trim() : "User variables");
                    nv.Value = Funcs.Unhold(nv.Value);
                    nv.Description = description.Text;
                    if (v != null) c.RemoveVariable(v);
                    c.AddVariable(nv);
                    Result = nv;
                    return true;
                }
                catch (CalcException ex) { Fail(ex.Message); return false; }
            });
        }
    }

    public sealed class UnknownEditDialog : DialogBase
    {
        public Variable Result;

        public UnknownEditDialog(Calculator c, Variable v) : base(v == null ? "New Unknown Variable" : "Edit Unknown Variable", 460)
        {
            var name = AddRow("Name:", Ui.Box(v?.Name ?? ""));
            var type = AddRow("Type:", Ui.Combo(Enum.GetNames(typeof(AssumeType)), (int)(v?.AType ?? AssumeType.Real)));
            var sign = AddRow("Sign:", Ui.Combo(Enum.GetNames(typeof(AssumeSign)).Select(ExpressionBox.Words), (int)(v?.ASign ?? AssumeSign.Unknown)));
            var title = AddRow("Title:", Ui.Box(v?.Title ?? ""));
            AddRow(null, new Label { Text = "Unknown variables are kept symbolic. Assumptions are used when simplifying, e.g. sqrt(x^2) = x for positive x.", AutoSize = true, MaximumSize = new Size(420, 0), ForeColor = Theme.TextFaint, Margin = new Padding(0, 6, 0, 2) });
            OkCancel(() =>
            {
                var n = name.Text.Trim();
                if (n.Length == 0 || !Lexer.IsIdentStart(n[0]) || !n.All(Lexer.IsIdentPart)) { Fail("Invalid name."); return false; }
                var nv = v != null && v.Name == n ? v : new Variable { Name = n, Names = { n }, IsUnknown = true, IsUser = true, Category = "Unknowns" };
                nv.IsUser = true;
                nv.AType = (AssumeType)type.SelectedIndex;
                nv.ASign = (AssumeSign)sign.SelectedIndex;
                nv.Title = title.Text.Trim().Length > 0 ? title.Text.Trim() : n;
                if (v != null && v != nv) c.RemoveVariable(v);
                c.AddVariable(nv);
                Result = nv;
                return true;
            });
        }
    }

    public sealed class UnitEditDialog : DialogBase
    {
        public Unit Result;

        public UnitEditDialog(Calculator c, Unit u) : base(u == null ? "New Unit" : "Edit Unit", 540)
        {
            var name = AddRow("Abbreviation:", Ui.Box(u?.Name ?? ""));
            var names = AddRow("Other names:", Ui.Box(u != null ? string.Join(", ", u.Names.Skip(1)) : ""));
            var plural = AddRow("Plural:", Ui.Box(u?.Plural ?? ""));
            var def = AddRow("Equals:", Ui.Box(u?.Definition ?? "1 m"));
            def.Font = Theme.MonoFont;
            var title = AddRow("Title:", Ui.Box(u?.Title ?? ""));
            var category = AddRow("Category:", Ui.Box(u?.Category ?? "User units"));
            var pref = AddRow("Prefixes:", Ui.Combo(new[] { "None", "Some", "All", "Binary" }, (int)(u?.Prefixes ?? PrefixUse.None)));
            AddRow(null, new Label { Text = "Define the unit by its value in other units, for example \"0.3048 m\" or \"1 kg*m/s^2\".", AutoSize = true, MaximumSize = new Size(500, 0), ForeColor = Theme.TextFaint, Margin = new Padding(0, 6, 0, 2) });
            if (u != null && !u.IsUser) { name.ReadOnly = true; def.ReadOnly = true; }
            OkCancel(() =>
            {
                var n = name.Text.Trim();
                if (n.Length == 0) { Fail("A name is required."); return false; }
                if (u != null && !u.IsUser) { u.Title = title.Text; u.Category = category.Text; Result = u; return true; }
                try
                {
                    var nu = UserItems.MakeUnit(c, n, names.Text.Split(',').Select(x => x.Trim()), plural.Text.Trim(), def.Text.Trim(), title.Text.Trim(), category.Text.Trim(), (PrefixUse)pref.SelectedIndex);
                    if (u != null) c.RemoveUnit(u);
                    c.AddUnit(nu);
                    Result = nu;
                    return true;
                }
                catch (CalcException ex) { Fail(ex.Message); return false; }
            });
        }
    }

    public sealed class MatrixEditDialog : DialogBase
    {
        readonly DataGridView grid;
        readonly NumericUpDown rows, cols;
        readonly CheckBox vector;
        public string ResultText;

        public MatrixEditDialog(MatE initial) : base("Matrix", 620)
        {
            int r0 = initial?.RowCount ?? 3, c0 = initial?.ColCount ?? 3;
            rows = Ui.Number(r0, 1, 100, 70); cols = Ui.Number(c0, 1, 100, 70);
            vector = Ui.Check("Vector", initial != null && initial.IsVector);
            var top = Ui.Row(Ui.Label("Rows:", true), rows, Ui.Label("Columns:", true), cols, vector);
            AddRow(null, top);
            grid = new DataGridView { Width = 580, Height = 280, AllowUserToAddRows = false, AllowUserToDeleteRows = false, ColumnHeadersVisible = true, SelectionMode = DataGridViewSelectionMode.CellSelect };
            Theme.StyleGrid(grid);
            grid.DefaultCellStyle.Font = Theme.MonoFont;
            AddRow(null, grid);
            ResizeGrid(r0, c0);
            if (initial != null)
                for (int i = 0; i < initial.RowCount; i++)
                    for (int j = 0; j < initial.ColCount; j++)
                        grid.Rows[i].Cells[j].Value = Formatter.PlainText(initial.Rows[i][j]);
            rows.ValueChanged += (s, e) => ResizeGrid((int)rows.Value, (int)cols.Value);
            cols.ValueChanged += (s, e) => ResizeGrid((int)rows.Value, (int)cols.Value);
            vector.CheckedChanged += (s, e) => { if (vector.Checked) rows.Value = 1; };
            OkCancel(() =>
            {
                var rowsText = new List<string>();
                for (int i = 0; i < grid.RowCount; i++)
                {
                    var cells = new List<string>();
                    for (int j = 0; j < grid.ColumnCount; j++)
                    {
                        var v = grid.Rows[i].Cells[j].Value?.ToString().Trim();
                        cells.Add(string.IsNullOrEmpty(v) ? "0" : v);
                    }
                    rowsText.Add(string.Join(", ", cells));
                }
                ResultText = vector.Checked || grid.RowCount == 1 && vector.Checked ? "[" + rowsText[0] + "]" : "[" + string.Join("; ", rowsText) + "]";
                if (vector.Checked && grid.RowCount > 1) ResultText = "[" + string.Join(", ", rowsText) + "]";
                return true;
            }, "Insert");
        }

        void ResizeGrid(int r, int c)
        {
            while (grid.ColumnCount < c) grid.Columns.Add("c" + grid.ColumnCount, (grid.ColumnCount + 1).ToString());
            while (grid.ColumnCount > c) grid.Columns.RemoveAt(grid.ColumnCount - 1);
            foreach (DataGridViewColumn col in grid.Columns) { col.Width = 80; col.SortMode = DataGridViewColumnSortMode.NotSortable; }
            while (grid.RowCount < r) grid.Rows.Add();
            while (grid.RowCount > r) grid.Rows.RemoveAt(grid.RowCount - 1);
        }
    }

    // argument entry for inserting a function call
    public sealed class InsertFunctionDialog : DialogBase
    {
        readonly List<TextBox> boxes = new List<TextBox>();
        public string CallText;
        public bool CalculateNow;

        public InsertFunctionDialog(Calculator c, MathFunction f, string firstArg) : base(f.DisplayTitle, 520)
        {
            AddRow(null, new Label { Text = f.Signature, AutoSize = true, Font = Theme.MonoFont, ForeColor = Theme.TokFunction, Margin = new Padding(0, 2, 0, 6) });
            if (f.Description.Length > 0) AddRow(null, new Label { Text = f.Description, AutoSize = true, MaximumSize = new Size(480, 0), ForeColor = Theme.TextDim, Margin = new Padding(0, 0, 0, 8) });
            int n = f.MaxArgs < 0 ? Math.Max(f.Args.Count, f.MinArgs) : f.Args.Count;
            for (int i = 0; i < n; i++)
            {
                var a = f.Args[Math.Min(i, f.Args.Count - 1)];
                string def = i == 0 && !string.IsNullOrEmpty(firstArg) ? firstArg : (a.Default ?? "");
                if (def == "\"\"") def = "";
                var b = AddRow(Capital(a.Name.Replace('_', ' ')) + (i >= f.MinArgs ? " (optional)" : "") + ":", Ui.Box(def));
                b.Font = Theme.MonoFont;
                boxes.Add(b);
                b.KeyDown += (s, e) => { if (e.KeyCode == Keys.Enter) { Build(); CalculateNow = true; DialogResult = DialogResult.OK; Close(); } };
            }
            if (f.Example.Length > 0) AddRow(null, new Label { Text = "Example: " + f.Example, AutoSize = true, ForeColor = Theme.TextFaint });
            AddButton("Cancel", () => { DialogResult = DialogResult.Cancel; Close(); });
            AddButton("Insert", () => { Build(); DialogResult = DialogResult.OK; Close(); });
            AddButton("Calculate", () => { Build(); CalculateNow = true; DialogResult = DialogResult.OK; Close(); }, true);
            Shown += (s, e) => { if (boxes.Count > 0) { boxes[0].Focus(); boxes[0].SelectAll(); } };

            void Build()
            {
                var vals = boxes.Select(b => b.Text.Trim()).ToList();
                while (vals.Count > 0 && vals[vals.Count - 1].Length == 0 && vals.Count > f.MinArgs) vals.RemoveAt(vals.Count - 1);
                CallText = f.Names[0] + "(" + string.Join(c.Opts.CommaDecimal ? "; " : ", ", vals) + ")";
            }
        }

        static string Capital(string s) => s.Length == 0 ? s : char.ToUpperInvariant(s[0]) + s.Substring(1);
    }

    public sealed class ShortcutDialog : DialogBase
    {
        readonly Label keys;
        public string Value = "";

        public ShortcutDialog(string action, string current) : base("Keyboard Shortcut", 400)
        {
            AddRow(null, new Label { Text = "Press the key combination you wish to use for:\n" + action, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 2, 0, 8) });
            keys = AddRow(null, new Label { Text = string.IsNullOrEmpty(current) ? "(none)" : current, AutoSize = false, Height = 40, Width = 360, TextAlign = ContentAlignment.MiddleCenter, Font = Theme.HeaderFont, BackColor = Theme.Raised, ForeColor = Theme.Text });
            Value = current ?? "";
            KeyDown += (s, e) =>
            {
                if (e.KeyCode == Keys.ControlKey || e.KeyCode == Keys.ShiftKey || e.KeyCode == Keys.Menu || e.KeyCode == Keys.Escape) return;
                Value = new KeysConverter().ConvertToInvariantString(e.KeyData);
                keys.Text = Value;
                e.Handled = e.SuppressKeyPress = true;
            };
            AddButton("Cancel", () => { DialogResult = DialogResult.Cancel; Close(); });
            AddButton("Clear", () => { Value = ""; keys.Text = "(none)"; });
            AddButton("Set", () => { DialogResult = DialogResult.OK; Close(); }, true);
            foreach (Control ctl in Buttons.Controls) ctl.TabStop = false;
        }

        protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
        {
            if ((keyData & Keys.KeyCode) == Keys.Tab || (keyData & Keys.KeyCode) == Keys.Enter || (keyData & Keys.KeyCode) == Keys.Space)
            {
                OnKeyDown(new KeyEventArgs(keyData));
                return true;
            }
            return base.ProcessCmdKey(ref msg, keyData);
        }
    }

    public sealed class AboutDialog : DialogBase
    {
        public AboutDialog() : base("About OrangCalc", 440)
        {
            var v = typeof(AboutDialog).Assembly.GetName().Version;
            AddRow(null, new Label { Text = "OrangCalc", Font = new Font("Segoe UI Semibold", 20f), AutoSize = true, ForeColor = Theme.Accent, Margin = new Padding(0, 4, 0, 0) });
            AddRow(null, new Label { Text = "Version " + v.Major + "." + v.Minor + "." + v.Build, AutoSize = true, ForeColor = Theme.TextDim });
            AddRow(null, new Label { Text = "A dark, calculator with arithmetic, symbolic algebra, units and currencies, plotting, data sets and a periodic table.", AutoSize = true, MaximumSize = new Size(400, 0), ForeColor = Theme.Text, Margin = new Padding(0, 10, 0, 6) });
            AddRow(null, new Label { Text = "License: GNU GPL ver 3\n(c) Orang Studio 2026", AutoSize = true, ForeColor = Theme.TextFaint, Margin = new Padding(0, 6, 0, 4) });
            AddButton("Close", () => Close(), true);
        }
    }

    public sealed class HelpDialog : Form
    {
        public HelpDialog()
        {
            Text = "OrangCalc Help";
            Theme.PrepareForm(this);
            Icon = Theme.AppIcon;
            Size = new Size(760, 640);
            StartPosition = FormStartPosition.CenterParent;
            var box = new RichTextBox { Dock = DockStyle.Fill, ReadOnly = true, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text, Font = Theme.UiFont, DetectUrls = false };
            Theme.Style(box);
            box.BackColor = Theme.Panel;
            var wrap = new Panel { Dock = DockStyle.Fill, Padding = new Padding(16), BackColor = Theme.Panel };
            wrap.Controls.Add(box);
            Controls.Add(wrap);
            foreach (var (head, body) in HelpText.Sections)
            {
                box.SelectionFont = Theme.HeaderFont; box.SelectionColor = Theme.Accent;
                box.AppendText(head + "\n");
                box.SelectionFont = Theme.UiFont; box.SelectionColor = Theme.Text;
                box.AppendText(body + "\n\n");
            }
            box.SelectionStart = 0;
        }
    }

    public static class HelpText
    {
        public static readonly (string, string)[] Sections =
        {
            ("Basics", "Type an expression and press Enter. Shift+Enter inserts a new line. Up and Down browse previous expressions. Ctrl+Space shows completion.\nOperators: + − × ÷ ^, implicit multiplication (2x, 5 m), ! factorial, % percent, mod, rem, // integer division.\nComparisons and logic: = ≠ < > ≤ ≥, and, or, xor, not, && ||. Bitwise: & | ~ << >> xor."),
            ("Numbers", "Decimal 1.5, scientific 1.5E3, hexadecimal 0xFF, binary 0b1010, octal 0o17, sexagesimal 5°30′15″, time 10:30, dates 2024-01-31, fractions ½, uncertainty 5±0.1 or interval(4.9, 5.1), complex 3+4i or 5∠30°."),
            ("Conversion", "Append \"to\" (or →) with a target: units (5 ft to m), bases (to hex, to bin, to oct, to base 7, to roman, to bijective, to bcd), formats (to fraction, to mixed, to scientific, to engineering, to factors, to partial fraction, to polar, to cis, to angle), time zones (to utc, to utc+3), to calendars, to bases, to fp32 / fp64."),
            ("Variables and functions", "Assign with name := value. Define functions with f(x) := x^2 + 1. Unknowns (x, y, z) are kept symbolic. ans is the last result. Substitute with \"where\": x^2 + y where x = 2, y = 3."),
            ("Algebra and calculus", "Equations with one unknown are solved automatically: x^2 = 4. Functions: solve, multisolve, factor, expand, partfrac, diff, integrate, limit, sum, product, taylor, dsolve, extremum."),
            ("Units", "Units and prefixes are written directly: 5 km/h, 3 kWh, 100 °F to °C. Currencies update from the European Central Bank. Physical constants carry units (c, planck, k_B)."),
            ("Modes", "Approximation: automatic, dual (exact and approximate), exact, approximate. Angle unit: radians, degrees, gradians. Parsing: adaptive, conventional, implicit multiplication first, chain mode and RPN mode."),
            ("Keypad", "Each key has up to three actions: click, right-click and middle-click. The small hint on a key shows the right-click action. The Custom keypad can be filled with your own buttons."),
        };
    }
}