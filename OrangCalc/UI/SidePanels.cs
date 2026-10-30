using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class HistoryPanel : Panel
    {
        public readonly List<HistoryEntry> Entries;
        readonly ListBox list;
        readonly TextBox search;
        readonly AppSettings settings;
        List<HistoryEntry> shown = new List<HistoryEntry>();
        DateTime? dateFilter;
        public event Action<string> InsertText;
        public event Action<HistoryEntry> InsertValue;
        public event EventHandler Changed;
        public Font EntryFont = Theme.HistoryFont;
        public bool Colorize = true;

        public HistoryPanel(List<HistoryEntry> entries, AppSettings s)
        {
            Entries = entries; settings = s;
            BackColor = Theme.Panel;
            search = new TextBox { Dock = DockStyle.Top, BorderStyle = BorderStyle.FixedSingle, BackColor = Theme.Raised, ForeColor = Theme.Text, Font = Theme.UiFont };
            SetCue(search, "Search history");
            search.TextChanged += (o, e) => Refill();
            var searchWrap = new Panel { Dock = DockStyle.Top, Height = 34, Padding = new Padding(8, 6, 8, 4), BackColor = Theme.Panel };
            searchWrap.Controls.Add(search);
            list = new ListBox
            {
                Dock = DockStyle.Fill, DrawMode = DrawMode.OwnerDrawVariable, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text,
                SelectionMode = SelectionMode.MultiExtended, IntegralHeight = false
            };
            list.MeasureItem += MeasureEntry;
            list.DrawItem += DrawEntry;
            list.DoubleClick += (o, e) =>
            {
                if (!(list.SelectedItem is HistoryEntry h)) return;
                if ((ModifierKeys & Keys.Control) != 0 || h.IsError) InsertText?.Invoke(h.Expression);
                else InsertValue?.Invoke(h);
            };
            list.Resize += (o, e) => { list.Invalidate(); Refill(); };
            list.KeyDown += (o, e) =>
            {
                if (e.KeyCode == Keys.Delete) RemoveSelected();
                if (e.Control && e.KeyCode == Keys.C) CopySelected(false);
                if (e.Control && e.KeyCode == Keys.A) SelectAll();
            };
            list.ContextMenuStrip = BuildMenu();
            Controls.Add(list);
            Controls.Add(searchWrap);
            Theme.DarkScrollbars(list);
            Refill();
        }

        [System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
        static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wp, string lp);

        public static void SetCue(TextBox t, string cue)
        {
            if (t.IsHandleCreated) SendMessage(t.Handle, 0x1501, (IntPtr)1, cue);
            else t.HandleCreated += (o, e) => SendMessage(t.Handle, 0x1501, (IntPtr)1, cue);
        }

        ContextMenuStrip BuildMenu()
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            m.Items.Add("Insert Value", null, (s, e) => { if (list.SelectedItem is HistoryEntry h) InsertValue?.Invoke(h); });
            m.Items.Add("Insert Text", null, (s, e) => { if (list.SelectedItem is HistoryEntry h) InsertText?.Invoke(h.Expression); });
            m.Items.Add(new ToolStripSeparator());
            ((ToolStripMenuItem)m.Items.Add("Copy", null, (s, e) => CopySelected(false))).ShortcutKeyDisplayString = "Ctrl+C";
            m.Items.Add("Copy Unformatted ASCII", null, (s, e) => CopySelected(true));
            m.Items.Add("Copy Expression", null, (s, e) => { if (list.SelectedItem is HistoryEntry h) SafeClipboard(h.Expression); });
            m.Items.Add(new ToolStripSeparator());
            var comment = m.Items.Add("Add Comment…", null, (s, e) => EditComment());
            var protect = new ToolStripMenuItem("Protect");
            protect.Click += (s, e) => { foreach (var h in Selected()) h.Protected = !protect.Checked; list.Invalidate(); Changed?.Invoke(this, EventArgs.Empty); };
            m.Items.Add(protect);
            m.Items.Add("Move to Top", null, (s, e) => MoveToTop());
            ((ToolStripMenuItem)m.Items.Add("Remove", null, (s, e) => RemoveSelected())).ShortcutKeyDisplayString = "Del";
            m.Items.Add("Clear", null, (s, e) => ClearUnprotected());
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Search…", null, (s, e) => search.Focus());
            m.Items.Add("Search by Date…", null, (s, e) => SearchByDate());
            m.Items.Add("Show All Dates", null, (s, e) => { dateFilter = null; Refill(); });
            ((ToolStripMenuItem)m.Items.Add("Select All", null, (s, e) => SelectAll())).ShortcutKeyDisplayString = "Ctrl+A";
            m.Opening += (s, e) =>
            {
                var h = list.SelectedItem as HistoryEntry;
                comment.Text = h != null && h.Comment.Length > 0 ? "Edit Comment…" : "Add Comment…";
                protect.Checked = h != null && h.Protected;
                foreach (ToolStripItem it in m.Items) if (it is ToolStripMenuItem mi && mi.Text != "Clear" && mi.Text != "Search…" && mi.Text != "Search by Date…" && mi.Text != "Show All Dates" && mi.Text != "Select All") mi.Enabled = h != null;
            };
            return m;
        }

        IEnumerable<HistoryEntry> Selected() => list.SelectedItems.Cast<HistoryEntry>().ToList();

        void SelectAll() { for (int i = 0; i < list.Items.Count; i++) list.SetSelected(i, true); }

        public static void SafeClipboard(string text)
        {
            if (string.IsNullOrEmpty(text)) return;
            try { Clipboard.SetText(text); } catch (Exception) { }
        }

        void CopySelected(bool ascii)
        {
            var parts = Selected().Select(h => h.IsError || h.IsMessage ? h.Expression + "  " + h.Result : (h.Expression + " = " + (ascii ? h.ResultAscii : h.Result)));
            SafeClipboard(string.Join(Environment.NewLine, parts));
        }

        void EditComment()
        {
            if (!(list.SelectedItem is HistoryEntry h)) return;
            var t = InputDialog.Ask(FindForm(), "Comment", "Comment:", h.Comment, true);
            if (t == null) return;
            h.Comment = t.Trim();
            Refill();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        void MoveToTop()
        {
            foreach (var h in Selected()) { Entries.Remove(h); Entries.Add(h); }
            Refill();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        void RemoveSelected()
        {
            foreach (var h in Selected()) Entries.Remove(h);
            Refill();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        public void ClearUnprotected()
        {
            Entries.RemoveAll(h => !h.Protected);
            Refill();
            Changed?.Invoke(this, EventArgs.Empty);
        }

        void SearchByDate()
        {
            using (var d = new DateDialog("Search by Date", DateTime.Today, false))
            {
                if (d.ShowDialog(FindForm()) != DialogResult.OK) return;
                dateFilter = d.Value.Date;
                Refill();
            }
        }

        public void Add(HistoryEntry h)
        {
            Entries.Add(h);
            while (Entries.Count > Math.Max(10, settings.MaxHistory))
            {
                var old = Entries.FirstOrDefault(x => !x.Protected);
                if (old == null) break;
                Entries.Remove(old);
            }
            Refill();
        }

        public void Refill()
        {
            var q = search.Text.Trim();
            shown = Entries.Where(h => (q.Length == 0 || h.Expression.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0 || h.Result.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0 || h.Comment.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0)
                && (dateFilter == null || h.Time.Date == dateFilter.Value)).ToList();
            list.BeginUpdate();
            list.Items.Clear();
            foreach (var h in shown) list.Items.Add(h);
            list.EndUpdate();
            if (list.Items.Count > 0) list.TopIndex = list.Items.Count - 1;
        }

        string ExprText(HistoryEntry h)
        {
            switch (settings.HistoryExpression)
            {
                case HistoryExprMode.Parsed: return string.IsNullOrEmpty(h.Parsed) ? h.Expression : h.Parsed;
                case HistoryExprMode.EnteredAndParsed: return string.IsNullOrEmpty(h.Parsed) || h.Parsed == h.Expression ? h.Expression : h.Expression + "  ⇒  " + h.Parsed;
                default: return h.Expression;
            }
        }

        string ResultLine(HistoryEntry h)
        {
            if (h.IsError || h.IsMessage) return h.Result;
            var prefix = h.Approx ? "≈ " : "= ";
            if (!string.IsNullOrEmpty(h.ExactText)) return "= " + h.ExactText + "  ≈ " + h.Result;
            return prefix + h.Result;
        }

        void MeasureEntry(object s, MeasureItemEventArgs e)
        {
            if (e.Index < 0 || e.Index >= list.Items.Count) return;
            var h = (HistoryEntry)list.Items[e.Index];
            int w = Math.Max(60, list.ClientSize.Width - 24);
            int hh = 10;
            hh += TextRenderer.MeasureText(ExprText(h), Theme.UiFont, new Size(w, 0), TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix).Height;
            hh += TextRenderer.MeasureText(ResultLine(h), EntryFont, new Size(w, 0), TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix).Height + 2;
            if (h.Comment.Length > 0) hh += TextRenderer.MeasureText(h.Comment, Theme.UiFont, new Size(w, 0), TextFormatFlags.WordBreak).Height;
            e.ItemHeight = Math.Min(255, hh + 6);
        }

        void DrawEntry(object s, DrawItemEventArgs e)
        {
            if (e.Index < 0 || e.Index >= list.Items.Count) return;
            var h = (HistoryEntry)list.Items[e.Index];
            bool sel = (e.State & DrawItemState.Selected) != 0;
            var g = e.Graphics;
            using (var b = new SolidBrush(sel ? Theme.Selection : Theme.Panel)) g.FillRectangle(b, e.Bounds);
            using (var p = new Pen(Theme.Line)) g.DrawLine(p, e.Bounds.Left + 10, e.Bounds.Bottom - 1, e.Bounds.Right - 10, e.Bounds.Bottom - 1);
            int x = e.Bounds.Left + 12, w = e.Bounds.Width - 24, y = e.Bounds.Top + 6;
            if (h.Protected) using (var b = new SolidBrush(Theme.Accent)) g.FillRectangle(b, e.Bounds.Left + 3, e.Bounds.Top + 8, 3, e.Bounds.Height - 16);
            var et = ExprText(h);
            var es = TextRenderer.MeasureText(et, Theme.UiFont, new Size(w, 0), TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
            TextRenderer.DrawText(g, et, Theme.UiFont, new Rectangle(x, y, w, es.Height), Theme.TextDim, TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
            y += es.Height + 2;
            var rl = ResultLine(h);
            var rs = TextRenderer.MeasureText(rl, EntryFont, new Size(w, 0), TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
            var rr = new Rectangle(x, y, w, rs.Height);
            if (h.IsError) TextRenderer.DrawText(g, rl, EntryFont, rr, Theme.Error, TextFormatFlags.Right | TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
            else if (h.IsMessage || !Colorize || h.Tokens.Count == 0 || rs.Height > EntryFont.Height + 4 || !string.IsNullOrEmpty(h.ExactText))
                TextRenderer.DrawText(g, rl, EntryFont, rr, Theme.Text, TextFormatFlags.Right | TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
            else
            {
                var toks = new List<OutTok> { new OutTok(h.Approx ? "≈ " : "= ", TokType.Plain) };
                toks.AddRange(h.Tokens);
                int total = toks.Sum(t => TextRenderer.MeasureText(g, t.Text, EntryFont, new Size(int.MaxValue, int.MaxValue), TextFormatFlags.NoPadding | TextFormatFlags.NoPrefix).Width);
                int cx = x + w - total;
                foreach (var t in toks)
                {
                    var c = t.Type == TokType.Plain && (t.Text == "= " || t.Text == "≈ ") ? Theme.TextFaint : Theme.TokenColor(t.Type);
                    TextRenderer.DrawText(g, t.Text, EntryFont, new Point(cx, y), c, TextFormatFlags.NoPadding | TextFormatFlags.NoPrefix);
                    cx += TextRenderer.MeasureText(g, t.Text, EntryFont, new Size(int.MaxValue, int.MaxValue), TextFormatFlags.NoPadding | TextFormatFlags.NoPrefix).Width;
                }
            }
            y += rs.Height;
            if (h.Comment.Length > 0)
                TextRenderer.DrawText(g, h.Comment, Theme.UiFont, new Rectangle(x, y, w, e.Bounds.Bottom - y), Theme.TokText, TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
        }
    }

    public sealed class RpnPanel : Panel
    {
        readonly List<Expr> stack;
        readonly ListBox list;
        readonly Func<Expr, string> format;
        public event Action<string> Operation;
        public event Action<int, string> EditRegister;

        public RpnPanel(List<Expr> s, Func<Expr, string> fmt)
        {
            stack = s; format = fmt;
            BackColor = Theme.Panel;
            list = new ListBox { Dock = DockStyle.Fill, DrawMode = DrawMode.OwnerDrawFixed, ItemHeight = 30, BorderStyle = BorderStyle.None, BackColor = Theme.Panel, ForeColor = Theme.Text, IntegralHeight = false };
            list.DrawItem += DrawRegister;
            list.DoubleClick += (o, e) =>
            {
                int idx = SelectedRegister;
                if (idx < 0) return;
                var t = InputDialog.Ask(FindForm(), "Edit Register", "Value of register " + (idx + 1) + ":", format(stack[stack.Count - 1 - idx]));
                if (t != null) EditRegister?.Invoke(idx, t);
            };
            Theme.DarkScrollbars(list);
            var bar = new TableLayoutPanel { Dock = DockStyle.Bottom, Height = 76, ColumnCount = 4, RowCount = 2, BackColor = Theme.Panel, Padding = new Padding(6) };
            for (int i = 0; i < 4; i++) bar.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 25));
            bar.RowStyles.Add(new RowStyle(SizeType.Percent, 50)); bar.RowStyles.Add(new RowStyle(SizeType.Percent, 50));
            var tip = new DarkToolTip();
            void B(string label, string op, string t, int c, int r)
            {
                var k = new FlatKey(label, () => Operation?.Invoke(op), t) { Dock = DockStyle.Fill, Font = Theme.UiFont, LabelFont = Theme.UiFont };
                tip.SetToolTip(k, t);
                bar.Controls.Add(k, c, r);
            }
            B("↑", "up", "Rotate the stack or move the selected register up", 0, 0);
            B("↓", "down", "Rotate the stack or move the selected register down", 1, 0);
            B("⇅", "swap", "Swap the top two values or move the selected value to the top of the stack", 2, 0);
            B("copy", "copy", "Copy the selected or top value to the top of the stack", 3, 0);
            B("lastx", "lastx", "Enter the top value from before the last numeric operation", 0, 1);
            B("drop", "delete", "Delete the top or selected value", 1, 1);
            B("±", "negate", "Negate the top value", 2, 1);
            B("clear", "clear", "Clear the RPN stack", 3, 1);
            Controls.Add(list);
            Controls.Add(bar);
            Refill();
        }

        public int SelectedRegister => list.SelectedIndex < 0 ? -1 : list.Items.Count - 1 - list.SelectedIndex;

        public void Refill()
        {
            list.BeginUpdate();
            list.Items.Clear();
            for (int i = 0; i < stack.Count; i++) list.Items.Add(i);
            list.EndUpdate();
            if (list.Items.Count > 0) list.TopIndex = list.Items.Count - 1;
        }

        void DrawRegister(object s, DrawItemEventArgs e)
        {
            if (e.Index < 0 || e.Index >= stack.Count) return;
            bool sel = (e.State & DrawItemState.Selected) != 0;
            using (var b = new SolidBrush(sel ? Theme.Selection : Theme.Panel)) e.Graphics.FillRectangle(b, e.Bounds);
            int reg = stack.Count - e.Index;
            TextRenderer.DrawText(e.Graphics, reg.ToString(), Theme.UiFont, new Rectangle(e.Bounds.X + 8, e.Bounds.Y, 30, e.Bounds.Height), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter);
            TextRenderer.DrawText(e.Graphics, format(stack[e.Index]), Theme.HistoryFont, new Rectangle(e.Bounds.X + 40, e.Bounds.Y, e.Bounds.Width - 50, e.Bounds.Height), Theme.Text, TextFormatFlags.VerticalCenter | TextFormatFlags.Right | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPrefix);
            using (var p = new Pen(Theme.Line)) e.Graphics.DrawLine(p, e.Bounds.Left + 8, e.Bounds.Bottom - 1, e.Bounds.Right - 8, e.Bounds.Bottom - 1);
        }
    }

    public sealed class BasesPanel : Panel
    {
        readonly Label bin, oct, dec, hex, note;
        readonly BitsGrid bits;
        readonly DarkCombo width;
        readonly CheckBox signed;
        BigInteger value;
        bool hasValue;
        public event Action<BigInteger> ValueEdited;

        public BasesPanel()
        {
            BackColor = Theme.Panel;
            AutoScroll = true;
            var flow = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 2, Padding = new Padding(12, 10, 12, 10), BackColor = Theme.Panel };
            flow.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            flow.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
            Label Row(string name, int r)
            {
                flow.Controls.Add(new Label { Text = name, AutoSize = true, ForeColor = Theme.TextDim, Font = Theme.UiFont, Margin = new Padding(0, 8, 10, 4) }, 0, r);
                var v = new Label { Text = "-", AutoSize = true, ForeColor = Theme.Text, Font = Theme.MonoFont, Margin = new Padding(0, 6, 0, 4), Cursor = Cursors.Hand, MaximumSize = new Size(260, 0) };
                v.Click += (o, e) => { HistoryPanel.SafeClipboard(v.Text.Replace(" ", "")); note.Text = "Copied " + name.TrimEnd(':').ToLowerInvariant() + " value"; };
                flow.Controls.Add(v, 1, r);
                return v;
            }
            bin = Row("Binary:", 0); oct = Row("Octal:", 1); dec = Row("Decimal:", 2); hex = Row("Hexadecimal:", 3);
            var opts = Ui.Row();
            width = Ui.Combo(new[] { "8 bits", "16 bits", "32 bits", "64 bits" }, 3, 90);
            signed = Ui.Check("Signed", true);
            width.SelectedIndexChanged += (o, e) => Render();
            signed.CheckedChanged += (o, e) => Render();
            opts.Controls.Add(width); opts.Controls.Add(signed);
            opts.Margin = new Padding(0, 8, 0, 4);
            flow.Controls.Add(opts, 0, 4);
            flow.SetColumnSpan(opts, 2);
            bits = new BitsGrid { Height = 150, Dock = DockStyle.Top };
            bits.BitToggled += i =>
            {
                if (!hasValue) value = 0;
                int w = BitWidth();
                var mask = (BigInteger.One << w) - 1;
                var u = value & mask;
                u ^= BigInteger.One << i;
                var nv = signed.Checked && ((u >> (w - 1)) & 1) == 1 ? u - (BigInteger.One << w) : u;
                ValueEdited?.Invoke(nv);
            };
            note = new Label { Dock = DockStyle.Top, Height = 40, ForeColor = Theme.TextFaint, Font = Theme.UiFont, Padding = new Padding(12, 6, 12, 0), Text = "Click a bit to toggle it. Click a value to copy it." };
            Controls.Add(note);
            Controls.Add(bits);
            Controls.Add(flow);
        }

        int BitWidth() => new[] { 8, 16, 32, 64 }[Math.Max(0, width.SelectedIndex)];

        public void SetValue(Expr e)
        {
            if (e is NumE n && n.V.IsReal && n.V.Re.IsInt && Real.BitLen(n.V.Re.N) <= 64)
            {
                value = n.V.Re.N; hasValue = true;
            }
            else hasValue = false;
            Render();
        }

        void Render()
        {
            if (!hasValue)
            {
                bin.Text = oct.Text = dec.Text = hex.Text = "-";
                bits.Value = 0; bits.Bits = BitWidth(); bits.Invalidate();
                return;
            }
            int w = BitWidth();
            while (w < 64 && (value >= (BigInteger.One << (signed.Checked ? w - 1 : w)) || value < -(BigInteger.One << (w - 1)))) w *= 2;
            if (w != BitWidth()) width.SelectedIndex = Array.IndexOf(new[] { 8, 16, 32, 64 }, w);
            var mask = (BigInteger.One << w) - 1;
            var u = value & mask;
            string b = ToBase(u, 2).PadLeft(w, '0');
            var grouped = string.Join(" ", Enumerable.Range(0, w / 4).Select(i => b.Substring(i * 4, 4)));
            bin.Text = grouped;
            oct.Text = (value.Sign < 0 ? "−" : "") + ToBase(BigInteger.Abs(value), 8);
            dec.Text = value.ToString(CultureInfo.InvariantCulture);
            hex.Text = (value.Sign < 0 ? "−" + ToBase(BigInteger.Abs(value), 16) + "  (" + ToBase(u, 16) + ")" : ToBase(value, 16));
            bits.Bits = w; bits.Value = u; bits.Invalidate();
        }

        static string ToBase(BigInteger v, int b)
        {
            if (v.IsZero) return "0";
            const string d = "0123456789ABCDEF";
            var s = "";
            while (v > 0) { s = d[(int)(v % b)] + s; v /= b; }
            return s;
        }
    }

    public sealed class BitsGrid : Control
    {
        public int Bits = 64;
        public BigInteger Value;
        public event Action<int> BitToggled;

        public BitsGrid()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            Cursor = Cursors.Hand;
        }

        Rectangle Cell(int bit, out int row)
        {
            int perRow = 16;
            int rows = (Bits + perRow - 1) / perRow;
            int index = Bits - 1 - bit;
            row = index / perRow;
            int col = index % perRow;
            int gap = 6;
            float cw = (Width - 24 - gap * 3) / 16f;
            int x = 12 + (int)(col * cw) + (col / 4) * gap;
            int y = 6 + row * 34;
            return new Rectangle(x, y, (int)cw - 2, 20);
        }

        protected override void OnMouseDown(MouseEventArgs e)
        {
            for (int b = 0; b < Bits; b++)
                if (Cell(b, out _).Contains(e.Location)) { BitToggled?.Invoke(b); return; }
            base.OnMouseDown(e);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(Theme.Panel);
            for (int b = 0; b < Bits; b++)
            {
                var r = Cell(b, out int row);
                bool on = ((Value >> b) & 1) == 1;
                using (var br = new SolidBrush(on ? Theme.AccentSoft : Theme.Raised)) g.FillRectangle(br, r);
                using (var p = new Pen(on ? Theme.Accent : Theme.Line)) g.DrawRectangle(p, r);
                TextRenderer.DrawText(g, on ? "1" : "0", Theme.MonoFont, r, on ? Theme.Text : Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter);
                if (b % 4 == 0) TextRenderer.DrawText(g, b.ToString(), Theme.SmallFont, new Rectangle(r.X - 8, r.Bottom, r.Width + 16, 12), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter);
            }
        }
    }
}
