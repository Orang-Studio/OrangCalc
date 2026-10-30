using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public enum ItemKind { Function, Variable, Unit }

    public sealed class ItemsPage : Panel, IPage
    {
        readonly MainForm main;
        readonly ItemKind kind;
        readonly ListBox categories;
        readonly TextBox search;
        readonly DataGridView grid;
        readonly RichTextBox details;
        readonly FlowLayoutPanel actions;
        FlatButton insertBtn, editBtn, deleteBtn, activeBtn, favBtn;
        TextBox convValue, convResult;
        DarkCombo convTo;
        Label convFrom;
        List<Item> current = new List<Item>();

        Calculator C => main.Calc;

        public ItemsPage(MainForm m, ItemKind k)
        {
            main = m; kind = k;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);

            var title = Ui.Header(k == ItemKind.Function ? "Functions" : k == ItemKind.Variable ? "Variables and Constants" : "Units");
            title.Dock = DockStyle.Top;

            categories = Ui.ShareWidth(new ListBox { Dock = DockStyle.Left, Width = 210, DrawMode = DrawMode.OwnerDrawFixed, ItemHeight = 28, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text, IntegralHeight = false }, 0.2f, 130, 210);
            categories.DrawItem += (s, e) =>
            {
                if (e.Index < 0) return;
                bool sel = (e.State & DrawItemState.Selected) != 0;
                using (var b = new SolidBrush(sel ? Theme.Selection : Theme.Panel)) e.Graphics.FillRectangle(b, e.Bounds);
                if (sel) using (var b = new SolidBrush(Theme.Accent)) e.Graphics.FillRectangle(b, e.Bounds.X, e.Bounds.Y + 6, 3, e.Bounds.Height - 12);
                TextRenderer.DrawText(e.Graphics, categories.Items[e.Index].ToString(), Theme.UiFont, new Rectangle(e.Bounds.X + 12, e.Bounds.Y, e.Bounds.Width - 14, e.Bounds.Height), Theme.Text, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.EndEllipsis);
            };
            categories.SelectedIndexChanged += (s, e) => FillGrid();
            Theme.DarkScrollbars(categories);

            search = new TextBox { Dock = DockStyle.Top, BorderStyle = BorderStyle.FixedSingle, BackColor = Theme.Raised, ForeColor = Theme.Text };
            HistoryPanel.SetCue(search, "Search");
            search.TextChanged += (s, e) => FillGrid();

            grid = new DataGridView { Dock = DockStyle.Fill, ReadOnly = true, AllowUserToAddRows = false, AllowUserToDeleteRows = false, SelectionMode = DataGridViewSelectionMode.FullRowSelect, MultiSelect = false, AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill };
            Theme.StyleGrid(grid);
            grid.Columns.Add("title", "Title");
            grid.Columns.Add("name", "Name");
            if (k != ItemKind.Function) grid.Columns.Add("value", k == ItemKind.Unit ? "Definition" : "Value");
            grid.Columns[0].FillWeight = 45;
            grid.Columns[1].FillWeight = 25;
            grid.SelectionChanged += (s, e) => ShowDetails();
            grid.CellDoubleClick += (s, e) => InsertSelected();

            details = new RichTextBox { Dock = DockStyle.Fill, ReadOnly = true, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text, Font = Theme.UiFont };
            Theme.DarkScrollbars(details);
            actions = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 40, BackColor = Theme.Panel, Padding = new Padding(8, 6, 8, 0) });
            insertBtn = new FlatButton(k == ItemKind.Function ? "Insert…" : "Insert", (s, e) => InsertSelected(), true);
            editBtn = new FlatButton("Edit…", (s, e) => EditSelected());
            deleteBtn = new FlatButton("Delete", (s, e) => DeleteSelected());
            activeBtn = new FlatButton("Deactivate", (s, e) => ToggleActive());
            favBtn = new FlatButton("Favorite", (s, e) => ToggleFavorite());
            var newBtn = new FlatButton("New…", (s, e) => NewItem());
            foreach (var b in new Control[] { insertBtn, newBtn, editBtn, deleteBtn, activeBtn, favBtn }) { b.Margin = new Padding(0, 0, 6, 6); actions.Controls.Add(b); }
            if (k == ItemKind.Variable)
            {
                var nu = new FlatButton("New Unknown…", (s, e) => { main.NewUnknown(); }) { Margin = new Padding(0, 0, 6, 6) };
                actions.Controls.Add(nu);
            }

            var detailCard = Ui.ShareWidth(new Card { Dock = DockStyle.Right, Width = 380, Padding = new Padding(12, 10, 4, 4) }, 0.36f, 240, 380);
            detailCard.Controls.Add(details);
            detailCard.Controls.Add(actions);

            var middle = new Panel { Dock = DockStyle.Fill, Padding = new Padding(12, 0, 12, 0), BackColor = Theme.Window };
            var searchWrap = new Panel { Dock = DockStyle.Top, Height = 32, BackColor = Theme.Window, Padding = new Padding(0, 0, 0, 8) };
            searchWrap.Controls.Add(search);
            middle.Controls.Add(grid);
            middle.Controls.Add(searchWrap);

            var content = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window };
            content.Controls.Add(middle);
            content.Controls.Add(detailCard);
            content.Controls.Add(categories);

            if (k == ItemKind.Unit) content.Controls.Add(BuildConverter());
            Controls.Add(content);
            Controls.Add(title);
            FillCategories();
        }

        Control BuildConverter()
        {
            var card = new Card { Dock = DockStyle.Bottom, Height = 58, Padding = new Padding(12, 12, 12, 8) };
            var row = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Top, BackColor = Theme.Panel });
            row.Resize += (s, e) => card.Height = row.Height + card.Padding.Vertical;
            convValue = Ui.Box("1", 110);
            convFrom = new Label { AutoSize = true, ForeColor = Theme.TokUnit, Font = Theme.UiBold, Margin = new Padding(6, 6, 10, 0), Text = "m" };
            convResult = Ui.Box("", 200);
            convResult.ReadOnly = true;
            convTo = Ui.Combo(new string[0], 0, 230);
            var go = new FlatButton("Convert", (s, e) => Convert(), true);
            var swap = new FlatButton("Swap", (s, e) => SwapUnits());
            var ins = new FlatButton("Insert", (s, e) => { if (convResult.Text.Length > 0) main.InsertIntoExpression(convResult.Text); });
            row.Controls.AddRange(new Control[] { Ui.Label("Convert", true), convValue, convFrom, Ui.Label("to", true), convTo, Ui.Label("=", true), convResult, go, swap, ins });
            foreach (Control c in row.Controls) if (c is FlatButton) c.Margin = new Padding(6, 0, 0, 0);
            convValue.KeyDown += (s, e) => { if (e.KeyCode == Keys.Enter) { Convert(); e.SuppressKeyPress = true; } };
            convTo.SelectedIndexChanged += (s, e) => Convert();
            card.Controls.Add(row);
            return card;
        }

        void FillConverterTargets(Unit u)
        {
            if (convTo == null || u == null) return;
            convFrom.Text = u.Abbrev;
            convTo.Items.Clear();
            var same = C.Units.Where(x => x.Active && x != u && x.Dim.SameAs(u.Dim)).OrderBy(x => x.DisplayTitle).ToList();
            foreach (var x in same) convTo.Items.Add(new UnitChoice(x));
            if (convTo.Items.Count > 0) convTo.SelectedIndex = 0;
            Convert();
        }

        sealed class UnitChoice
        {
            public readonly Unit U;
            public UnitChoice(Unit u) { U = u; }
            public override string ToString() => U.DisplayTitle + " (" + U.Abbrev + ")";
        }

        void Convert()
        {
            if (convTo?.SelectedItem is UnitChoice uc)
            {
                var expr = "(" + convValue.Text + ") " + convFrom.Text + " to " + uc.U.Abbrev;
                main.Svc.Run(() => Display.From(C.Calculate(expr, 3000, true), C)).ContinueWith(t => BeginInvoke((Action)(() =>
                {
                    if (t.IsFaulted) return;
                    convResult.Text = t.Result.Error ?? t.Result.Text;
                })));
            }
        }

        void SwapUnits()
        {
            if (!(convTo?.SelectedItem is UnitChoice uc)) return;
            var from = C.FindUnit(convFrom.Text, out _, false);
            var res = convResult.Text;
            FillConverterTargets(uc.U);
            for (int i = 0; i < convTo.Items.Count; i++) if (((UnitChoice)convTo.Items[i]).U == from) { convTo.SelectedIndex = i; break; }
        }

        IEnumerable<Item> All()
        {
            switch (kind)
            {
                case ItemKind.Function: return C.Functions.Where(f => !f.Name.StartsWith("_") && !f.Hidden);
                case ItemKind.Variable: return C.Variables.Where(v => !v.Hidden);
                default: return C.Units.Where(u => !u.Hidden);
            }
        }

        List<string> Favorites => kind == ItemKind.Function ? main.Settings.FavoriteFunctions : kind == ItemKind.Variable ? main.Settings.FavoriteVariables : main.Settings.FavoriteUnits;

        string KeyOf(Item i) => i is Unit u ? u.Abbrev : i.Names[0];

        void FillCategories()
        {
            var sel = categories.SelectedItem as string;
            categories.Items.Clear();
            categories.Items.Add("All");
            categories.Items.Add("Favorites");
            if (All().Any(i => i.IsUser)) categories.Items.Add("User " + (kind == ItemKind.Function ? "functions" : kind == ItemKind.Variable ? "variables" : "units"));
            if (All().Any(i => !i.Active)) categories.Items.Add("Inactive");
            foreach (var c in All().Select(i => i.Category.Length == 0 ? "Other" : i.Category).Distinct().OrderBy(x => x)) categories.Items.Add(c);
            int idx = sel != null ? categories.Items.IndexOf(sel) : 0;
            categories.SelectedIndex = idx >= 0 ? idx : 0;
        }

        void FillGrid()
        {
            var cat = categories.SelectedItem as string ?? "All";
            var q = search.Text.Trim().ToLowerInvariant();
            IEnumerable<Item> items = All();
            if (cat == "Favorites") items = items.Where(i => Favorites.Contains(KeyOf(i)));
            else if (cat.StartsWith("User ")) items = items.Where(i => i.IsUser);
            else if (cat == "Inactive") items = items.Where(i => !i.Active);
            else if (cat != "All") items = items.Where(i => (i.Category.Length == 0 ? "Other" : i.Category) == cat);
            if (q.Length > 0) items = items.Where(i => i.DisplayTitle.ToLowerInvariant().Contains(q) || i.Names.Any(n => n.ToLowerInvariant().Contains(q)) || i.Description.ToLowerInvariant().Contains(q));
            current = items.OrderBy(i => i.DisplayTitle).ToList();
            grid.SuspendLayout();
            grid.Rows.Clear();
            foreach (var i in current)
            {
                var cells = new List<object> { i.DisplayTitle + (i.Active ? "" : "  (inactive)"), i is MathFunction f ? f.Signature : i is Unit u ? string.Join(", ", u.Names.Take(3)) : string.Join(", ", i.Names.Take(3)) };
                if (kind == ItemKind.Variable) cells.Add(ValueText((Variable)i));
                if (kind == ItemKind.Unit) cells.Add(((Unit)i).IsBase ? "base unit" : ((Unit)i).Definition.StartsWith("@") ? "offset scale" : ((Unit)i).Definition);
                grid.Rows.Add(cells.ToArray());
            }
            grid.ResumeLayout();
            if (grid.Rows.Count > 0) { grid.ClearSelection(); grid.Rows[0].Selected = true; grid.CurrentCell = grid.Rows[0].Cells[0]; }
            ShowDetails();
        }

        string ValueText(Variable v)
        {
            if (v.IsUnknown) return "unknown (" + v.AType.ToString().ToLowerInvariant() + ")";
            if (v.Dynamic != null && v.ValueText.Length == 0) return v.KeepSymbolic ? "constant" : "dynamic";
            if (v.ValueText.Length > 0) return v.ValueText;
            return v.Value != null ? Formatter.Ascii(v.Value, C.Opts) : "";
        }

        Item Selected => grid.SelectedRows.Count > 0 && grid.SelectedRows[0].Index < current.Count ? current[grid.SelectedRows[0].Index] : null;

        void Add(string text, Color color, Font font)
        {
            details.SelectionStart = details.TextLength;
            details.SelectionColor = color;
            details.SelectionFont = font;
            details.AppendText(text);
        }

        void ShowDetails()
        {
            details.Clear();
            var it = Selected;
            insertBtn.Enabled = editBtn.Enabled = activeBtn.Enabled = favBtn.Enabled = it != null;
            deleteBtn.Enabled = it != null && it.IsUser;
            if (it == null) return;
            activeBtn.Text = it.Active ? "Deactivate" : "Activate";
            favBtn.Text = Favorites.Contains(KeyOf(it)) ? "Unfavorite" : "Favorite";
            Add(it.DisplayTitle + "\n", Theme.Text, Theme.HeaderFont);
            if (it is MathFunction f)
            {
                Add(f.Signature + "\n\n", Theme.TokFunction, Theme.MonoFont);
                if (f.Description.Length > 0) Add(f.Description + "\n\n", Theme.Text, Theme.UiFont);
                if (f.Args.Count > 0)
                {
                    Add("Arguments\n", Theme.TextDim, Theme.UiBold);
                    for (int i = 0; i < f.Args.Count; i++)
                    {
                        var a = f.Args[i];
                        Add("  " + (i + 1) + ". " + a.Name + ": " + TypeName(a.Type) + (i >= f.MinArgs ? (a.Default != null ? " (default " + a.Default + ")" : " (optional)") : "") + "\n", Theme.Text, Theme.UiFont);
                    }
                    if (f.MaxArgs < 0) Add("  … more arguments allowed\n", Theme.TextFaint, Theme.UiFont);
                    Add("\n", Theme.Text, Theme.UiFont);
                }
                if (f.IsUser) Add("Expression: " + f.Formula + "\n", Theme.Text, Theme.MonoFont);
                if (f.Example.Length > 0) Add("Example: " + f.Example + "\n", Theme.TextDim, Theme.UiFont);
                Add("Names: " + string.Join(", ", f.Names) + "\n", Theme.TextDim, Theme.UiFont);
            }
            else if (it is Variable v)
            {
                Add(string.Join(", ", v.Names) + "\n\n", Theme.TokVariable, Theme.MonoFont);
                if (v.IsUnknown) Add("Unknown variable, assumed " + v.AType.ToString().ToLowerInvariant() + (v.ASign != AssumeSign.Unknown ? ", " + ExpressionBox.Words(v.ASign.ToString()).ToLowerInvariant() : "") + ".\n", Theme.Text, Theme.UiFont);
                else
                {
                    Add("Value: ", Theme.TextDim, Theme.UiFont);
                    string val;
                    try
                    {
                        var e = v.Dynamic != null || v.Value == null ? C.EvalText(v.Names[0]) : v.Value;
                        val = Formatter.Format(e, C.Opts, out bool r);
                        if (v.IsApproximate || r) val = "≈ " + val;
                    }
                    catch (CalcException ex) { val = ex.Message; }
                    Add(val + "\n", Theme.Text, Theme.UiBold);
                    if (v.ValueText.Length > 0) Add("Definition: " + v.ValueText + "\n", Theme.TextDim, Theme.MonoFont);
                }
                if (v.Description.Length > 0) Add("\n" + v.Description + "\n", Theme.Text, Theme.UiFont);
            }
            else if (it is Unit u)
            {
                Add(string.Join(", ", u.Names) + (u.Plural.Length > 0 && !u.Names.Contains(u.Plural) ? ", " + u.Plural : "") + "\n\n", Theme.TokUnit, Theme.MonoFont);
                if (u.IsBase) Add("SI base unit of " + u.Category.ToLowerInvariant() + ".\n", Theme.Text, Theme.UiFont);
                else if (u.HasOffset) Add("Temperature scale: K = " + (u.Factor.IsOne ? "" : Formatter.PlainText(new NumE(new Cx(u.Factor))) + " × ") + "value + " + Formatter.PlainText(new NumE(new Cx(u.Offset))) + "\n", Theme.Text, Theme.UiFont);
                else
                {
                    Add("1 " + u.Abbrev + " = " + u.Definition + "\n", Theme.Text, Theme.UiFont);
                    try
                    {
                        var bu = Units.BaseUnits(u.Dim);
                        var e = Simp.Mul(new NumE(new Cx(u.Factor)), bu);
                        Add("1 " + u.Abbrev + " = " + Formatter.Format(e, C.Opts, out _) + " (base units)\n", Theme.TextDim, Theme.UiFont);
                    }
                    catch (Exception) { }
                }
                Add("\nSystem: " + u.System + "\nPrefixes: " + u.Prefixes.ToString().ToLowerInvariant() + "\n", Theme.TextDim, Theme.UiFont);
                FillConverterTargets(u);
            }
            Add("Category: " + it.Category + (it.IsUser ? " (user defined)" : "") + "\n", Theme.TextFaint, Theme.UiFont);
        }

        static string TypeName(ArgType t)
        {
            switch (t)
            {
                case ArgType.Number: return "number";
                case ArgType.Integer: return "integer";
                case ArgType.Real: return "real number";
                case ArgType.NonNegInt: return "non-negative integer";
                case ArgType.PositiveInt: return "positive integer";
                case ArgType.Matrix: return "matrix";
                case ArgType.Vector: return "vector";
                case ArgType.Text: return "text";
                case ArgType.Boolean: return "boolean (0 or 1)";
                case ArgType.Date: return "date";
                case ArgType.Angle: return "angle";
                case ArgType.Unevaluated: return "expression or variable name";
                default: return "free value";
            }
        }

        void InsertSelected()
        {
            var it = Selected;
            if (it == null) return;
            if (it is MathFunction f) main.InsertFunctionWithDialog(f);
            else if (it is Unit u) { main.AddRecent(main.Settings.RecentUnits, u.Abbrev); main.InsertIntoExpression(u.Abbrev); }
            else { main.AddRecent(main.Settings.RecentVariables, it.Names[0]); main.InsertIntoExpression(it.Names[0]); }
        }

        void NewItem()
        {
            switch (kind)
            {
                case ItemKind.Function: main.NewFunction(); break;
                case ItemKind.Variable: main.NewVariable(); break;
                default: main.NewUnit(); break;
            }
        }

        void EditSelected()
        {
            var it = Selected;
            if (it == null) return;
            DialogResult r;
            if (it is MathFunction f) using (var d = new FunctionEditDialog(C, f)) r = d.ShowDialog(main);
            else if (it is Variable v && v.IsUnknown) using (var d = new UnknownEditDialog(C, v)) r = d.ShowDialog(main);
            else if (it is Variable v2) using (var d = new VariableEditDialog(C, v2)) r = d.ShowDialog(main);
            else using (var d = new UnitEditDialog(C, (Unit)it)) r = d.ShowDialog(main);
            if (r == DialogResult.OK) main.DefinitionsChanged();
        }

        void DeleteSelected()
        {
            var it = Selected;
            if (it == null || !it.IsUser) return;
            if (!Ui.Confirm(main, "Delete \"" + it.DisplayTitle + "\"?")) return;
            if (it is MathFunction f) C.RemoveFunction(f);
            else if (it is Variable v) C.RemoveVariable(v);
            else if (it is Unit u) C.RemoveUnit(u);
            main.DefinitionsChanged();
        }

        void ToggleActive()
        {
            var it = Selected;
            if (it == null) return;
            it.Active = !it.Active;
            main.DefinitionsChanged();
        }

        void ToggleFavorite()
        {
            var it = Selected;
            if (it == null) return;
            var k = KeyOf(it);
            if (Favorites.Contains(k)) Favorites.Remove(k); else Favorites.Add(k);
            main.MarkDirty();
            ShowDetails();
        }

        public void Activated() { if (Selected == null && grid.Rows.Count > 0) { grid.Rows[0].Selected = true; } ShowDetails(); }
        public void DefinitionsChanged() { FillCategories(); FillGrid(); }
        public void OptionsChanged() { ShowDetails(); }
    }
}
