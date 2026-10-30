using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Windows.Forms;

namespace OrangCalc.UI
{
    public enum KeyStyle { Normal, Number, Operator, Accent, Muted }

    // calculator key with up to three actions: left click, right click, middle click
    public class FlatKey : Control
    {
        public string Label = "";
        public string Hint = "";
        public Action Primary, Secondary, Tertiary;
        public string PrimaryTip = "", SecondaryTip = "", TertiaryTip = "";
        public KeyStyle KeyStyle = KeyStyle.Normal;
        public bool Toggle;
        public bool Checked;
        bool hover, down;
        public Font LabelFont;

        public FlatKey()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            Cursor = Cursors.Hand;
            TabStop = false;
            Margin = new Padding(2);
        }

        public FlatKey(string label, Action primary, string tip = "") : this()
        {
            Label = label; Primary = primary; PrimaryTip = tip;
        }

        public string TooltipText
        {
            get
            {
                var parts = new List<string>();
                if (!string.IsNullOrEmpty(PrimaryTip)) parts.Add(PrimaryTip);
                if (Secondary != null && !string.IsNullOrEmpty(SecondaryTip)) parts.Add("Right-click: " + SecondaryTip);
                if (Tertiary != null && !string.IsNullOrEmpty(TertiaryTip)) parts.Add("Middle-click: " + TertiaryTip);
                return string.Join("\n", parts);
            }
        }

        protected override void OnMouseEnter(EventArgs e) { hover = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hover = false; down = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { down = true; Invalidate(); base.OnMouseDown(e); }

        protected override void OnMouseUp(MouseEventArgs e)
        {
            bool wasDown = down;
            down = false; Invalidate();
            base.OnMouseUp(e);
            if (!wasDown || !ClientRectangle.Contains(e.Location)) return;
            if (e.Button == MouseButtons.Left) Primary?.Invoke();
            else if (e.Button == MouseButtons.Right) (Secondary ?? Primary)?.Invoke();
            else if (e.Button == MouseButtons.Middle) (Tertiary ?? Secondary ?? Primary)?.Invoke();
        }

        Color BackFor()
        {
            Color b;
            switch (KeyStyle)
            {
                case KeyStyle.Number: b = Theme.Raised; break;
                case KeyStyle.Operator: b = Color.FromArgb(34, 38, 44); break;
                case KeyStyle.Accent: b = Theme.Accent; break;
                case KeyStyle.Muted: b = Theme.Panel; break;
                default: b = Color.FromArgb(32, 35, 41); break;
            }
            if (Toggle && Checked) b = Theme.AccentSoft;
            if (KeyStyle == KeyStyle.Accent)
            {
                if (down) return ControlPaint.Dark(b, 0.1f);
                if (hover) return ControlPaint.Light(b, 0.15f);
                return b;
            }
            if (down) return Theme.Pressed;
            if (hover) return Theme.Hover;
            return b;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(Parent?.BackColor ?? Theme.Window);
            var r = new Rectangle(0, 0, Width - 1, Height - 1);
            using (var b = new SolidBrush(BackFor())) g.FillRectangle(b, r);
            if (KeyStyle != KeyStyle.Accent)
                using (var p = new Pen(Toggle && Checked ? Theme.Accent : Theme.Line)) g.DrawRectangle(p, r);
            var fg = KeyStyle == KeyStyle.Accent ? Theme.AccentText : Enabled ? Theme.Text : Theme.TextFaint;
            var font = LabelFont ?? Font;
            g.TextRenderingHint = System.Drawing.Text.TextRenderingHint.ClearTypeGridFit;
            var flags = TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.NoPrefix;
            var tr = new Rectangle(2, string.IsNullOrEmpty(Hint) ? 0 : 6, Width - 4, Height - (string.IsNullOrEmpty(Hint) ? 0 : 6));
            TextRenderer.DrawText(g, Label, font, tr, fg, flags);
            if (!string.IsNullOrEmpty(Hint))
            {
                var hc = KeyStyle == KeyStyle.Accent ? Theme.Mix(Theme.Accent, Theme.AccentText, 0.6) : Theme.TextFaint;
                TextRenderer.DrawText(g, Hint, Theme.SmallFont, new Rectangle(3, 2, Width - 6, 13), hc, TextFormatFlags.Right | TextFormatFlags.Top | TextFormatFlags.SingleLine | TextFormatFlags.NoPrefix | TextFormatFlags.EndEllipsis);
            }
            base.OnPaint(e);
        }
    }

