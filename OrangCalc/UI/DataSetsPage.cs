using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Windows.Forms;
using OrangCalc.Engine;
using DataObject = OrangCalc.Engine.DataObject;

namespace OrangCalc.UI
{
    public sealed class DataSetsPage : Panel, IPage
    {
        readonly MainForm main;
        readonly ListBox sets;
        readonly DataGridView objects, props;
        readonly TextBox search;
        readonly Label desc;
        DataSet Set => sets.SelectedItem as DataSet;
        List<DataObject> shown = new List<DataObject>();

        public DataSetsPage(MainForm m)
        {
            main = m;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);
            var header = Ui.Header("Data Sets");
            header.Dock = DockStyle.Top;
            sets = Ui.ShareWidth(new ListBox { Dock = DockStyle.Left, Width = 200, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text, ItemHeight = 26, DrawMode = DrawMode.OwnerDrawFixed, IntegralHeight = false }, 0.2f, 130, 200);
            sets.DrawItem += (s, e) =>
            {
                if (e.Index < 0) return;
                bool sel = (e.State & DrawItemState.Selected) != 0;
                using (var b = new SolidBrush(sel ? Theme.Selection : Theme.Panel)) e.Graphics.FillRectangle(b, e.Bounds);
                var d = (DataSet)sets.Items[e.Index];
                TextRenderer.DrawText(e.Graphics, d.DisplayTitle + (d.IsUser ? "  (user)" : ""), Theme.UiFont, new Rectangle(e.Bounds.X + 12, e.Bounds.Y, e.Bounds.Width - 12, e.Bounds.Height), Theme.Text, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter);
            };
            sets.SelectedIndexChanged += (s, e) => FillObjects();

            search = Ui.Box("", 200);
            search.Dock = DockStyle.Top;
            HistoryPanel.SetCue(search, "Search objects");
            search.TextChanged += (s, e) => FillObjects();
            desc = new Label { Dock = DockStyle.Top, Height = 34, ForeColor = Theme.TextDim, AutoEllipsis = true, Padding = new Padding(0, 4, 0, 0) };
            objects = new DataGridView { Dock = DockStyle.Fill, ReadOnly = true, AllowUserToAddRows = false, SelectionMode = DataGridViewSelectionMode.FullRowSelect, MultiSelect = false, AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill };
            Theme.StyleGrid(objects);
            objects.SelectionChanged += (s, e) => FillProps();
            objects.CurrentCellChanged += (s, e) => FillProps();

            props = new DataGridView { Dock = DockStyle.Fill, ReadOnly = true, AllowUserToAddRows = false, SelectionMode = DataGridViewSelectionMode.FullRowSelect, MultiSelect = false, AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill, ColumnHeadersVisible = false };
            Theme.StyleGrid(props);
            props.Columns.Add("p", "Property"); props.Columns.Add("v", "Value");
            props.Columns[0].DefaultCellStyle.ForeColor = Theme.TextDim;
            props.CellDoubleClick += (s, e) => InsertProp(false);

            var bar = Ui.FitHeight(new FlowLayoutPanel { Dock = DockStyle.Bottom, Height = 76, BackColor = Theme.Panel, Padding = new Padding(0, 6, 0, 0) });
            void B(string t, Action a, bool p = false) => bar.Controls.Add(new FlatButton(t, (s, e) => a(), p) { Margin = new Padding(0, 0, 6, 6) });
            B("Insert value", () => InsertProp(false), true);
            B("Insert lookup", () => InsertProp(true));
            B("New set…", () => EditSet(null));
            B("Edit set…", () => { if (Set != null && Set.IsUser) EditSet(Set); });
            B("Delete set", () => { if (Set != null && Set.IsUser && Ui.Confirm(main, "Delete data set \"" + Set.DisplayTitle + "\"?")) { main.Calc.DataSets.Remove(Set); main.DefinitionsChanged(); } });
            var card = Ui.ShareWidth(new Card { Dock = DockStyle.Right, Width = 380, Padding = new Padding(12, 10, 6, 8) }, 0.36f, 240, 380);
            card.Controls.Add(props);
            card.Controls.Add(bar);

