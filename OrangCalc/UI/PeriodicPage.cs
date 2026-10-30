using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.Linq;
using System.Text.RegularExpressions;
using System.Windows.Forms;
using OrangCalc.Engine;
using DataObject = OrangCalc.Engine.DataObject;

namespace OrangCalc.UI
{
    public enum ElementColoring { Category, Phase, Electronegativity, Density, Melting, Boiling, Weight, Discovery }

    public sealed class PeriodicTableView : Control
    {
        public DataSet Set;
        public DataObject Selected, Hover;
        public ElementColoring Coloring = ElementColoring.Category;
        public int HighlightClass = -1;
        public string Filter = "";
        public event Action<DataObject> ElementClicked;
        public event Action<DataObject> ElementDoubleClicked;

        public static readonly Color[] ClassColors =
        {
            Color.FromArgb(84, 88, 96),
            Color.FromArgb(160, 78, 72), Color.FromArgb(166, 116, 60), Color.FromArgb(124, 98, 158), Color.FromArgb(156, 84, 130),
            Color.FromArgb(74, 106, 156), Color.FromArgb(62, 132, 140), Color.FromArgb(104, 136, 74), Color.FromArgb(68, 146, 102),
            Color.FromArgb(158, 148, 68), Color.FromArgb(98, 94, 168), Color.FromArgb(90, 94, 102)
        };

