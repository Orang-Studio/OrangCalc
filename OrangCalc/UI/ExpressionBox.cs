using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class CompletionItem
    {
        public string Name, Title, Kind;
        public object Item;
        public int Score;
    }

    // popup list that never takes focus from the editor
    public sealed class CompletionPopup : Form
    {
        public readonly ListBox List;

        public CompletionPopup()
        {
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            StartPosition = FormStartPosition.Manual;
            BackColor = Theme.LineStrong;
            Padding = new Padding(1);
            List = new ListBox
            {
                Dock = DockStyle.Fill, BorderStyle = BorderStyle.None, BackColor = Theme.Raised, ForeColor = Theme.Text,
                DrawMode = DrawMode.OwnerDrawFixed, ItemHeight = 24, IntegralHeight = false, Font = Theme.UiFont
            };
            List.DrawItem += DrawEntry;
            Controls.Add(List);
            Theme.DarkScrollbars(List);
        }

        protected override bool ShowWithoutActivation => true;

        protected override CreateParams CreateParams
        {
            get
            {
                var cp = base.CreateParams;
                cp.ExStyle |= 0x08000000 | 0x00000080 | 0x00000008;
                return cp;
            }
        }

        void DrawEntry(object s, DrawItemEventArgs e)
        {
            if (e.Index < 0) return;
            var it = (CompletionItem)List.Items[e.Index];
            bool sel = (e.State & DrawItemState.Selected) != 0;
            using (var b = new SolidBrush(sel ? Theme.Selection : Theme.Raised)) e.Graphics.FillRectangle(b, e.Bounds);
            var r = e.Bounds;
            int kindW = 70;
            TextRenderer.DrawText(e.Graphics, it.Name, Theme.UiBold, new Rectangle(r.X + 8, r.Y, r.Width / 2, r.Height), Theme.Text, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
            var nw = TextRenderer.MeasureText(it.Name, Theme.UiBold).Width;
            TextRenderer.DrawText(e.Graphics, it.Title, Theme.UiFont, new Rectangle(r.X + 14 + nw, r.Y, r.Width - nw - kindW - 22, r.Height), Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
            TextRenderer.DrawText(e.Graphics, it.Kind, Theme.SmallFont, new Rectangle(r.Right - kindW - 6, r.Y, kindW, r.Height), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.Right | TextFormatFlags.SingleLine);
        }
    }

    public sealed class ExpressionBox : Card
    {
        public readonly TextBox Box;
        readonly Calculator calc;
        readonly AppSettings settings;
        readonly CompletionPopup popup = new CompletionPopup();
        readonly List<string> undo = new List<string>();
        readonly List<int> undoCaret = new List<int>();
        int undoPos = -1;
        bool suppressUndo;
        readonly Timer undoTimer = new Timer { Interval = 400 };
        int completionStart = -1;

        public Func<List<string>> HistoryProvider;
        int historyIndex = -1;
        string historyDraft = "";

        public event EventHandler CalculateRequested;
        public event Action<char> RpnOperator;
        public event EventHandler InsertDateRequested, InsertMatrixRequested, CalculateSelectionRequested;
        public Func<bool> RpnActive;

        public ExpressionBox(Calculator c, AppSettings s)
        {
            calc = c; settings = s;
            BackColor = Theme.Raised;
            Padding = new Padding(10, 8, 10, 8);
            Box = new TextBox
            {
                Multiline = true, WordWrap = true, AcceptsReturn = false, AcceptsTab = false, BorderStyle = BorderStyle.None,
                BackColor = Theme.Raised, ForeColor = Theme.Text, Font = Theme.ExpressionFont, Dock = DockStyle.Fill, ScrollBars = ScrollBars.None
            };
            Controls.Add(Box);
            Box.KeyDown += OnKeyDown;
            Box.KeyPress += OnKeyPress;
            Box.TextChanged += OnTextChanged;
            Box.LostFocus += (o, e) => { if (!popup.ContainsFocus) HidePopup(); };
            Box.MouseUp += (o, e) => { if (settings.AutoCalculateSelection && Box.SelectionLength > 0) CalculateSelectionRequested?.Invoke(this, EventArgs.Empty); };
            popup.List.MouseClick += (o, e) => { AcceptCompletion(); Box.Focus(); };
            undoTimer.Tick += (o, e) => { undoTimer.Stop(); PushUndo(); };
            Box.ContextMenuStrip = BuildMenu();
            PushUndo();
            Theme.DarkScrollbars(Box);
            Box.Resize += (o, e) => UpdateScrollbars();
        }

        public override string Text { get => Box.Text; set => SetText(value); }

        public void SetText(string t, bool selectAll = false)
        {
            Box.Text = t ?? "";
            Box.SelectionStart = Box.TextLength;
            if (selectAll) Box.SelectAll();
            PushUndo();
        }

        void UpdateScrollbars()
        {
            var lines = Box.GetLineFromCharIndex(Box.TextLength) + 1;
            int lineH = Box.Font.Height;
            bool need = lines * lineH > Box.ClientSize.Height;
            var want = need ? ScrollBars.Vertical : ScrollBars.None;
            if (Box.ScrollBars != want) Box.ScrollBars = want;
        }

        ContextMenuStrip BuildMenu()
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            ((ToolStripMenuItem)m.Items.Add("Undo", null, (s, e) => Undo())).ShortcutKeyDisplayString = "Ctrl+Z";
            ((ToolStripMenuItem)m.Items.Add("Redo", null, (s, e) => Redo())).ShortcutKeyDisplayString = "Ctrl+Y";
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Cut", null, (s, e) => Box.Cut());
            m.Items.Add("Copy", null, (s, e) => { if (Box.SelectionLength > 0) Box.Copy(); });
            m.Items.Add("Paste", null, (s, e) => Box.Paste());
            m.Items.Add("Delete", null, (s, e) => Box.SelectedText = "");
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Select All", null, (s, e) => Box.SelectAll());
            m.Items.Add("Clear", null, (s, e) => SetText(""));
            m.Items.Add("Clear History Navigation", null, (s, e) => historyIndex = -1);
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Insert Date…", null, (s, e) => InsertDateRequested?.Invoke(this, EventArgs.Empty));
            m.Items.Add("Insert Matrix…", null, (s, e) => InsertMatrixRequested?.Invoke(this, EventArgs.Empty));
            m.Items.Add("Calculate Selection", null, (s, e) => CalculateSelectionRequested?.Invoke(this, EventArgs.Empty));
            var comp = new ToolStripMenuItem("Completion");
            foreach (CompletionMode cm in Enum.GetValues(typeof(CompletionMode)))
            {
                var mi = new ToolStripMenuItem(Words(cm.ToString())) { Tag = cm };
                mi.Click += (s, e) => settings.Completion = (CompletionMode)((ToolStripMenuItem)s).Tag;
                comp.DropDownItems.Add(mi);
            }
            comp.DropDownOpening += (s, e) => { foreach (ToolStripMenuItem mi in comp.DropDownItems) mi.Checked = (CompletionMode)mi.Tag == settings.Completion; };
            m.Items.Add(comp);
            var auto = new ToolStripMenuItem("Automatically Calculate Selection");
            auto.Click += (s, e) => settings.AutoCalculateSelection = !settings.AutoCalculateSelection;
            m.Items.Add(auto);
            m.Opening += (s, e) => { auto.Checked = settings.AutoCalculateSelection; m.Items[0].Enabled = undoPos > 0; m.Items[1].Enabled = undoPos < undo.Count - 1; };
            return m;
        }

        public static string Words(string s) => System.Text.RegularExpressions.Regex.Replace(s, "(?<=[a-z])([A-Z])", " $1");

        void OnTextChanged(object s, EventArgs e)
        {
            if (!suppressUndo) { undoTimer.Stop(); undoTimer.Start(); }
            UpdateScrollbars();
            UpdateCompletion(false);
        }

        void PushUndo()
        {
            if (undoPos >= 0 && undoPos < undo.Count && undo[undoPos] == Box.Text) return;
            if (undoPos < undo.Count - 1) { undo.RemoveRange(undoPos + 1, undo.Count - undoPos - 1); undoCaret.RemoveRange(undoPos + 1, undoCaret.Count - undoPos - 1); }
            undo.Add(Box.Text); undoCaret.Add(Box.SelectionStart);
            if (undo.Count > 200) { undo.RemoveAt(0); undoCaret.RemoveAt(0); }
            undoPos = undo.Count - 1;
        }

        public void Undo()
        {
            undoTimer.Stop();
            PushUndo();
            if (undoPos <= 0) return;
            undoPos--;
            Restore();
        }

        public void Redo()
        {
            if (undoPos >= undo.Count - 1) return;
            undoPos++;
            Restore();
        }

        void Restore()
        {
            suppressUndo = true;
            Box.Text = undo[undoPos];
            Box.SelectionStart = Math.Min(undoCaret[undoPos], Box.TextLength);
            suppressUndo = false;
        }

        public void Insert(string text, int caretBack = 0)
        {
            int start = Box.SelectionStart;
            Box.SelectedText = text;
            Box.SelectionStart = Math.Max(0, start + text.Length - caretBack);
            Box.SelectionLength = 0;
            PushUndo();
            Box.Focus();
        }

        // inserts name( ) around the selection, or around everything if wrapAll
        public void InsertFunction(string name, bool wrapAll, string extraArgs = "")
        {
            if (Box.SelectionLength > 0)
            {
                var sel = Box.SelectedText;
                Insert(name + "(" + sel + extraArgs + ")");
                return;
            }
            if (wrapAll && Box.TextLength > 0)
            {
                SetText(name + "(" + Box.Text + extraArgs + ")");
                Box.Focus();
                return;
            }
            Insert(name + "(" + extraArgs + ")", 1 + extraArgs.Length);
        }

        public void Backspace()
        {
            if (Box.SelectionLength > 0) { Box.SelectedText = ""; return; }
            int p = Box.SelectionStart;
            if (p == 0) return;
            int n = 1;
            if (p >= 2 && char.IsSurrogatePair(Box.Text[p - 2], Box.Text[p - 1])) n = 2;
            Box.Text = Box.Text.Remove(p - n, n);
            Box.SelectionStart = p - n;
            Box.Focus();
        }

        public void DeleteForward()
        {
            if (Box.SelectionLength > 0) { Box.SelectedText = ""; return; }
            int p = Box.SelectionStart;
            if (p >= Box.TextLength) return;
            Box.Text = Box.Text.Remove(p, 1);
            Box.SelectionStart = p;
            Box.Focus();
        }

        public void MoveCaret(int delta, bool absolute = false)
        {
            int p = absolute ? (delta < 0 ? 0 : Box.TextLength) : Math.Max(0, Math.Min(Box.TextLength, Box.SelectionStart + delta));
            Box.SelectionStart = p; Box.SelectionLength = 0;
            Box.Focus();
        }

        void OnKeyDown(object s, KeyEventArgs e)
        {
            if (popup.Visible)
            {
                switch (e.KeyCode)
                {
                    case Keys.Down: popup.List.SelectedIndex = Math.Min(popup.List.Items.Count - 1, popup.List.SelectedIndex + 1); e.Handled = e.SuppressKeyPress = true; return;
                    case Keys.Up: popup.List.SelectedIndex = Math.Max(0, popup.List.SelectedIndex - 1); e.Handled = e.SuppressKeyPress = true; return;
                    case Keys.PageDown: popup.List.SelectedIndex = Math.Min(popup.List.Items.Count - 1, popup.List.SelectedIndex + 8); e.Handled = e.SuppressKeyPress = true; return;
                    case Keys.PageUp: popup.List.SelectedIndex = Math.Max(0, popup.List.SelectedIndex - 8); e.Handled = e.SuppressKeyPress = true; return;
                    case Keys.Tab:
                    case Keys.Enter when popup.List.SelectedIndex >= 0 && !e.Shift:
                        AcceptCompletion(); e.Handled = e.SuppressKeyPress = true; return;
                    case Keys.Escape: HidePopup(); e.Handled = e.SuppressKeyPress = true; return;
                }
            }
            if (e.KeyCode == Keys.Enter && !e.Shift)
            {
                e.Handled = e.SuppressKeyPress = true;
                HidePopup();
                historyIndex = -1;
                CalculateRequested?.Invoke(this, EventArgs.Empty);
                return;
            }
            if (e.KeyCode == Keys.Enter && e.Shift)
            {
                e.Handled = e.SuppressKeyPress = true;
                Insert("\r\n");
                return;
            }
            if (e.Control && e.KeyCode == Keys.Space) { UpdateCompletion(true); e.Handled = e.SuppressKeyPress = true; return; }
            if (e.Control && e.KeyCode == Keys.Z && !e.Shift) { Undo(); e.Handled = e.SuppressKeyPress = true; return; }
            if (e.Control && (e.KeyCode == Keys.Y || e.KeyCode == Keys.Z && e.Shift)) { Redo(); e.Handled = e.SuppressKeyPress = true; return; }
            if (e.Control && e.KeyCode == Keys.A) { Box.SelectAll(); e.Handled = e.SuppressKeyPress = true; return; }
            if ((e.KeyCode == Keys.Up || e.KeyCode == Keys.Down) && !e.Control && !e.Alt && HistoryProvider != null)
            {
                int line = Box.GetLineFromCharIndex(Box.SelectionStart);
                int last = Box.GetLineFromCharIndex(Box.TextLength);
                if ((e.KeyCode == Keys.Up && line == 0) || (e.KeyCode == Keys.Down && line == last))
                {
                    var hist = HistoryProvider();
                    if (hist.Count == 0) return;
                    if (historyIndex == -1) historyDraft = Box.Text;
                    if (e.KeyCode == Keys.Up) historyIndex = Math.Min(hist.Count - 1, historyIndex + 1);
                    else historyIndex = Math.Max(-1, historyIndex - 1);
                    SetText(historyIndex < 0 ? historyDraft : hist[historyIndex]);
                    e.Handled = e.SuppressKeyPress = true;
                }
            }
        }

        void OnKeyPress(object s, KeyPressEventArgs e)
        {
            if (RpnActive != null && RpnActive() && settings.RpnKeys && Box.SelectionStart == Box.TextLength)
            {
                char ch = e.KeyChar;
                if ("+-*/^".IndexOf(ch) < 0) return;
                var t = Box.Text.TrimEnd();
                if (t.EndsWith("(") || t.EndsWith(",")) return;
                if (ch == '-' && (t.EndsWith("e") || t.EndsWith("E"))) return;
                if (t.Count(x => x == '(') > t.Count(x => x == ')')) return;
                e.Handled = true;
                RpnOperator?.Invoke(ch);
            }
        }

        public void HidePopup()
        {
            if (popup.Visible) popup.Hide();
            completionStart = -1;
        }

        string CurrentWord(out int start)
        {
            int p = Box.SelectionStart;
            var t = Box.Text;
            start = p;
            while (start > 0 && (Lexer.IsIdentPart(t[start - 1]))) start--;
            if (start < t.Length && start < p && char.IsDigit(t[start])) { while (start < p && char.IsDigit(t[start])) start++; }
            return t.Substring(start, p - start);
        }

        void UpdateCompletion(bool force)
        {
            if (settings.Completion == CompletionMode.Off && !force) { HidePopup(); return; }
            if (!Box.Focused && !force) { HidePopup(); return; }
            var word = CurrentWord(out int start);
            int min = settings.Completion == CompletionMode.LimitedFull || settings.Completion == CompletionMode.LimitedStrict ? Math.Max(3, settings.CompletionMinChars) : settings.CompletionMinChars;
            if (!force && word.Length < min) { HidePopup(); return; }
            if (word.Length == 0 && !force) { HidePopup(); return; }
            bool strict = settings.Completion == CompletionMode.Strict || settings.Completion == CompletionMode.LimitedStrict;
            var items = Matches(word, strict);
            if (items.Count == 0 || (items.Count == 1 && items[0].Name == word && !force)) { HidePopup(); return; }
            completionStart = start;
            popup.List.BeginUpdate();
            popup.List.Items.Clear();
            foreach (var it in items.Take(60)) popup.List.Items.Add(it);
            popup.List.EndUpdate();
            popup.List.SelectedIndex = 0;
            var caret = Box.GetPositionFromCharIndex(Math.Max(0, start));
            var pt = Box.PointToScreen(new Point(caret.X, caret.Y + Box.Font.Height + 4));
            int h = Math.Min(10, popup.List.Items.Count) * popup.List.ItemHeight + 2;
            var screen = Screen.FromControl(this).WorkingArea;
            if (pt.Y + h > screen.Bottom) pt.Y = Box.PointToScreen(caret).Y - h - 4;
            popup.Bounds = new Rectangle(Math.Min(pt.X, screen.Right - 460), pt.Y, 460, h);
            if (!popup.Visible) popup.Show(FindForm());
        }

        List<CompletionItem> Matches(string word, bool strict)
        {
            var res = new List<CompletionItem>();
            string lw = word.ToLowerInvariant();
            int Score(string name, string title)
            {
                var ln = name.ToLowerInvariant();
                if (ln == lw) return 100;
                if (name.StartsWith(word, StringComparison.Ordinal)) return 90 - name.Length;
                if (ln.StartsWith(lw)) return 80 - name.Length;
                if (strict) return -1;
                if (ln.Contains(lw)) return 50 - name.Length;
                if (title != null && lw.Length >= 3 && title.ToLowerInvariant().Contains(lw)) return 30;
                return -1;
            }
            foreach (var f in calc.Functions.Where(f => f.Active && !f.Hidden && !f.Name.StartsWith("_")))
            {
                int best = -1; string bn = f.Names[0];
                foreach (var n in f.Names) { int sc = Score(n, f.Title); if (sc > best) { best = sc; bn = n; } }
                if (best >= 0) res.Add(new CompletionItem { Name = bn, Title = f.DisplayTitle, Kind = "function", Item = f, Score = best });
            }
            foreach (var v in calc.Variables.Where(v => v.Active && !v.Hidden))
            {
                int best = -1; string bn = v.Names[0];
                foreach (var n in v.Names) { int sc = Score(n, v.Title); if (sc > best) { best = sc; bn = n; } }
                if (best >= 0) res.Add(new CompletionItem { Name = bn, Title = v.DisplayTitle, Kind = v.IsUnknown ? "unknown" : "variable", Item = v, Score = best });
            }
            foreach (var u in calc.Units.Where(u => u.Active && !u.Hidden))
            {
                int best = -1; string bn = u.Abbrev;
                foreach (var n in u.Names.Concat(new[] { u.Plural })) { int sc = Score(n, u.Title); if (sc > best) { best = sc; bn = n; } }
                if (best >= 0) res.Add(new CompletionItem { Name = bn, Title = u.DisplayTitle, Kind = u.IsCurrency ? "currency" : "unit", Item = u, Score = best - 2 });
            }
            foreach (var p in Units.Prefixes.Where(p => p.Name.StartsWith(lw)))
                res.Add(new CompletionItem { Name = p.Name, Title = "Prefix " + p.Symbol, Kind = "prefix", Item = p, Score = 20 });
            foreach (var kw in new[] { "to", "where", "mod", "rem", "and", "or", "xor", "not", "per" })
                if (kw.StartsWith(lw) && kw != lw) res.Add(new CompletionItem { Name = kw, Title = "Operator", Kind = "keyword", Score = 25 });
            return res.OrderByDescending(r => r.Score).ThenBy(r => r.Name.Length).ThenBy(r => r.Name, StringComparer.Ordinal).ToList();
        }

        void AcceptCompletion()
        {
            if (popup.List.SelectedItem is CompletionItem it && completionStart >= 0)
            {
                int p = Box.SelectionStart;
                int end = p;
                while (end < Box.TextLength && Lexer.IsIdentPart(Box.Text[end])) end++;
                Box.SelectionStart = completionStart;
                Box.SelectionLength = end - completionStart;
                bool fn = it.Item is MathFunction mf;
                bool hasParen = end < Box.TextLength && Box.Text[end] == '(';
                if (fn && !hasParen)
                {
                    var f = (MathFunction)it.Item;
                    Box.SelectedText = it.Name + "()";
                    if (f.MaxArgs != 0) Box.SelectionStart -= 1;
                }
                else Box.SelectedText = it.Name;
                PushUndo();
            }
            HidePopup();
        }
    }
}