            var middle = new Panel { Dock = DockStyle.Fill, Padding = new Padding(12, 0, 12, 0), BackColor = Theme.Window };
            middle.Controls.Add(objects);
            middle.Controls.Add(desc);
            middle.Controls.Add(search);
            Controls.Add(middle);
            Controls.Add(card);
            Controls.Add(sets);
            Controls.Add(header);
            FillSets();
        }

        void FillSets()
        {
            var cur = Set;
            sets.Items.Clear();
            foreach (var d in main.Calc.DataSets) sets.Items.Add(d);
            sets.SelectedItem = cur != null && sets.Items.Contains(cur) ? cur : (sets.Items.Count > 0 ? sets.Items[0] : null);
        }

        void FillObjects()
        {
            objects.Columns.Clear();
            objects.Rows.Clear();
            var d = Set;
            if (d == null) return;
            desc.Text = d.Description + (d.IsUser ? "" : "   Use " + d.Name + "(object, property) in expressions.");
            var cols = d.Properties.Where(p => p.IsKey).Take(3).ToList();
            var extra = d.Properties.FirstOrDefault(p => !p.IsKey && !p.Hidden && !p.IsText);
            if (extra != null) cols.Add(extra);
            foreach (var p in cols) objects.Columns.Add(p.Name, p.Title);
            var q = search.Text.Trim().ToLowerInvariant();
            shown = d.Objects.Where(o => q.Length == 0 || o.Values.Values.Any(v => v.ToLowerInvariant().Contains(q))).ToList();
            foreach (var o in shown) objects.Rows.Add(cols.Select(p => (object)(o.Get(p.Name) ?? "")).ToArray());
            FillProps();
        }

        // a grid built while hidden has no current row yet, so fall back to the first object
        DataObject Obj
        {
            get
            {
                int i = objects.CurrentRow?.Index ?? 0;
                return shown != null && i < shown.Count ? shown[i] : null;
            }
        }

        void FillProps()
        {
            props.Rows.Clear();
            var o = Obj;
            if (o == null || Set == null) return;
            foreach (var p in Set.Properties.Where(x => !x.Hidden))
            {
                var v = o.Get(p.Name);
                if (v == null) continue;
                int r = props.Rows.Add(p.Title, v + (p.Unit.Length > 0 ? " " + p.Unit : ""));
                props.Rows[r].Tag = p;
            }
        }

        void InsertProp(bool lookup)
        {
            var o = Obj;
            if (o == null || props.CurrentRow == null) return;
            var p = (DataProperty)props.CurrentRow.Tag;
            var key = Set.Properties.Where(x => x.IsKey).Select(x => o.Get(x.Name)).FirstOrDefault(x => x != null) ?? "";
            if (lookup || p.IsText) { main.InsertIntoExpression(Set.Name + "(" + (key.Contains(" ") ? "\"" + key + "\"" : key) + ", " + p.Name + ")"); return; }
            main.InsertIntoExpression(o.Get(p.Name) + (p.Unit.Length > 0 ? " " + p.Unit : ""));
        }

        void EditSet(DataSet d)
        {
            using (var dlg = new DataSetEditDialog(main.Calc, d))
                if (dlg.ShowDialog(main) == DialogResult.OK) { main.DefinitionsChanged(); FillSets(); sets.SelectedItem = dlg.Result; }
        }

