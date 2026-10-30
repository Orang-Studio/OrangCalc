using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace OrangCalc.UI
{
    public interface IKeypadHost
    {
        void Insert(string text, int caretBack = 0);
        void InsertFunction(string name, string extra = "");
        void InsertOperator(string op);
        void Calculate();
        void ClearExpression();
        void Backspace();
        void DeleteForward();
        void MoveCaret(int delta, bool absolute = false);
        void MemoryStore();
        void MemoryRecall();
        void MemoryClear();
        void MemoryAdd(bool subtract);
        void StoreResult();
        void ShowUnitsMenu(Control anchor);
        void ShowFunctionsMenu(Control anchor);
        void ShowVariablesMenu(Control anchor);
        void ShowConversionMenu(Control anchor);
        void SetInputBase(int b);
        void SetOutputBase(int b);
        int InputBase { get; }
        int OutputBase { get; }
        void OpenMatrixEditor();
        void RunAction(string id);
        void ClearStack();
        void Negate();
        string LastUnit { get; }
    }

    public sealed class KeypadPanel : Panel
    {
        readonly IKeypadHost host;
        readonly AppSettings settings;
        readonly SegmentBar tabs = new SegmentBar();
        readonly Panel modePanel = new Panel { Dock = DockStyle.Fill };
        readonly TableLayoutPanel numpad;
        readonly DarkToolTip tip = new DarkToolTip();
        readonly List<FlatKey> hypKeys = new List<FlatKey>();
        bool hyp;
        FlatKey hypKey, unitKey, editKey;
        bool customEdit;
        readonly List<FlatKey> inBaseKeys = new List<FlatKey>(), outBaseKeys = new List<FlatKey>();
        readonly List<FlatKey> hexKeys = new List<FlatKey>();
        public event EventHandler CustomButtonsChanged;
        Font keyFont = Theme.KeypadFont;

        public KeypadPanel(IKeypadHost h, AppSettings s)
        {
            host = h; settings = s;
            BackColor = Theme.Window;
            Padding = new Padding(8, 0, 8, 8);
            tabs.Items.AddRange(new[] { "General", "Programming", "Algebra", "Custom" });
            tabs.Dock = DockStyle.Top;
            tabs.Compact = true;
            tabs.BackColor = Theme.Window;
            tabs.SelectedChanged += (o, e) => { settings.Keypad = (KeypadMode)tabs.SelectedIndex; BuildMode(); };
            numpad = BuildNumpad();
            var split = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = 2, RowCount = 1, BackColor = Theme.Window, Margin = new Padding(0), Padding = new Padding(0, 6, 0, 0) };
            split.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 55));
            split.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 45));
            split.Controls.Add(modePanel, 0, 0);
            split.Controls.Add(numpad, 1, 0);
            Controls.Add(split);
            Controls.Add(tabs);
            int idx = (int)settings.Keypad;
            if (idx > 3) idx = 0;
            tabs.SelectedIndex = idx;
            BuildMode();
            ApplyNumpadVisibility();
        }

        public void SetKeyFont(Font f)
        {
            keyFont = f;
            foreach (var k in AllKeys(this)) k.LabelFont = f;
            Invalidate(true);
        }

        static IEnumerable<FlatKey> AllKeys(Control c)
        {
            foreach (Control ch in c.Controls)
            {
                if (ch is FlatKey k) yield return k;
                foreach (var x in AllKeys(ch)) yield return x;
            }
        }

        public void ApplyNumpadVisibility()
        {
            var split = (TableLayoutPanel)numpad.Parent;
            bool modeVisible = settings.Keypad != KeypadMode.NumberPadOnly;
            split.ColumnStyles[0].Width = modeVisible ? 55 : 0;
            split.ColumnStyles[1].Width = settings.NumpadVisible || !modeVisible ? 45 : 0;
            numpad.Visible = settings.NumpadVisible || !modeVisible;
            modePanel.Visible = modeVisible;
        }

        public void SelectMode(KeypadMode m)
        {
            settings.Keypad = m;
            if ((int)m <= 3) tabs.SelectedIndex = (int)m;
            BuildMode();
            ApplyNumpadVisibility();
        }

        FlatKey K(string label, Action a, string tip1, string hint = null, Action a2 = null, string tip2 = null, Action a3 = null, string tip3 = null, KeyStyle style = KeyStyle.Normal)
        {
            var k = new FlatKey(label, a, tip1) { Hint = hint ?? "", Secondary = a2, SecondaryTip = tip2 ?? "", Tertiary = a3, TertiaryTip = tip3 ?? "", KeyStyle = style, Dock = DockStyle.Fill, LabelFont = keyFont, Font = Theme.KeypadFont };
            if (settings.Tooltips && k.TooltipText.Length > 0) tip.SetToolTip(k, k.TooltipText);
            return k;
        }

        static TableLayoutPanel Grid(int cols, int rows)
        {
            var t = new TableLayoutPanel { Dock = DockStyle.Fill, ColumnCount = cols, RowCount = rows, BackColor = Theme.Window, Margin = new Padding(0), Padding = new Padding(0) };
            for (int i = 0; i < cols; i++) t.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100f / cols));
            for (int i = 0; i < rows; i++) t.RowStyles.Add(new RowStyle(SizeType.Percent, 100f / rows));
            return t;
        }

        void Fn(string name) => host.InsertFunction(name);

        void BuildMode()
        {
            modePanel.SuspendLayout();
            foreach (Control c in modePanel.Controls.Cast<Control>().ToList()) { modePanel.Controls.Remove(c); c.Dispose(); }
            hypKeys.Clear(); inBaseKeys.Clear(); outBaseKeys.Clear(); hexKeys.Clear();
            Control grid;
            switch (settings.Keypad)
            {
                case KeypadMode.Programming: grid = BuildProgramming(); break;
                case KeypadMode.Algebra: grid = BuildAlgebra(); break;
                case KeypadMode.Custom: grid = BuildCustom(); break;
                default: grid = BuildGeneral(); break;
            }
            grid.Padding = new Padding(0, 0, 6, 0);
            modePanel.Controls.Add(grid);
            modePanel.ResumeLayout();
            RefreshBaseKeys();
        }

        Control BuildGeneral()
        {
            var g = Grid(6, 5);
            hypKey = K("hyp", () => { hyp = !hyp; hypKey.Checked = hyp; UpdateHyp(); }, "Hyperbolic functions", style: KeyStyle.Muted);
            hypKey.Toggle = true; hypKey.Checked = hyp;
            var sin = K("sin", null, "Sine", "asin", null, "Arcsine", () => Fn("csc"), "Cosecant");
            var cos = K("cos", null, "Cosine", "acos", null, "Arccosine", () => Fn("sec"), "Secant");
            var tan = K("tan", null, "Tangent", "atan", null, "Arctangent", () => Fn("cot"), "Cotangent");
            hypKeys.AddRange(new[] { sin, cos, tan });
            UpdateHyp();
            var row0 = new Control[] { hypKey, sin, cos, tan,
                K("ln", () => Fn("ln"), "Natural logarithm", "log₂", () => Fn("log2"), "Base-2 logarithm", () => host.InsertFunction("log", ", "), "Logarithm with base"),
                K("log", () => Fn("log10"), "Base-10 logarithm", "logₙ", () => host.InsertFunction("log", ", "), "Logarithm with base") };
            var row1 = new Control[] {
                K("x²", () => host.InsertOperator("^2"), "Square", "x³", () => host.InsertOperator("^3"), "Cube", () => host.InsertOperator("^-1"), "Reciprocal"),
                K("xʸ", () => host.InsertOperator("^"), "Power", "E", () => host.InsertOperator("E"), "Exponent ×10ⁿ"),
                K("√", () => Fn("sqrt"), "Square root", "∛", () => Fn("cbrt"), "Cube root", () => host.InsertFunction("root", ", "), "Nth root"),
                K("eˣ", () => Fn("exp"), "Exponential", "2ˣ", () => host.Insert("2^"), "Power of two"),
                K("10ˣ", () => host.Insert("10^"), "Power of ten", "E", () => host.InsertOperator("E"), "Scientific exponent"),
                K("x!", () => host.InsertOperator("!"), "Factorial", "x!!", () => host.InsertOperator("!!"), "Double factorial", () => host.InsertFunction("binomial", ", "), "Binomial coefficient") };
            var row2 = new Control[] {
                K("π", () => host.Insert("π"), "Pi", "γ", () => host.Insert("euler"), "Euler–Mascheroni constant", () => host.Insert("golden"), "Golden ratio"),
                K("e", () => host.Insert("e"), "Euler's number", "φ", () => host.Insert("golden"), "Golden ratio"),
                K("i", () => host.Insert("i"), "Imaginary unit", "∠", () => host.Insert("∠"), "Angle (phasor) notation", () => Fn("arg"), "Argument"),
                K("|x|", () => Fn("abs"), "Absolute value", "sgn", () => Fn("sgn"), "Signum", () => Fn("round"), "Round"),
                K("1/x", () => host.InsertOperator("^-1"), "Reciprocal", "mod", () => host.InsertOperator(" mod "), "Modulo"),
                K("%", () => host.Insert("%"), "Percent", "‰", () => host.Insert("‰"), "Per mille", () => host.RunAction("percentage"), "Percentage calculation tool") };
            var row3 = new Control[] {
                K("x", () => host.Insert("x"), "Unknown x", "y", () => host.Insert("y"), "Unknown y", () => host.Insert("z"), "Unknown z"),
                K("=", () => host.Insert("="), "Equals (equation)", "≠", () => host.Insert("≠"), "Not equal", () => host.InsertFunction("solve"), "Solve"),
                K("Σ", () => host.InsertFunction("sum", ", 1, 10, x"), "Sum", "∏", () => host.InsertFunction("product", ", 1, 10, x"), "Product", () => Fn("mean"), "Mean"),
                K("∫", () => host.InsertFunction("integrate", ", x"), "Integral", "lim", () => host.InsertFunction("limit", ", 0, x"), "Limit"),
                K("d/dx", () => host.InsertFunction("diff", ", x"), "Derivative", "dsolve", () => Fn("dsolve"), "Differential equation", () => host.InsertFunction("extremum", ", x"), "Extrema"),
                K("( )", () => host.InsertFunction(""), "Parenthesize selection or insert parentheses", "[ ]", () => host.Insert("[]", 1), "Vector brackets", () => host.OpenMatrixEditor(), "Matrix editor") };
            unitKey = K(host.LastUnit, () => host.Insert(host.LastUnit), "Insert last used unit", "▾", () => host.ShowUnitsMenu(unitKey), "Units menu");
            var row4 = new Control[] {
                K("STO", () => host.StoreResult(), "Store result as variable", "new", () => host.RunAction("newvariable"), "New variable"),
                K("MR", () => host.MemoryRecall(), "Memory recall", "MS", () => host.MemoryStore(), "Memory store", () => host.MemoryClear(), "Memory clear"),
                K("M+", () => host.MemoryAdd(false), "Memory add", "M−", () => host.MemoryAdd(true), "Memory subtract", () => host.MemoryClear(), "Memory clear"),
                unitKey,
                K("f(x)", null, "Functions menu", "var", null, "Variables menu", () => host.RunAction("newfunction"), "New function"),
                K("to", null, "Conversion menu", "→", () => host.Insert(" to "), "Insert conversion operator") };
            var fx = (FlatKey)row4[4];
            fx.Primary = () => host.ShowFunctionsMenu(fx); fx.Secondary = () => host.ShowVariablesMenu(fx);
            var to = (FlatKey)row4[5];
            to.Primary = () => host.ShowConversionMenu(to);
            AddRows(g, row0, row1, row2, row3, row4);
            return g;
        }

        void UpdateHyp()
        {
            if (hypKeys.Count < 3) return;
            string[] n = hyp ? new[] { "sinh", "cosh", "tanh" } : new[] { "sin", "cos", "tan" };
            string[] inv = hyp ? new[] { "asinh", "acosh", "atanh" } : new[] { "asin", "acos", "atan" };
            for (int i = 0; i < 3; i++)
            {
                var k = hypKeys[i];
                string f = n[i], fi = inv[i];
                k.Label = f; k.Hint = fi;
                k.Primary = () => Fn(f);
                k.Secondary = () => Fn(fi);
                k.PrimaryTip = f; k.SecondaryTip = fi;
                if (settings.Tooltips) tip.SetToolTip(k, k.TooltipText);
                k.Invalidate();
            }
        }

        static void AddRows(TableLayoutPanel g, params Control[][] rows)
        {
            for (int r = 0; r < rows.Length; r++)
                for (int c = 0; c < rows[r].Length; c++)
                    g.Controls.Add(rows[r][c], c, r);
        }

        Control BuildProgramming()
        {
            var g = Grid(6, 5);
            var row0 = "ABCDEF".Select(ch => { var k = K(ch.ToString(), () => host.Insert(ch.ToString()), "Hexadecimal digit " + ch, style: KeyStyle.Number); hexKeys.Add(k); return (Control)k; }).ToArray();
            var row1 = new Control[] {
                K("AND", () => host.InsertOperator("&"), "Bitwise AND", "&&", () => host.InsertOperator("&&"), "Logical AND"),
                K("OR", () => host.InsertOperator("|"), "Bitwise OR", "||", () => host.InsertOperator("||"), "Logical OR"),
                K("XOR", () => host.InsertOperator(" xor "), "Bitwise exclusive OR"),
                K("NOT", () => host.Insert("~"), "Bitwise NOT", "!", () => host.Insert("!"), "Logical NOT"),
                K("<<", () => host.InsertOperator("<<"), "Left shift"),
                K(">>", () => host.InsertOperator(">>"), "Right shift") };
            var row2 = new Control[] {
                K("mod", () => host.InsertOperator(" mod "), "Modulo", "rem", () => host.InsertOperator(" rem "), "Remainder"),
                K("div", () => host.InsertOperator("//"), "Integer division"),
                K("rot", () => host.InsertFunction("circshift", ", 1, 8"), "Circular shift"),
                K("cmp", () => host.InsertFunction("bitcmp", ", 8"), "Bitwise complement"),
                K("a→1", () => host.InsertFunction("code"), "Character code", "1→a", () => host.InsertFunction("char"), "Character from code"),
                K("log₂", () => host.InsertFunction("log2"), "Base-2 logarithm", "pop", () => host.InsertFunction("bitcount"), "Count set bits") };
            var row3 = new List<Control> { K("in", null, "Input base", style: KeyStyle.Muted) };
            var row4 = new List<Control> { K("out", null, "Result base", style: KeyStyle.Muted) };
            foreach (var b in new[] { (2, "BIN"), (8, "OCT"), (10, "DEC"), (16, "HEX"), (12, "DUO") })
            {
                int bv = b.Item1;
                var ik = K(b.Item2, () => { host.SetInputBase(bv); RefreshBaseKeys(); }, "Interpret input in base " + bv); ik.Toggle = true; ik.Tag = bv; inBaseKeys.Add(ik); row3.Add(ik);
                var ok = K(b.Item2, () => { host.SetOutputBase(bv); RefreshBaseKeys(); }, "Show results in base " + bv); ok.Toggle = true; ok.Tag = bv; outBaseKeys.Add(ok); row4.Add(ok);
            }
            AddRows(g, row0, row1, row2, row3.ToArray(), row4.ToArray());
            return g;
        }

        public void RefreshBaseKeys()
        {
            foreach (var k in inBaseKeys) { k.Checked = (int)k.Tag == host.InputBase; k.Invalidate(); }
            foreach (var k in outBaseKeys) { k.Checked = (int)k.Tag == host.OutputBase; k.Invalidate(); }
            foreach (var k in hexKeys) { k.Enabled = host.InputBase > 10; k.Invalidate(); }
        }

        Control BuildAlgebra()
        {
            var g = Grid(6, 5);
            var row0 = new[] { "x", "y", "z", "n", "a", "b" }.Select(v => (Control)K(v, () => host.Insert(v), "Variable " + v, style: KeyStyle.Number)).ToArray();
            var row1 = new Control[] {
                K("=", () => host.Insert("="), "Equals"), K("≠", () => host.Insert("≠"), "Not equal"),
                K("<", () => host.Insert("<"), "Less than"), K(">", () => host.Insert(">"), "Greater than"),
                K("≤", () => host.Insert("≤"), "Less than or equal"), K("≥", () => host.Insert("≥"), "Greater than or equal") };
            var row2 = new Control[] {
                K("solve", () => host.InsertFunction("solve", ", x"), "Solve equation", "sys", () => host.InsertFunction("multisolve", ", [x, y]"), "Solve system"),
                K("factor", () => host.InsertFunction("factor"), "Factorize"),
                K("expand", () => host.InsertFunction("expand"), "Expand"),
                K("partfr", () => host.InsertFunction("partfrac"), "Partial fractions"),
                K("simpl", () => host.InsertFunction("simplify"), "Simplify", "tog", () => host.InsertFunction("together"), "Combine fractions"),
                K("/.", () => host.Insert(" /. "), "Substitute (where)", "where", () => host.Insert(" where "), "Where") };
            var row3 = new Control[] {
                K("d/dx", () => host.InsertFunction("diff", ", x"), "Derivative"),
                K("∫", () => host.InsertFunction("integrate", ", x"), "Integral", "∫ab", () => host.InsertFunction("integrate", ", x, 0, 1"), "Definite integral"),
                K("lim", () => host.InsertFunction("limit", ", 0, x"), "Limit"),
                K("Σ", () => host.InsertFunction("sum", ", 1, 10, x"), "Sum"),
                K("∏", () => host.InsertFunction("product", ", 1, 10, x"), "Product"),
                K("taylor", () => host.InsertFunction("taylor", ", x, 0, 5"), "Taylor polynomial") };
            var row4 = new Control[] {
                K("[ ]", () => host.Insert("[]", 1), "Vector brackets", "mat", () => host.OpenMatrixEditor(), "Matrix editor"),
                K(";", () => host.Insert("; "), "Matrix row separator"),
                K("det", () => host.InsertFunction("det"), "Determinant"),
                K("inv", () => host.InsertFunction("inverse"), "Inverse"),
                K("ᵀ", () => host.InsertFunction("transpose"), "Transpose"),
                K("coeff", () => host.InsertFunction("coeff", ", 1, x"), "Polynomial coefficient", "deg", () => host.InsertFunction("degree", ", x"), "Degree") };
            AddRows(g, row0, row1, row2, row3, row4);
            return g;
        }

        Control BuildCustom()
        {
            var outer = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window };
            var g = Grid(6, 5);
            for (int i = 0; i < 30; i++)
            {
                int idx = i;
                var data = idx < settings.CustomButtons.Count ? settings.CustomButtons[idx] : null;
                FlatKey k;
                if (data != null && data.Length >= 2 && data[0].Length > 0)
                {
                    string label = data[0], a1 = data[1], a2 = data.Length > 2 ? data[2] : "", a3 = data.Length > 3 ? data[3] : "";
                    k = K(label, () => RunCustom(idx, a1), a1, a2.Length > 0 ? a2 : null, a2.Length > 0 ? (Action)(() => RunCustom(idx, a2)) : null, a2, a3.Length > 0 ? (Action)(() => RunCustom(idx, a3)) : null, a3);
                }
                else k = K("+", () => EditCustom(idx), "Add a custom button", style: KeyStyle.Muted);
                if (customEdit) { k.Primary = () => EditCustom(idx); k.Secondary = () => EditCustom(idx); }
                g.Controls.Add(k, i % 6, i / 6);
            }
            var bar = new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 34, BackColor = Theme.Window, Padding = new Padding(0, 4, 0, 0) };
            var edit = new FlatButton(customEdit ? "Done" : "Edit buttons", (s, e) => { customEdit = !customEdit; BuildMode(); }, customEdit);
            bar.Controls.Add(edit);
            outer.Controls.Add(g);
            outer.Controls.Add(bar);
            return outer;
        }

        void RunCustom(int idx, string action)
        {
            if (customEdit) { EditCustom(idx); return; }
            if (action.StartsWith("@")) host.RunAction(action.Substring(1));
            else if (action.EndsWith("()") && action.Length > 2) host.InsertFunction(action.Substring(0, action.Length - 2));
            else host.Insert(action);
        }

        void EditCustom(int idx)
        {
            var cur = idx < settings.CustomButtons.Count ? settings.CustomButtons[idx] : new[] { "", "", "", "" };
            using (var d = new CustomButtonDialog(cur))
            {
                var r = d.ShowDialog(FindForm());
                if (r == DialogResult.Abort)
                {
                    if (idx < settings.CustomButtons.Count) settings.CustomButtons[idx] = new[] { "", "" };
                }
                else if (r == DialogResult.OK)
                {
                    while (settings.CustomButtons.Count <= idx) settings.CustomButtons.Add(new[] { "", "" });
                    settings.CustomButtons[idx] = d.Values;
                }
                else return;
            }
            CustomButtonsChanged?.Invoke(this, EventArgs.Empty);
            BuildMode();
        }

        TableLayoutPanel BuildNumpad()
        {
            var g = Grid(5, 5);
            Func<string, string, string, KeyStyle, FlatKey> digit = (d, sup, frac, st) =>
                K(d, () => host.Insert(d), "Digit " + d, sup, () => host.Insert(sup), "Superscript " + sup, () => host.Insert(frac), frac.Length > 0 ? "Fraction " + frac : null, st);
            var row0 = new Control[] {
                K("(", () => host.Insert("("), "Left parenthesis", "[", () => host.Insert("["), "Left vector bracket"),
                K(")", () => host.Insert(")"), "Right parenthesis", "]", () => host.Insert("]"), "Right vector bracket"),
                K("←", () => host.MoveCaret(-1), "Move cursor left", "|←", () => host.MoveCaret(-1, true), "Move cursor to start"),
                K("→", () => host.MoveCaret(1), "Move cursor right", "→|", () => host.MoveCaret(1, true), "Move cursor to end"),
                K("⌫", () => host.Backspace(), "Backspace", "del", () => host.DeleteForward(), "Delete", style: KeyStyle.Operator) };
            var row1 = new Control[] { digit("7", "⁷", "⅐", KeyStyle.Number), digit("8", "⁸", "⅛", KeyStyle.Number), digit("9", "⁹", "⅑", KeyStyle.Number),
                K("÷", () => host.InsertOperator("/"), "Division", "//", () => host.InsertOperator("//"), "Integer division", () => host.InsertOperator(" mod "), "Modulo", KeyStyle.Operator),
                K("AC", () => host.ClearExpression(), "Clear expression", "stack", () => host.ClearStack(), "Clear the RPN stack", style: KeyStyle.Operator) };
            var row2 = new Control[] { digit("4", "⁴", "¼", KeyStyle.Number), digit("5", "⁵", "⅕", KeyStyle.Number), digit("6", "⁶", "⅙", KeyStyle.Number),
                K("×", () => host.InsertOperator("*"), "Multiplication", "&", () => host.InsertOperator("&"), "Bitwise AND", () => host.InsertOperator("<<"), "Left shift", KeyStyle.Operator),
                K("ANS", () => host.Insert("ans"), "Previous result", "ans₂", () => host.Insert("ans2"), "Second previous result", style: KeyStyle.Operator) };
            var row3 = new Control[] { digit("1", "¹", "", KeyStyle.Number), digit("2", "²", "½", KeyStyle.Number), digit("3", "³", "⅓", KeyStyle.Number),
                K("−", () => host.InsertOperator("-"), "Subtraction", "±", () => host.Negate(), "Negate / plus-minus", () => host.Insert("±"), "Uncertainty", KeyStyle.Operator),
                K("EXP", () => host.InsertOperator("E"), "×10ⁿ exponent", "^", () => host.InsertOperator("^"), "Power", style: KeyStyle.Operator) };
            var row4 = new Control[] { digit("0", "⁰", "°", KeyStyle.Number),
                K(".", () => host.Insert("."), "Decimal point", "␣", () => host.Insert(" "), "Space", () => host.Insert("\r\n"), "New line", KeyStyle.Number),
                K(",", () => host.Insert(", "), "Argument separator", ";", () => host.Insert("; "), "Matrix row separator", style: KeyStyle.Number),
                K("+", () => host.InsertOperator("+"), "Addition", "|", () => host.InsertOperator("|"), "Bitwise OR", () => host.InsertOperator("~"), "Bitwise NOT", KeyStyle.Operator),
                K("=", () => host.Calculate(), "Calculate (Enter)", "to", () => host.Insert(" to "), "Conversion", style: KeyStyle.Accent) };
            AddRows(g, row0, row1, row2, row3, row4);
            return g;
        }

        public void RefreshUnitKey()
        {
            if (unitKey != null) { unitKey.Label = host.LastUnit; unitKey.Invalidate(); }
}   }   }