        public PeriodicTableView()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw | ControlStyles.Selectable, true);
            BackColor = Theme.Window;
        }

        void Geometry(out float cell, out float ox, out float oy, out float gap)
        {
            float w = Width - 30, h = Height - 22;
            gap = 3;
            float byW = w / 18f, byH = h / 9.6f;
            cell = Math.Max(16, Math.Min(byW, byH));
            ox = 26 + (w - cell * 18) / 2;
            oy = 20 + Math.Max(0, (h - cell * 9.45f) / 2);
        }

        RectangleF CellRect(int x, int y)
        {
            Geometry(out float cell, out float ox, out float oy, out float gap);
            float yy = y <= 7 ? (y - 1) * cell : (7 * cell + cell * 0.45f + (y - 9) * cell);
            return new RectangleF(ox + (x - 1) * cell + gap / 2, oy + yy + gap / 2, cell - gap, cell - gap);
        }

        DataObject HitTest(Point p)
        {
            if (Set == null) return null;
            foreach (var o in Set.Objects)
            {
                int x = int.Parse(o.Get("x_pos")), y = int.Parse(o.Get("y_pos"));
                if (CellRect(x, y).Contains(p)) return o;
            }
            return null;
        }

        protected override void OnMouseMove(MouseEventArgs e)
        {
            var h = HitTest(e.Location);
            if (h != Hover) { Hover = h; Invalidate(); Cursor = h != null ? Cursors.Hand : Cursors.Default; }
            base.OnMouseMove(e);
        }

        protected override void OnMouseLeave(EventArgs e) { Hover = null; Invalidate(); base.OnMouseLeave(e); }

        protected override void OnMouseDown(MouseEventArgs e)
        {
            Focus();
            var h = HitTest(e.Location);
            if (h != null) { Selected = h; Invalidate(); ElementClicked?.Invoke(h); }
            base.OnMouseDown(e);
        }

        protected override void OnMouseDoubleClick(MouseEventArgs e)
        {
            var h = HitTest(e.Location);
            if (h != null) ElementDoubleClicked?.Invoke(h);
            base.OnMouseDoubleClick(e);
        }

        protected override bool IsInputKey(Keys k) => k == Keys.Left || k == Keys.Right || k == Keys.Up || k == Keys.Down || base.IsInputKey(k);

        protected override void OnKeyDown(KeyEventArgs e)
        {
            if (Set == null) return;
            if (Selected == null) { Selected = Set.Objects[0]; Invalidate(); ElementClicked?.Invoke(Selected); return; }
            int z = int.Parse(Selected.Get("number"));
            int x = int.Parse(Selected.Get("x_pos")), y = int.Parse(Selected.Get("y_pos"));
            DataObject next = null;
            if (e.KeyCode == Keys.Right) next = Set.Objects.FirstOrDefault(o => o.Get("number") == (z + 1).ToString());
            else if (e.KeyCode == Keys.Left) next = Set.Objects.FirstOrDefault(o => o.Get("number") == (z - 1).ToString());
            else if (e.KeyCode == Keys.Down || e.KeyCode == Keys.Up)
            {
                int dy = e.KeyCode == Keys.Down ? 1 : -1;
                for (int yy = y + dy; yy >= 1 && yy <= 10; yy += dy)
                {
                    next = Set.Objects.FirstOrDefault(o => o.Get("x_pos") == x.ToString() && o.Get("y_pos") == yy.ToString());
                    if (next != null) break;
                }
            }
            else if (e.KeyCode == Keys.Enter) { ElementDoubleClicked?.Invoke(Selected); return; }
            if (next != null) { Selected = next; Invalidate(); ElementClicked?.Invoke(next); e.Handled = true; }
            base.OnKeyDown(e);
        }

        static double? Num(DataObject o, string p)
        {
            var v = o.Get(p);
            if (v == null) return null;
            return double.TryParse(v, NumberStyles.Float, CultureInfo.InvariantCulture, out var d) ? d : (double?)null;
        }

        // range of a property over all elements, for the heat map
        bool Range(string prop, bool log, out double lo, out double hi)
        {
            var vals = Set.Objects.Select(o => Num(o, prop)).Where(v => v.HasValue && (!log || v.Value > 0)).Select(v => log ? Math.Log10(v.Value) : v.Value).ToList();
            lo = vals.Count > 0 ? vals.Min() : 0; hi = vals.Count > 0 ? vals.Max() : 1;
            return vals.Count > 0;
        }

        static readonly Color ScaleLow = Color.FromArgb(52, 84, 140), ScaleMid = Color.FromArgb(70, 140, 110), ScaleHigh = Color.FromArgb(176, 100, 52);

        static Color Lerp(Color a, Color b, double t)
        {
            t = Math.Max(0, Math.Min(1, t));
            return Color.FromArgb((int)(a.R + (b.R - a.R) * t), (int)(a.G + (b.G - a.G) * t), (int)(a.B + (b.B - a.B) * t));
        }

        public const int Steps = 7;

        // stepped scale, solid colors only
        public static Color Scale(double t)
        {
            int step = (int)Math.Floor(Math.Max(0, Math.Min(0.9999, t)) * Steps);
            double q = step / (double)(Steps - 1);
            return q < 0.5 ? Lerp(ScaleLow, ScaleMid, q * 2) : Lerp(ScaleMid, ScaleHigh, (q - 0.5) * 2);
        }

        public Color ColorFor(DataObject o)
        {
            switch (Coloring)
            {
                case ElementColoring.Phase:
                    switch (o.Get("phase"))
                    {
                        case "Solid": return Color.FromArgb(92, 100, 112);
                        case "Liquid": return Color.FromArgb(62, 112, 160);
                        case "Gas": return Color.FromArgb(150, 110, 62);
                        default: return ClassColors[0];
                    }
                case ElementColoring.Discovery:
                    {
                        var d = o.Get("discovered");
                        if (d == "Ancient") return Scale(0);
                        if (int.TryParse(d, out int y)) return Scale((y - 1600) / 420.0);
                        return ClassColors[0];
                    }
                case ElementColoring.Category:
                    int.TryParse(o.Get("class_id"), out int cls);
                    return ClassColors[Math.Max(0, Math.Min(11, cls))];
            }
            string prop = Coloring == ElementColoring.Electronegativity ? "electronegativity" : Coloring == ElementColoring.Density ? "density" : Coloring == ElementColoring.Melting ? "melting" : Coloring == ElementColoring.Boiling ? "boiling" : "weight";
            bool log = Coloring == ElementColoring.Density;
            var v = Num(o, prop);
            if (!v.HasValue || !Range(prop, log, out double lo, out double hi)) return ClassColors[0];
            double val = log ? Math.Log10(Math.Max(v.Value, 1e-12)) : v.Value;
            return Scale((val - lo) / Math.Max(1e-12, hi - lo));
        }

        bool Matches(DataObject o)
        {
            if (Filter.Length == 0) return true;
            var f = Filter.ToLowerInvariant();
            return o.Get("symbol").ToLowerInvariant() == f || o.Get("name").ToLowerInvariant().StartsWith(f) || o.Get("number") == f;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(BackColor);
            if (Set == null) return;
            Geometry(out float cell, out float ox, out float oy, out float gap);
            var numFont = new Font("Segoe UI", Math.Max(6f, cell * 0.17f));
            var symFont = new Font("Segoe UI Semibold", Math.Max(7f, cell * 0.3f));
            var nameFont = new Font("Segoe UI", Math.Max(5.5f, cell * 0.13f));
            for (int i = 1; i <= 18; i++)
            {
                var r = CellRect(i, 1);
                TextRenderer.DrawText(g, i.ToString(), Theme.SmallFont, new Rectangle((int)r.X, (int)r.Y - 18, (int)r.Width, 16), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter);
            }
            for (int p = 1; p <= 7; p++)
            {
                var r = CellRect(1, p);
                TextRenderer.DrawText(g, p.ToString(), Theme.SmallFont, new Rectangle((int)ox - 22, (int)r.Y, 18, (int)r.Height), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.Right);
            }
            foreach (var (row, label) in new[] { (6, "57–71"), (7, "89–103") })
            {
                var r = CellRect(3, row);
                using (var pen = new Pen(Theme.Line) { DashStyle = System.Drawing.Drawing2D.DashStyle.Dot }) g.DrawRectangle(pen, r.X, r.Y, r.Width, r.Height);
                TextRenderer.DrawText(g, label, nameFont, Rectangle.Round(r), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter);
            }
            foreach (var o in Set.Objects)
            {
                int x = int.Parse(o.Get("x_pos")), y = int.Parse(o.Get("y_pos"));
                var r = CellRect(x, y);
                int.TryParse(o.Get("class_id"), out int cls);
                bool dim = (HighlightClass >= 0 && cls != HighlightClass && Coloring == ElementColoring.Category) || !Matches(o);
                var baseColor = ColorFor(o);
                var fill = Lerp(Theme.Panel, baseColor, dim ? 0.12 : (o == Hover ? 0.75 : 0.55));
                using (var b = new SolidBrush(fill)) g.FillRectangle(b, r);
                var border = o == Selected ? Theme.Text : dim ? Theme.Line : baseColor;
                using (var pen = new Pen(border, o == Selected ? 2f : 1f)) g.DrawRectangle(pen, r.X, r.Y, r.Width, r.Height);
                var fg = dim ? Theme.TextFaint : Theme.Text;
                var ri = Rectangle.Round(r);
                TextRenderer.DrawText(g, o.Get("number"), numFont, new Rectangle(ri.X + 2, ri.Y + 1, ri.Width - 4, numFont.Height), dim ? Theme.TextFaint : Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.Left | TextFormatFlags.NoPadding);
                TextRenderer.DrawText(g, o.Get("symbol"), symFont, new Rectangle(ri.X, ri.Y + (int)(cell * 0.13f), ri.Width, (int)(cell * 0.6f)), fg, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.NoPadding);
                if (cell >= 44)
                    TextRenderer.DrawText(g, o.Get("name"), nameFont, new Rectangle(ri.X + 1, ri.Bottom - nameFont.Height - 3, ri.Width - 2, nameFont.Height + 1), dim ? Theme.TextFaint : Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPadding);
            }
            numFont.Dispose(); symFont.Dispose(); nameFont.Dispose();
        }
    }

    public sealed class PeriodicPage : Panel, IPage
    {
        readonly MainForm main;
        readonly PeriodicTableView table = new PeriodicTableView { Dock = DockStyle.Fill };
        readonly RichTextBox details;
        readonly DataGridView props;
        readonly FlowLayoutPanel legend;
        readonly DarkCombo coloring;
        readonly TextBox search, formula;
        readonly Label molar;
        readonly Label bigSymbol, bigName, bigNumber, bigClass;
        DataObject current;

        public PeriodicPage(MainForm m)
        {
            main = m;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);
            table.Set = Data.Atoms;
            table.ElementClicked += o => ShowElement(o);
            table.ElementDoubleClicked += o => main.InsertIntoExpression(o.Get("symbol"));

            var header = new Panel { Dock = DockStyle.Top, Height = 40, BackColor = Theme.Window };
            var title = Ui.Header("Periodic Table");
            title.Location = new Point(0, 4);
            header.Controls.Add(title);
            coloring = Ui.Combo(new[] { "Color by category", "Color by phase (STP)", "Color by electronegativity", "Color by density", "Color by melting point", "Color by boiling point", "Color by atomic weight", "Color by discovery year" }, 0, 210);
            coloring.SelectedIndexChanged += (s, e) => { table.Coloring = (ElementColoring)coloring.SelectedIndex; table.HighlightClass = -1; BuildLegend(); table.Invalidate(); };
            search = Ui.Box("", 170);
            HistoryPanel.SetCue(search, "Find element");
            search.TextChanged += (s, e) =>
            {
                table.Filter = search.Text.Trim();
                var hit = Data.Atoms.Objects.FirstOrDefault(o => o.Get("symbol").Equals(table.Filter, StringComparison.OrdinalIgnoreCase) || o.Get("number") == table.Filter) ??
                          Data.Atoms.Objects.FirstOrDefault(o => table.Filter.Length > 1 && o.Get("name").StartsWith(table.Filter, StringComparison.OrdinalIgnoreCase));
                if (hit != null) { table.Selected = hit; ShowElement(hit); }
                table.Invalidate();
            };
            var tools = Ui.Row(search, coloring);
            tools.Dock = DockStyle.Right;
            tools.Padding = new Padding(0, 6, 0, 0);
            header.Controls.Add(tools);

            legend = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 56, BackColor = Theme.Window, Padding = new Padding(20, 4, 0, 0) });

            var molarRow = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 38, BackColor = Theme.Window, Padding = new Padding(20, 4, 0, 0) });
            formula = Ui.Box("H2O", 200);
            formula.Font = Theme.MonoFont;
            molar = new Label { AutoSize = true, ForeColor = Theme.Text, Font = Theme.UiBold, Margin = new Padding(10, 7, 10, 0) };
            formula.TextChanged += (s, e) => UpdateMolar();
            var insMolar = new FlatButton("Insert", (s, e) => { var v = MolarMass(formula.Text, out _); if (v > 0) main.InsertIntoExpression(v.ToString("0.#####", CultureInfo.InvariantCulture) + " g/mol"); });
            molarRow.Controls.AddRange(new Control[] { Ui.Label("Molar mass of", true), formula, molar, insMolar });

            var left = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window };
            left.Controls.Add(table);
            left.Controls.Add(molarRow);
            left.Controls.Add(legend);

            var card = Ui.ShareWidth(new Card { Dock = DockStyle.Right, Width = 360, Padding = new Padding(14, 12, 6, 8) }, 0.32f, 230, 380);
            var top = new Panel { Dock = DockStyle.Top, Height = 110, BackColor = Theme.Panel };
            bigNumber = new Label { Location = new Point(2, 2), AutoSize = true, ForeColor = Theme.TextDim, Font = Theme.UiFont };
            bigSymbol = new Label { Location = new Point(0, 18), AutoSize = true, ForeColor = Theme.Text, Font = new Font("Segoe UI Semibold", 30f) };
            bigName = new Label { Location = new Point(4, 70), AutoSize = true, ForeColor = Theme.Text, Font = Theme.HeaderFont };
            bigClass = new Label { Location = new Point(140, 30), AutoSize = true, ForeColor = Theme.TextDim, Font = Theme.UiFont };
            top.Controls.AddRange(new Control[] { bigNumber, bigSymbol, bigName, bigClass });
            top.Resize += (s, e) => FitClassLabel();
            props = new DataGridView { Dock = DockStyle.Fill, ReadOnly = true, AllowUserToAddRows = false, SelectionMode = DataGridViewSelectionMode.FullRowSelect, MultiSelect = false, AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill, ColumnHeadersVisible = false };
            Theme.StyleGrid(props);
            props.CellBorderStyle = DataGridViewCellBorderStyle.SingleHorizontal;
            props.Columns.Add("p", "Property");
            props.Columns.Add("v", "Value");
            props.Columns[0].DefaultCellStyle.ForeColor = Theme.TextDim;
            props.Columns[0].AutoSizeMode = DataGridViewAutoSizeColumnMode.None;
            props.Resize += (s, e) => FitPropColumn();
            props.CellDoubleClick += (s, e) => InsertProperty(e.RowIndex, false);
            details = new RichTextBox { Visible = false };
            var buttons = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 72, BackColor = Theme.Panel, Padding = new Padding(0, 6, 0, 0) });
            buttons.Controls.Add(new FlatButton("Insert value", (s, e) => InsertProperty(props.CurrentCell?.RowIndex ?? -1, false), true) { Margin = new Padding(0, 0, 6, 6) });
            buttons.Controls.Add(new FlatButton("Insert lookup", (s, e) => InsertProperty(props.CurrentCell?.RowIndex ?? -1, true)) { Margin = new Padding(0, 0, 6, 6) });
            buttons.Controls.Add(new FlatButton("Insert symbol", (s, e) => { if (current != null) main.InsertIntoExpression(current.Get("symbol")); }) { Margin = new Padding(0, 0, 6, 6) });
            buttons.Controls.Add(new FlatButton("Add to formula", (s, e) => { if (current != null) formula.Text += current.Get("symbol"); }) { Margin = new Padding(0, 0, 6, 6) });
            card.Controls.Add(props);
            card.Controls.Add(buttons);
            card.Controls.Add(top);

            var gapPanel = new Panel { Dock = DockStyle.Right, Width = 12, BackColor = Theme.Window };
            Controls.Add(left);
            Controls.Add(gapPanel);
            Controls.Add(card);
            Controls.Add(header);
            BuildLegend();
            ShowElement(Data.Atoms.Objects[0]);
            table.Selected = Data.Atoms.Objects[0];
            UpdateMolar();
        }

        void BuildLegend()
        {
            legend.Controls.Clear();
            if (table.Coloring == ElementColoring.Category)
            {
                for (int i = 1; i <= 11; i++)
                {
                    int cls = i;
                    var chip = new FlatKey(Data.ElementClasses[i], null) { Width = TextRenderer.MeasureText(Data.ElementClasses[i], Theme.UiFont).Width + 30, Height = 22, Font = Theme.UiFont, LabelFont = Theme.SmallFont, KeyStyle = KeyStyle.Muted, Margin = new Padding(0, 0, 4, 4) };
                    chip.Paint += (s, e) => { using (var b = new SolidBrush(PeriodicTableView.ClassColors[cls])) e.Graphics.FillRectangle(b, 6, 7, 8, 8); };
                    chip.Label = "    " + Data.ElementClasses[i];
                    chip.Primary = () => { table.HighlightClass = table.HighlightClass == cls ? -1 : cls; table.Invalidate(); };
                    legend.Controls.Add(chip);
                }
            }
            else if (table.Coloring == ElementColoring.Phase)
            {
                foreach (var (n, c) in new[] { ("Solid", Color.FromArgb(92, 100, 112)), ("Liquid", Color.FromArgb(62, 112, 160)), ("Gas", Color.FromArgb(150, 110, 62)), ("Unknown", PeriodicTableView.ClassColors[0]) })
                {
                    var col = c;
                    var l = new Label { Text = "      " + n, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 4, 14, 0) };
                    l.Paint += (s, e) => { using (var b = new SolidBrush(col)) e.Graphics.FillRectangle(b, 2, 3, 10, 10); };
                    legend.Controls.Add(l);
                }
            }
            else
            {
                var bar = new Panel { Width = 7 * 34, Height = 14, Margin = new Padding(0, 6, 8, 0) };
                bar.Paint += (s, e) =>
                {
                    for (int i = 0; i < PeriodicTableView.Steps; i++)
                        using (var b = new SolidBrush(PeriodicTableView.Scale((i + 0.5) / PeriodicTableView.Steps))) e.Graphics.FillRectangle(b, i * 34, 0, 32, bar.Height);
                };
                legend.Controls.Add(Ui.Label("low", true));
                legend.Controls.Add(bar);
                legend.Controls.Add(Ui.Label(table.Coloring == ElementColoring.Discovery ? "recent" : "high", true));
                legend.Controls.Add(Ui.Label(table.Coloring == ElementColoring.Density ? "   (logarithmic scale, gray = no data)" : "   (gray = no data)", true));
            }
        }

        void ShowElement(DataObject o)
        {
            current = o;
            bigNumber.Text = o.Get("number");
            bigSymbol.Text = o.Get("symbol");
            bigName.Text = o.Get("name");
            int.TryParse(o.Get("class_id"), out int cls);
            bigClass.Text = o.Get("class") + "\n" + (o.Get("group") != null ? "Group " + o.Get("group") + ", " : "") + "Period " + o.Get("period");
            bigClass.ForeColor = Theme.TextDim;
            bigClass.Location = new Point(Math.Max(110, bigSymbol.Right + 16), 30);
            FitClassLabel();
            props.Rows.Clear();
            var c = main.Calc;
            foreach (var p in Data.Atoms.Properties.Where(x => !x.Hidden))
            {
                var v = o.Get(p.Name);
                if (v == null) continue;
                string shown = v;
                if (!p.IsText && p.Name != "number" && p.Name != "group" && p.Name != "period")
                {
                    shown = v + (p.Unit.Length > 0 ? " " + p.Unit.Replace("^3", "³") : "");
                    if (p.Name == "melting" || p.Name == "boiling")
                    {
                        if (double.TryParse(v, NumberStyles.Float, CultureInfo.InvariantCulture, out var k)) shown += "  (" + (k - 273.15).ToString("0.##", CultureInfo.InvariantCulture) + " °C)";
                    }
                    if (p.Name == "weight" && (o.Get("class_id") == "11" || int.Parse(o.Get("number")) > 83 && !v.Contains("."))) shown = "[" + v + "]";
                    if (p.Name == "density" && o.Get("phase") == "Gas") shown += "  (" + (double.Parse(v, CultureInfo.InvariantCulture) * 1000).ToString("0.####", CultureInfo.InvariantCulture) + " g/L)";
                }
                var row = props.Rows.Add(p.Title, shown);
                props.Rows[row].Tag = p;
            }
            props.ClearSelection();
            FitPropColumn();
            table.Invalidate();
        }

        void FitClassLabel() => bigClass.MaximumSize = new Size(Math.Max(60, bigClass.Parent.Width - bigClass.Left - 4), 0);

        // property names get their full width unless that would squeeze the values
        void FitPropColumn()
        {
            if (props.Width <= 0) return;
            props.Columns[0].Width = Math.Max(60, Math.Min(props.Columns[0].GetPreferredWidth(DataGridViewAutoSizeColumnMode.AllCells, true), props.Width / 2));
        }

        void InsertProperty(int row, bool lookup)
        {
            if (current == null || row < 0 || row >= props.Rows.Count) return;
            var p = (DataProperty)props.Rows[row].Tag;
            var sym = current.Get("symbol");
            if (lookup || p.IsText) { main.InsertIntoExpression("atom(" + sym + ", " + p.Name + ")"); return; }
            var v = current.Get(p.Name);
            main.InsertIntoExpression(v + (p.Unit.Length > 0 ? " " + p.Unit : ""));
        }

        void UpdateMolar()
        {
            var v = MolarMass(formula.Text, out string err);
            molar.Text = err ?? "= " + v.ToString("0.#####", CultureInfo.InvariantCulture) + " g/mol";
            molar.ForeColor = err != null ? Theme.TextFaint : Theme.Text;
        }

        // parses formulas like H2O, Ca(OH)2, CuSO4·5H2O, [Fe(CN)6]3
        public static double MolarMass(string f, out string error)
        {
            error = null;
            f = (f ?? "").Replace(" ", "");
            if (f.Length == 0) { error = "enter a formula"; return 0; }
            try
            {
                double total = 0;
                foreach (var part in f.Split('·', '*', '.'))
                {
                    if (part.Length == 0) continue;
                    var m = Regex.Match(part, @"^(\d+)(.*)$");
                    int mult = 1;
                    string body = part;
                    if (m.Success && m.Groups[2].Value.Length > 0) { mult = int.Parse(m.Groups[1].Value); body = m.Groups[2].Value; }
                    int pos = 0;
                    total += mult * ParseGroup(body, ref pos);
                    if (pos != body.Length) throw new FormatException("unexpected '" + body[pos] + "'");
                }
                return total;
            }
            catch (FormatException ex) { error = ex.Message; return 0; }
        }

        static double ParseGroup(string s, ref int pos)
        {
            double sum = 0;
            while (pos < s.Length)
            {
                char ch = s[pos];
                if (ch == '(' || ch == '[')
                {
                    pos++;
                    double inner = ParseGroup(s, ref pos);
                    if (pos >= s.Length || (s[pos] != ')' && s[pos] != ']')) throw new FormatException("missing closing parenthesis");
                    pos++;
                    sum += inner * ReadCount(s, ref pos);
                }
                else if (ch == ')' || ch == ']') return sum;
                else if (char.IsUpper(ch))
                {
                    int start = pos++;
                    while (pos < s.Length && char.IsLower(s[pos])) pos++;
                    var sym = s.Substring(start, pos - start);
                    var el = Data.Atoms.Objects.FirstOrDefault(o => o.Get("symbol") == sym);
                    if (el == null) throw new FormatException("unknown element " + sym);
                    double w = double.Parse(el.Get("weight"), CultureInfo.InvariantCulture);
                    sum += w * ReadCount(s, ref pos);
                }
                else throw new FormatException("unexpected '" + ch + "'");
            }
            return sum;
        }

        static int ReadCount(string s, ref int pos)
        {
            int start = pos;
            while (pos < s.Length && char.IsDigit(s[pos])) pos++;
            return pos > start ? int.Parse(s.Substring(start, pos - start)) : 1;
        }

        public void Activated() { table.Focus(); }
        public void DefinitionsChanged() { }
        public void OptionsChanged() { }
}   }