        public void Activated() { }
        public void DefinitionsChanged() { FillSets(); }
        public void OptionsChanged() { }
    }

    public sealed class DataSetEditDialog : DialogBase
    {
        public DataSet Result;
        readonly DataGridView pgrid, ogrid;

        public DataSetEditDialog(Calculator c, DataSet d) : base(d == null ? "New Data Set" : "Edit Data Set", 760)
        {
            var name = AddRow("Name:", Ui.Box(d?.Name ?? ""));
            var title = AddRow("Title:", Ui.Box(d?.Title ?? ""));
            var desc = AddRow("Description:", Ui.Box(d?.Description ?? ""));
            AddRow(null, new Label { Text = "Properties (name, title, unit, text, key)", AutoSize = true, ForeColor = Theme.Accent, Margin = new Padding(0, 10, 0, 4) });
            pgrid = new DataGridView { Width = 720, Height = 150, AllowUserToAddRows = true };
            Theme.StyleGrid(pgrid);
            pgrid.Columns.Add("n", "Name"); pgrid.Columns.Add("t", "Title"); pgrid.Columns.Add("u", "Unit");
            pgrid.Columns.Add(new DataGridViewCheckBoxColumn { Name = "x", HeaderText = "Text" });
            pgrid.Columns.Add(new DataGridViewCheckBoxColumn { Name = "k", HeaderText = "Key" });
            if (d != null) foreach (var p in d.Properties) pgrid.Rows.Add(p.Name, p.Title, p.Unit, p.IsText, p.IsKey);
            else pgrid.Rows.Add("name", "Name", "", true, true);
            AddRow(null, pgrid);
            AddRow(null, new Label { Text = "Objects (one row per object, columns follow the properties above)", AutoSize = true, ForeColor = Theme.Accent, Margin = new Padding(0, 10, 0, 4) });
            ogrid = new DataGridView { Width = 720, Height = 200, AllowUserToAddRows = true };
            Theme.StyleGrid(ogrid);
            AddRow(null, ogrid);
            void SyncColumns()
            {
                var names = pgrid.Rows.Cast<DataGridViewRow>().Where(r => !r.IsNewRow).Select(r => r.Cells[0].Value?.ToString() ?? "").Where(n => n.Length > 0).ToList();
                foreach (var n in names) if (!ogrid.Columns.Contains(n)) ogrid.Columns.Add(n, n);
                foreach (var col in ogrid.Columns.Cast<DataGridViewColumn>().ToList()) if (!names.Contains(col.Name)) ogrid.Columns.Remove(col);
            }
            SyncColumns();
            if (d != null) foreach (var o in d.Objects) ogrid.Rows.Add(ogrid.Columns.Cast<DataGridViewColumn>().Select(col => (object)(o.Get(col.Name) ?? "")).ToArray());
            pgrid.CellEndEdit += (s, e) => SyncColumns();
            OkCancel(() =>
            {
                var n = name.Text.Trim();
                if (n.Length == 0 || !n.All(Lexer.IsIdentPart)) { Fail("Invalid name."); return false; }
                var ds = d ?? new DataSet { IsUser = true, Category = "User data sets" };
                ds.Name = n; ds.Names = new List<string> { n }; ds.Title = title.Text.Trim().Length > 0 ? title.Text.Trim() : n; ds.Description = desc.Text; ds.IsUser = true;
                ds.Properties = pgrid.Rows.Cast<DataGridViewRow>().Where(r => !r.IsNewRow && (r.Cells[0].Value?.ToString() ?? "").Length > 0)
                    .Select(r => new DataProperty(r.Cells[0].Value.ToString(), r.Cells[1].Value?.ToString() ?? r.Cells[0].Value.ToString(), r.Cells[2].Value?.ToString() ?? "", r.Cells[3].Value is bool b1 && b1) { IsKey = r.Cells[4].Value is bool b2 && b2 }).ToList();
                if (!ds.Properties.Any(p => p.IsKey) && ds.Properties.Count > 0) ds.Properties[0].IsKey = true;
                ds.Objects = new List<DataObject>();
                foreach (DataGridViewRow r in ogrid.Rows)
                {
                    if (r.IsNewRow) continue;
                    var o = new DataObject();
                    foreach (DataGridViewColumn col in ogrid.Columns) { var v = r.Cells[col.Index].Value?.ToString(); if (!string.IsNullOrEmpty(v)) o.Values[col.Name] = v; }
                    if (o.Values.Count > 0) ds.Objects.Add(o);
                }
                if (d == null) c.DataSets.Add(ds);
                Result = ds;
                return true;
            });
        }
    }

    public static class CsvTools
    {
        public static void Import(MainForm main)
        {
            string file;
            using (var d = new OpenFileDialog { Filter = "CSV files (*.csv;*.txt)|*.csv;*.txt|All files|*.*" })
            {
                if (d.ShowDialog(main) != DialogResult.OK) return;
                file = d.FileName;
            }
            using (var dlg = new CsvImportDialog(Path.GetFileNameWithoutExtension(file)))
            {
                if (dlg.ShowDialog(main) != DialogResult.OK) return;
                try
                {
                    var lines = File.ReadAllLines(file).Skip(Math.Max(0, dlg.FirstRow - 1)).Where(l => l.Trim().Length > 0).ToList();
                    List<string> headings = null;
                    if (dlg.Headings && lines.Count > 0) { headings = Split(lines[0], dlg.Delimiter); lines.RemoveAt(0); }
                    var rows = lines.Select(l => Split(l, dlg.Delimiter)).ToList();
                    int cols = rows.Count == 0 ? 0 : rows.Max(r => r.Count);
                    var c = main.Calc;
                    var created = main.Svc.Run(() =>
                    {
                        Func<string, Expr> val = t => { try { return Funcs.Unhold(c.EvalText(t.Length == 0 ? "0" : t)); } catch (CalcException) { return new StrE(t); } };
                        var names = new List<string>();
                        if (dlg.AsMatrix)
                        {
                            var m = new MatE(rows.Select(r => Enumerable.Range(0, cols).Select(j => j < r.Count ? val(r[j]) : Engine.Expr.Zero).ToList()).ToList(), rows.Count == 1);
                            c.AddVariable(new Variable { Name = dlg.Name, Names = { dlg.Name }, Value = m, ValueText = Formatter.Ascii(m, c.Opts), IsUser = true, Category = "Matrices", Title = dlg.Name });
                            names.Add(dlg.Name);
                        }
                        else
                        {
                            for (int j = 0; j < cols; j++)
                            {
                                var vname = headings != null && j < headings.Count && headings[j].Length > 0 && headings[j].All(Lexer.IsIdentPart) && Lexer.IsIdentStart(headings[j][0]) ? headings[j] : dlg.Name + (j + 1);
                                var v = MatE.Vector(rows.Select(r => j < r.Count ? val(r[j]) : Engine.Expr.Zero).ToList());
                                c.AddVariable(new Variable { Name = vname, Names = { vname }, Value = v, ValueText = Formatter.Ascii(v, c.Opts), IsUser = true, Category = "Vectors", Title = vname });
                                names.Add(vname);
                            }
                        }
                        return names;
                    }).Result;
                    main.DefinitionsChanged();
                    Ui.Message(main, "Imported " + string.Join(", ", created) + ".");
                }
                catch (Exception ex) { Ui.Message(main, "Could not import from file\n" + file + "\n" + ex.Message, "Error"); }
            }
        }

        static List<string> Split(string line, string delim)
        {
            var res = new List<string>();
            var sb = new StringBuilder();
            bool q = false;
            for (int i = 0; i < line.Length; i++)
            {
                char ch = line[i];
                if (ch == '"') { q = !q; continue; }
                if (!q && (delim == " " ? char.IsWhiteSpace(ch) : string.CompareOrdinal(line, i, delim, 0, delim.Length) == 0))
                {
                    res.Add(sb.ToString().Trim()); sb.Clear();
                    i += delim.Length - 1;
                    if (delim == " ") while (i + 1 < line.Length && char.IsWhiteSpace(line[i + 1])) i++;
                    continue;
                }
                sb.Append(ch);
            }
            res.Add(sb.ToString().Trim());
            return res;
        }

        public static void Export(MainForm main)
        {
            using (var dlg = new CsvExportDialog(main))
            {
                if (dlg.ShowDialog(main) != DialogResult.OK) return;
                Expr value = dlg.Value;
                if (!(value is MatE m)) { Ui.Message(main, "No matrix or vector variable with the entered name was found.", "Error"); return; }
                using (var sd = new SaveFileDialog { Filter = "CSV file (*.csv)|*.csv", DefaultExt = "csv" })
                {
                    if (sd.ShowDialog(main) != DialogResult.OK) return;
                    try
                    {
                        var o = main.Calc.Opts.Clone();
                        o.Unicode = false; o.Grouping = DigitGrouping.None;
                        var sb = new StringBuilder();
                        var rows = m.IsVector ? m.Rows[0].Select(x => new List<Expr> { x }).ToList() : m.Rows;
                        foreach (var r in rows) sb.AppendLine(string.Join(dlg.Delimiter, r.Select(x => Formatter.Format(x, o, out _))));
                        File.WriteAllText(sd.FileName, sb.ToString());
                    }
                    catch (Exception ex) { Ui.Message(main, "Could not export to file\n" + sd.FileName + "\n" + ex.Message, "Error"); }
                }
            }
        }
    }

    public sealed class CsvImportDialog : DialogBase
    {
        public string Delimiter = ",";
        public int FirstRow = 1;
        public bool Headings, AsMatrix = true;
        public new string Name;

        public CsvImportDialog(string suggested) : base("Import CSV File", 460)
        {
            var name = AddRow("Name:", Ui.Box(new string(suggested.Where(Lexer.IsIdentPart).ToArray()) is string s && s.Length > 0 && Lexer.IsIdentStart(s[0]) ? s : "data"));
            var asWhat = AddRow("Import as:", Ui.Combo(new[] { "Matrix", "Vectors (one per column)" }));
            var delim = AddRow("Delimiter:", Ui.Combo(new[] { "Comma", "Tabulator", "Semicolon", "Space", "Other" }));
            var other = AddRow("Other delimiter:", Ui.Box(""));
            var first = AddRow("First row:", Ui.Number(1, 1, 1000000));
            var head = AddRow(null, Ui.Check("Includes headings", false));
            OkCancel(() =>
            {
                Name = name.Text.Trim();
                if (Name.Length == 0 || !Lexer.IsIdentStart(Name[0]) || !Name.All(Lexer.IsIdentPart)) { Fail("Invalid name."); return false; }
                AsMatrix = asWhat.SelectedIndex == 0;
                Delimiter = new[] { ",", "\t", ";", " ", other.Text }[delim.SelectedIndex];
                if (Delimiter.Length == 0) Delimiter = ",";
                FirstRow = (int)first.Value;
                Headings = head.Checked;
                return true;
            }, "Import");
        }
    }

    public sealed class CsvExportDialog : DialogBase
    {
        public Expr Value;
        public string Delimiter = ",";

        public CsvExportDialog(MainForm main) : base("Export CSV File", 460)
        {
            var src = AddRow("Export:", Ui.Combo(new[] { "Current result", "Matrix/vector variable" }));
            var name = AddRow("Variable:", Ui.Box(""));
            var delim = AddRow("Delimiter:", Ui.Combo(new[] { "Comma", "Tabulator", "Semicolon", "Space" }));
            OkCancel(() =>
            {
                Delimiter = new[] { ",", "\t", ";", " " }[delim.SelectedIndex];
                if (src.SelectedIndex == 0) Value = main.CalcView.Last?.Value;
                else
                {
                    var v = main.Calc.FindVariable(name.Text.Trim());
                    Value = v?.Value;
                    if (v != null && v.Value == null && v.ValueText.Length > 0) { try { Value = main.Svc.Run(() => main.Calc.EvalText(v.Names[0])).Result; } catch (AggregateException) { } }
                }
                return true;
            }, "Export");
}   }   }