    // plain flat button for dialogs and toolbars
    public class FlatButton : Control
    {
        public bool IsPrimary;
        bool hover, down;
        public event EventHandler Pressed;

        public FlatButton()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw | ControlStyles.Selectable, true);
            Height = 28; Width = 96;
            Cursor = Cursors.Hand;
            Font = Theme.UiFont;
        }

        public FlatButton(string text, EventHandler click, bool primary = false) : this()
        {
            Text = text; IsPrimary = primary;
            if (click != null) Pressed += click;
            Width = Math.Max(80, TextRenderer.MeasureText(text, Theme.UiFont).Width + 28);
        }

        public void PerformClick() => Pressed?.Invoke(this, EventArgs.Empty);

        protected override void OnMouseEnter(EventArgs e) { hover = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hover = false; down = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { if (e.Button == MouseButtons.Left) { down = true; Focus(); Invalidate(); } base.OnMouseDown(e); }
        protected override void OnMouseUp(MouseEventArgs e)
        {
            bool w = down; down = false; Invalidate(); base.OnMouseUp(e);
            if (w && e.Button == MouseButtons.Left && ClientRectangle.Contains(e.Location) && Enabled) Pressed?.Invoke(this, EventArgs.Empty);
        }
        protected override void OnKeyDown(KeyEventArgs e)
        {
            if (e.KeyCode == Keys.Space || e.KeyCode == Keys.Enter) { Pressed?.Invoke(this, EventArgs.Empty); e.Handled = true; }
            base.OnKeyDown(e);
        }
        protected override void OnGotFocus(EventArgs e) { Invalidate(); base.OnGotFocus(e); }
        protected override void OnLostFocus(EventArgs e) { Invalidate(); base.OnLostFocus(e); }
        protected override void OnEnabledChanged(EventArgs e) { Invalidate(); base.OnEnabledChanged(e); }
        protected override void OnTextChanged(EventArgs e) { Invalidate(); base.OnTextChanged(e); }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(Parent?.BackColor ?? Theme.Window);
            var r = new Rectangle(0, 0, Width - 1, Height - 1);
            Color bg = IsPrimary ? Theme.Accent : Theme.Raised;
            if (!Enabled) bg = Theme.Panel;
            else if (down) bg = IsPrimary ? ControlPaint.Dark(Theme.Accent, 0.1f) : Theme.Pressed;
            else if (hover) bg = IsPrimary ? ControlPaint.Light(Theme.Accent, 0.15f) : Theme.Hover;
            using (var b = new SolidBrush(bg)) g.FillRectangle(b, r);
            using (var pen = new Pen(Focused ? Theme.Accent : IsPrimary ? Theme.Accent : Theme.Line)) g.DrawRectangle(pen, r);
            var fg = !Enabled ? Theme.TextFaint : IsPrimary ? Theme.AccentText : Theme.Text;
            TextRenderer.DrawText(g, Text, Font, r, fg, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
        }
    }

    // horizontal segmented tabs with an accent underline
    public class SegmentBar : Control
    {
        public readonly List<string> Items = new List<string>();
        int selected;
        int hoverIndex = -1;
        public event EventHandler SelectedChanged;
        public bool Compact;

        public SegmentBar()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            Height = 32;
            Font = Theme.UiFont;
        }

        public int SelectedIndex
        {
            get => selected;
            set { if (value == selected) return; selected = value; Invalidate(); SelectedChanged?.Invoke(this, EventArgs.Empty); }
        }

        Rectangle ItemRect(int i)
        {
            if (Items.Count == 0) return Rectangle.Empty;
            if (Compact)
            {
                int x = 0;
                for (int k = 0; k < i; k++) x += TextRenderer.MeasureText(Items[k], Font).Width + 20;
                return new Rectangle(x, 0, TextRenderer.MeasureText(Items[i], Font).Width + 20, Height);
            }
            int w = Width / Items.Count;
            return new Rectangle(i * w, 0, i == Items.Count - 1 ? Width - i * w : w, Height);
        }

        int HitTest(Point p)
        {
            for (int i = 0; i < Items.Count; i++) if (ItemRect(i).Contains(p)) return i;
            return -1;
        }

        protected override void OnMouseMove(MouseEventArgs e) { int h = HitTest(e.Location); if (h != hoverIndex) { hoverIndex = h; Invalidate(); } base.OnMouseMove(e); }
        protected override void OnMouseLeave(EventArgs e) { hoverIndex = -1; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { int h = HitTest(e.Location); if (h >= 0) SelectedIndex = h; base.OnMouseDown(e); }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(BackColor == Color.Empty ? Theme.Window : BackColor);
            using (var p = new Pen(Theme.Line)) g.DrawLine(p, 0, Height - 1, Width, Height - 1);
            for (int i = 0; i < Items.Count; i++)
            {
                var r = ItemRect(i);
                if (i == hoverIndex && i != selected) using (var b = new SolidBrush(Theme.Panel)) g.FillRectangle(b, r);
                var c = i == selected ? Theme.Text : Theme.TextDim;
                TextRenderer.DrawText(g, Items[i], i == selected ? Theme.UiBold : Font, r, c, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
                if (i == selected) using (var b = new SolidBrush(Theme.Accent)) g.FillRectangle(b, r.Left + 6, Height - 3, r.Width - 12, 3);
            }
        }
    }

    // vertical navigation list
    public class NavRail : Control
    {
        public readonly List<string> Items = new List<string>();
        public readonly HashSet<int> GapBefore = new HashSet<int>();
        int selected, hoverIndex = -1;
        public event EventHandler SelectedChanged;
        const int ItemH = 38;

        public NavRail()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            Font = Theme.UiFont;
            Width = 128;
        }

        public int SelectedIndex
        {
            get => selected;
            set { if (value == selected) return; selected = value; Invalidate(); SelectedChanged?.Invoke(this, EventArgs.Empty); }
        }

        Rectangle ItemRect(int i)
        {
            int y = 8;
            for (int k = 0; k <= i; k++)
            {
                if (GapBefore.Contains(k)) y += 16;
                if (k < i) y += ItemH;
            }
            return new Rectangle(0, y, Width, ItemH);
        }

        int HitTest(Point p)
        {
            for (int i = 0; i < Items.Count; i++) if (ItemRect(i).Contains(p)) return i;
            return -1;
        }

        protected override void OnMouseMove(MouseEventArgs e) { int h = HitTest(e.Location); if (h != hoverIndex) { hoverIndex = h; Invalidate(); Cursor = h >= 0 ? Cursors.Hand : Cursors.Default; } base.OnMouseMove(e); }
        protected override void OnMouseLeave(EventArgs e) { hoverIndex = -1; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { int h = HitTest(e.Location); if (h >= 0) SelectedIndex = h; base.OnMouseDown(e); }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(Theme.Window);
            using (var p = new Pen(Theme.Line)) g.DrawLine(p, Width - 1, 0, Width - 1, Height);
            for (int i = 0; i < Items.Count; i++)
            {
                var r = ItemRect(i);
                if (GapBefore.Contains(i)) using (var p = new Pen(Theme.Line)) g.DrawLine(p, 12, r.Top - 8, Width - 13, r.Top - 8);
                if (i == selected) using (var b = new SolidBrush(Theme.Raised)) g.FillRectangle(b, 0, r.Top, Width - 1, r.Height);
                else if (i == hoverIndex) using (var b = new SolidBrush(Theme.Panel)) g.FillRectangle(b, 0, r.Top, Width - 1, r.Height);
                if (i == selected) using (var b = new SolidBrush(Theme.Accent)) g.FillRectangle(b, 0, r.Top + 8, 3, r.Height - 16);
                TextRenderer.DrawText(g, Items[i], i == selected ? Theme.UiBold : Font, new Rectangle(16, r.Top, Width - 20, r.Height), i == selected ? Theme.Text : Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
            }
        }
    }

    // combo box that draws its own dark list and edit area
    public class DarkCombo : ComboBox
    {
        public DarkCombo()
        {
            DrawMode = DrawMode.OwnerDrawFixed;
            DropDownStyle = ComboBoxStyle.DropDownList;
            FlatStyle = FlatStyle.Flat;
            BackColor = Theme.Raised;
            ForeColor = Theme.Text;
            ItemHeight = 20;
            IntegralHeight = false;
            MaxDropDownItems = 16;
        }

        protected override void OnDrawItem(DrawItemEventArgs e)
        {
            if (e.Index < 0) return;
            bool sel = (e.State & DrawItemState.Selected) != 0;
            bool edit = (e.State & DrawItemState.ComboBoxEdit) != 0;
            using (var b = new SolidBrush(sel && !edit ? Theme.Selection : Theme.Raised)) e.Graphics.FillRectangle(b, e.Bounds);
            TextRenderer.DrawText(e.Graphics, GetItemText(Items[e.Index]), Font, new Rectangle(e.Bounds.X + 4, e.Bounds.Y, e.Bounds.Width - 4, e.Bounds.Height), Enabled ? Theme.Text : Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis);
        }

        protected override void WndProc(ref Message m)
        {
            base.WndProc(ref m);
            if (m.Msg == 0x000F)
            {
                using (var g = Graphics.FromHwnd(Handle))
                {
                    var r = new Rectangle(0, 0, Width - 1, Height - 1);
                    using (var p = new Pen(Focused ? Theme.Accent : Theme.Line)) g.DrawRectangle(p, r);
                    int bw = SystemInformation.VerticalScrollBarWidth;
                    var br = new Rectangle(Width - bw - 1, 1, bw, Height - 2);
                    using (var b = new SolidBrush(Theme.Raised)) g.FillRectangle(b, br);
                    int cx = br.Left + br.Width / 2, cy = br.Top + br.Height / 2;
                    using (var b = new SolidBrush(Theme.TextDim)) g.FillPolygon(b, new[] { new Point(cx - 4, cy - 2), new Point(cx + 4, cy - 2), new Point(cx, cy + 3) });
                }
            }
        }
    }

    public class DarkToolTip : ToolTip
    {
        public DarkToolTip()
        {
            OwnerDraw = true;
            InitialDelay = 600;
            AutoPopDelay = 12000;
            Draw += (s, e) =>
            {
                using (var b = new SolidBrush(Theme.Raised)) e.Graphics.FillRectangle(b, e.Bounds);
                using (var p = new Pen(Theme.LineStrong)) e.Graphics.DrawRectangle(p, new Rectangle(0, 0, e.Bounds.Width - 1, e.Bounds.Height - 1));
                TextRenderer.DrawText(e.Graphics, e.ToolTipText, Theme.UiFont, new Rectangle(6, 4, e.Bounds.Width - 12, e.Bounds.Height - 8), Theme.Text, TextFormatFlags.NoPrefix | TextFormatFlags.Left | TextFormatFlags.Top | TextFormatFlags.WordBreak);
            };
            Popup += (s, e) =>
            {
                var sz = TextRenderer.MeasureText(GetToolTip(e.AssociatedControl), Theme.UiFont, new Size(420, 0), TextFormatFlags.WordBreak);
                e.ToolTipSize = new Size(sz.Width + 14, sz.Height + 10);
            };
        }
    }

    // panel that draws a one pixel border
    public class Card : Panel
    {
        public Color BorderColor = Theme.Line;
        public Card()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw | ControlStyles.UserPaint, true);
            BackColor = Theme.Panel;
            Padding = new Padding(1);
        }
        protected override void OnPaint(PaintEventArgs e)
        {
            base.OnPaint(e);
            using (var p = new Pen(BorderColor)) e.Graphics.DrawRectangle(p, 0, 0, Width - 1, Height - 1);
        }
    }

    // checkbox that draws its own box and check mark in theme colors
    public class DarkCheckBox : CheckBox
    {
        bool hover;
        const int BoxSize = 14, Gap = 7;

        public DarkCheckBox()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            Cursor = Cursors.Hand;
            Margin = new Padding(3, 4, 3, 4);
        }

        Color Back()
        {
            Control c = this;
            while (c.Parent != null)
            {
                c = c.Parent;
                if (c.BackColor != Color.Transparent) return c.BackColor;
            }
            return Theme.Window;
        }

        protected override void OnMouseEnter(EventArgs e) { hover = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hover = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnTextChanged(EventArgs e) { base.OnTextChanged(e); Invalidate(); }
        protected override void OnCheckedChanged(EventArgs e) { base.OnCheckedChanged(e); Invalidate(); }

        public override Size GetPreferredSize(Size proposedSize)
        {
            var ts = TextRenderer.MeasureText(Text, Font);
            return new Size(BoxSize + Gap + ts.Width, Math.Max(BoxSize, ts.Height) + 4);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(Back());
            var box = new Rectangle(0, (Height - BoxSize) / 2, BoxSize, BoxSize);
            bool hi = Enabled && (hover || Focused);
            using (var b = new SolidBrush(Checked ? Theme.AccentSoft : hi ? Theme.Hover : Theme.Panel)) g.FillRectangle(b, box);
            using (var p = new Pen(Enabled ? (Checked ? Theme.Accent : hi ? Theme.TextDim : Theme.LineStrong) : Theme.Line)) g.DrawRectangle(p, box);
            if (Checked)
            {
                using (var p = new Pen(Enabled ? Theme.Accent : Theme.TextFaint, 2f) { StartCap = LineCap.Round, EndCap = LineCap.Round, LineJoin = LineJoin.Round })
                {
                    g.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;
                    g.DrawLines(p, new[] { new Point(box.Left + 3, box.Top + 7), new Point(box.Left + 6, box.Top + 10), new Point(box.Left + 11, box.Top + 3) });
                }
            }
            TextRenderer.DrawText(g, Text, Font, new Rectangle(BoxSize + Gap, 0, Width - BoxSize - Gap, Height), Enabled ? ForeColor : Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine);
        }
    }

    public static class Ui
    {
        public static Label Label(string text, bool dim = false, Font font = null)
        {
            return new Label { Text = text, AutoSize = true, ForeColor = dim ? Theme.TextDim : Theme.Text, Font = font ?? Theme.UiFont, BackColor = Color.Transparent, Margin = new Padding(3, 6, 3, 3), UseMnemonic = false };
        }

        public static Label Header(string text) => new Label { Text = text, AutoSize = true, ForeColor = Theme.Text, Font = Theme.HeaderFont, Margin = new Padding(0, 4, 0, 8), UseMnemonic = false };

        public static TextBox Box(string text = "", int width = 200)
        {
            var t = new TextBox { Text = text, Width = width };
            Theme.Style(t);
            return t;
        }

        public static DarkCombo Combo(IEnumerable<string> items, int selected = 0, int width = 200)
        {
            var c = new DarkCombo { Width = width };
            foreach (var i in items) c.Items.Add(i);
            if (c.Items.Count > 0) c.SelectedIndex = Math.Max(0, Math.Min(selected, c.Items.Count - 1));
            return c;
        }

        public static CheckBox Check(string text, bool value)
        {
            var c = new DarkCheckBox { Text = text, Checked = value, AutoSize = true, Margin = new Padding(3, 4, 3, 4) };
            c.ForeColor = Theme.Text;
            return c;
        }

        public static NumericUpDown Number(decimal value, decimal min, decimal max, int width = 80)
        {
            var n = new NumericUpDown { Minimum = min, Maximum = max, Value = Math.Max(min, Math.Min(max, value)), Width = width };
            Theme.Style(n);
            return n;
        }

        // keeps a docked, wrapping row tall enough to show every child
        public static T FitHeight<T>(T f) where T : FlowLayoutPanel
        {
            f.WrapContents = true;
            bool busy = false;
            void Fit()
            {
                if (busy || f.Width <= 0) return;
                busy = true;
                int h = f.GetPreferredSize(new Size(f.Width, 0)).Height;
                if (h != f.Height)
                {
                    f.Height = h;
                    // a height change made inside the parent's layout pass is not docked again until asked
                    if (f.Parent != null && f.IsHandleCreated) f.BeginInvoke((Action)(() => f.Parent?.PerformLayout()));
                }
                busy = false;
            }
            f.Resize += (s, e) => Fit();
            f.ControlAdded += (s, e) => Fit();
            f.ControlRemoved += (s, e) => Fit();
            return f;
        }

        // sizes a docked side panel as a share of its parent's width
        public static T ShareWidth<T>(T side, float share, int min, int max) where T : Control
        {
            Control hooked = null;
            void Fit() { if (side.Parent != null) side.Width = (int)Math.Max(min, Math.Min(max, side.Parent.ClientSize.Width * share)); }
            side.ParentChanged += (s, e) =>
            {
                if (hooked != null) hooked.Resize -= (EventHandler)Handler;
                hooked = side.Parent;
                if (hooked != null) { hooked.Resize += Handler; Fit(); }
            };
            return side;

            void Handler(object s, EventArgs e) => Fit();
        }

        public static FlowLayoutPanel Row(params Control[] cs)
        {
            var f = new FlowLayoutPanel { AutoSize = true, FlowDirection = FlowDirection.LeftToRight, WrapContents = false, Margin = new Padding(0), BackColor = Color.Transparent };
            f.Controls.AddRange(cs);
            return f;
        }

        public static void Message(IWin32Window owner, string text, string title = "OrangCalc") => MessageDialog.Show(owner, text, title, false);
        public static bool Confirm(IWin32Window owner, string text, string title = "OrangCalc") => MessageDialog.Show(owner, text, title, true);
}   }