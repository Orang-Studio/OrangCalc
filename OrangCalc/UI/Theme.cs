using System;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public static class Theme
    {
        public static Color Window = Color.FromArgb(23, 25, 29);
        public static Color Panel = Color.FromArgb(30, 33, 38);
        public static Color Raised = Color.FromArgb(38, 42, 48);
        public static Color Hover = Color.FromArgb(48, 53, 61);
        public static Color Pressed = Color.FromArgb(58, 64, 73);
        public static Color Line = Color.FromArgb(50, 55, 63);
        public static Color LineStrong = Color.FromArgb(70, 76, 86);
        public static Color Text = Color.FromArgb(231, 233, 236);
        public static Color TextDim = Color.FromArgb(160, 167, 176);
        public static Color TextFaint = Color.FromArgb(110, 117, 126);
        public static Color Accent = Color.FromArgb(240, 138, 60);
        public static Color AccentText = Color.FromArgb(20, 20, 20);
        public static Color AccentSoft = Color.FromArgb(70, 48, 32);
        public static Color Error = Color.FromArgb(238, 104, 104);
        public static Color Good = Color.FromArgb(128, 196, 140);
        public static Color Selection = Color.FromArgb(58, 66, 78);

        public static Color TokNumber = Color.FromArgb(236, 238, 241);
        public static Color TokOperator = Color.FromArgb(150, 158, 168);
        public static Color TokUnit = Color.FromArgb(240, 168, 110);
        public static Color TokVariable = Color.FromArgb(132, 186, 232);
        public static Color TokFunction = Color.FromArgb(196, 168, 232);
        public static Color TokSymbol = Color.FromArgb(132, 206, 172);
        public static Color TokText = Color.FromArgb(214, 198, 140);

        public static Font UiFont = new Font("Segoe UI", 9f);
        public static Font UiBold = new Font("Segoe UI Semibold", 9f);
        public static Font SmallFont = new Font("Segoe UI", 7.5f);
        public static Font HeaderFont = new Font("Segoe UI Semibold", 12f);
        public static Font ExpressionFont = new Font("Consolas", 14f);
        public static Font ResultFont = new Font("Segoe UI Semibold", 22f);
        public static Font KeypadFont = new Font("Segoe UI", 11f);
        public static Font HistoryFont = new Font("Segoe UI", 10f);
        public static Font StatusFont = new Font("Segoe UI", 8.5f);
        public static Font MonoFont = new Font("Consolas", 10f);

        public static Color TokenColor(TokType t)
        {
            switch (t)
            {
                case TokType.Number: return TokNumber;
                case TokType.Operator: return TokOperator;
                case TokType.Paren: return TokOperator;
                case TokType.Unit: return TokUnit;
                case TokType.Variable: return TokVariable;
                case TokType.Function: return TokFunction;
                case TokType.Symbol: return TokSymbol;
                case TokType.Text: return TokText;
                case TokType.Error: return Error;
                default: return Text;
            }
        }

        public static readonly Color DefaultAccent = Color.FromArgb(240, 138, 60);

        public static Color Mix(Color a, Color b, double t)
        {
            t = Math.Max(0, Math.Min(1, t));
            return Color.FromArgb((int)(a.R + (b.R - a.R) * t), (int)(a.G + (b.G - a.G) * t), (int)(a.B + (b.B - a.B) * t));
        }

        static double Luma(Color c) => (0.2126 * c.R + 0.7152 * c.G + 0.0722 * c.B) / 255;

        // accent and the colors derived from it; open windows are recolored in place
        public static void SetAccent(Color c)
        {
            var old = new[] { Accent, AccentSoft, AccentText };
            Accent = c;
            AccentSoft = Mix(Window, c, 0.22);
            AccentText = Luma(c) > 0.5 ? Color.FromArgb(20, 20, 20) : Color.White;
            var now = new[] { Accent, AccentSoft, AccentText };
            foreach (Form f in Application.OpenForms)
            {
                Recolor(f, old, now);
                f.Invalidate(true);
                ClassicFrame.Redraw(f);
            }
        }

        static void Recolor(Control c, Color[] from, Color[] to)
        {
            for (int i = 0; i < from.Length; i++)
            {
                if (c.ForeColor.ToArgb() == from[i].ToArgb()) c.ForeColor = to[i];
                if (c.BackColor.ToArgb() == from[i].ToArgb()) c.BackColor = to[i];
                if (c is LinkLabel ll && ll.LinkColor.ToArgb() == from[i].ToArgb()) { ll.LinkColor = to[i]; ll.ActiveLinkColor = to[i]; }
            }
            foreach (Control k in c.Controls) Recolor(k, from, to);
        }

        // classic caption gradient: dark accent on the left fading to the accent
        public static Color CaptionStart => Mix(Accent, Color.Black, 0.5);
        public static Color CaptionEnd => Accent;
        public static Color CaptionText => Color.White;

        [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
        static extern int SetWindowTheme(IntPtr hwnd, string app, string idList);

        static bool IsWin10 => Environment.OSVersion.Version.Major >= 10;

        // turns off visual styles so a common control honors its back and fore colors
        public static void Unthemed(Control c)
        {
            if (c.IsHandleCreated) SetWindowTheme(c.Handle, "", "");
            c.HandleCreated += (s, e) => SetWindowTheme(c.Handle, "", "");
        }

        public static void DarkScrollbars(Control c)
        {
            if (!IsWin10) return;
            void Apply(Control x)
            {
                try { SetWindowTheme(x.Handle, "DarkMode_Explorer", null); } catch (Exception) { }
                foreach (var sb in x.Controls.OfType<ScrollBar>()) try { SetWindowTheme(sb.Handle, "DarkMode_Explorer", null); } catch (Exception) { }
            }
            if (c.IsHandleCreated) Apply(c);
            c.HandleCreated += (s, e) => Apply(c);
        }

        // applies colors recursively to standard controls
        public static void Apply(Control root)
        {
            Style(root);
            foreach (Control c in root.Controls) Apply(c);
        }

        public static void Style(Control c)
        {
            switch (c)
            {
                case TextBox tb:
                    tb.BackColor = Raised; tb.ForeColor = Text; tb.BorderStyle = BorderStyle.FixedSingle;
                    if (tb.Multiline) Hook(tb, DarkScrollbars);
                    break;
                case RichTextBox rb:
                    rb.BackColor = Raised; rb.ForeColor = Text; rb.BorderStyle = BorderStyle.None;
                    Hook(rb, DarkScrollbars);
                    break;
                case ComboBox cb:
                    cb.BackColor = Raised; cb.ForeColor = Text; cb.FlatStyle = FlatStyle.Flat;
                    break;
                case NumericUpDown nu:
                    nu.BackColor = Raised; nu.ForeColor = Text; nu.BorderStyle = BorderStyle.FixedSingle;
                    break;
                case CheckBox ch:
                    ch.ForeColor = Text; ch.FlatStyle = FlatStyle.Flat;
                    ch.FlatAppearance.BorderColor = LineStrong; ch.FlatAppearance.CheckedBackColor = Raised;
                    ch.FlatAppearance.MouseOverBackColor = Panel; ch.FlatAppearance.MouseDownBackColor = Panel;
                    break;
                case RadioButton rb2:
                    rb2.ForeColor = Text; rb2.FlatStyle = FlatStyle.Flat;
                    rb2.FlatAppearance.BorderColor = LineStrong; rb2.FlatAppearance.CheckedBackColor = Raised;
                    break;
                case Button b:
                    b.FlatStyle = FlatStyle.Flat; b.BackColor = Raised; b.ForeColor = Text;
                    b.FlatAppearance.BorderColor = Line; b.FlatAppearance.MouseOverBackColor = Hover; b.FlatAppearance.MouseDownBackColor = Pressed;
                    b.UseVisualStyleBackColor = false;
                    break;
                case ListBox lb:
                    lb.BackColor = Panel; lb.ForeColor = Text; lb.BorderStyle = BorderStyle.None;
                    Hook(lb, DarkScrollbars);
                    break;
                case TreeView tv:
                    tv.BackColor = Panel; tv.ForeColor = Text; tv.BorderStyle = BorderStyle.None; tv.LineColor = Line;
                    Hook(tv, DarkScrollbars);
                    break;
                case DataGridView g: StyleGrid(g); break;
                case Label l: if (l.ForeColor == SystemColors.ControlText) l.ForeColor = Text; break;
                case GroupBox gb: gb.ForeColor = TextDim; break;
                case TrackBar tr: tr.BackColor = c.Parent?.BackColor ?? Window; break;
                case DateTimePicker dp: break;
                case Panel p when p.AutoScroll: Hook(p, DarkScrollbars); break;
            }
        }

        static void Hook(Control c, Action<Control> a)
        {
            if (c.IsHandleCreated) a(c);
            else c.HandleCreated += (s, e) => a(c);
        }

        public static void StyleGrid(DataGridView g)
        {
            g.BackgroundColor = Panel;
            g.BorderStyle = BorderStyle.None;
            g.GridColor = Line;
            g.EnableHeadersVisualStyles = false;
            g.CellBorderStyle = DataGridViewCellBorderStyle.SingleHorizontal;
            g.ColumnHeadersBorderStyle = DataGridViewHeaderBorderStyle.Single;
            g.ColumnHeadersDefaultCellStyle.BackColor = Raised;
            g.ColumnHeadersDefaultCellStyle.ForeColor = TextDim;
            g.ColumnHeadersDefaultCellStyle.SelectionBackColor = Raised;
            g.ColumnHeadersDefaultCellStyle.Font = UiBold;
            g.ColumnHeadersDefaultCellStyle.Padding = new Padding(4, 0, 0, 0);
            g.ColumnHeadersDefaultCellStyle.WrapMode = DataGridViewTriState.False;
            g.ColumnHeadersHeight = 28;
            g.ColumnHeadersHeightSizeMode = DataGridViewColumnHeadersHeightSizeMode.DisableResizing;
            g.DefaultCellStyle.BackColor = Panel;
            g.DefaultCellStyle.ForeColor = Text;
            g.DefaultCellStyle.SelectionBackColor = Selection;
            g.DefaultCellStyle.SelectionForeColor = Text;
            g.DefaultCellStyle.Padding = new Padding(4, 0, 4, 0);
            g.AlternatingRowsDefaultCellStyle.BackColor = Panel;
            g.RowHeadersVisible = false;
            g.RowTemplate.Height = 26;
            g.AllowUserToResizeRows = false;
            g.Font = UiFont;
            DarkScrollbars(g);
        }

        public static void PrepareForm(Form f)
        {
            f.BackColor = Window;
            f.ForeColor = Text;
            f.Font = UiFont;
            f.AutoScaleMode = AutoScaleMode.Dpi;
            ClassicFrame.Attach(f);
        }

        public static Icon AppIcon
        {
            get
            {
                try
                {
                    var st = typeof(Theme).Assembly.GetManifestResourceStream("OrangCalc.orange.ico");
                    return st != null ? new Icon(st) : null;
                }
                catch (Exception) { return null; }
            }
        }
    }

    // flat solid menu renderer
    public sealed class DarkColors : ProfessionalColorTable
    {
        public override Color MenuItemSelected => Theme.Hover;
        public override Color MenuItemSelectedGradientBegin => Theme.Hover;
        public override Color MenuItemSelectedGradientEnd => Theme.Hover;
        public override Color MenuItemPressedGradientBegin => Theme.Raised;
        public override Color MenuItemPressedGradientMiddle => Theme.Raised;
        public override Color MenuItemPressedGradientEnd => Theme.Raised;
        public override Color MenuItemBorder => Theme.Hover;
        public override Color MenuBorder => Theme.LineStrong;
        public override Color ToolStripDropDownBackground => Theme.Raised;
        public override Color ImageMarginGradientBegin => Theme.Raised;
        public override Color ImageMarginGradientMiddle => Theme.Raised;
        public override Color ImageMarginGradientEnd => Theme.Raised;
        public override Color MenuStripGradientBegin => Theme.Window;
        public override Color MenuStripGradientEnd => Theme.Window;
        public override Color ToolStripGradientBegin => Theme.Window;
        public override Color ToolStripGradientMiddle => Theme.Window;
        public override Color ToolStripGradientEnd => Theme.Window;
        public override Color ToolStripBorder => Theme.Window;
        public override Color SeparatorDark => Theme.Line;
        public override Color SeparatorLight => Theme.Line;
        public override Color CheckBackground => Theme.AccentSoft;
        public override Color CheckSelectedBackground => Theme.AccentSoft;
        public override Color CheckPressedBackground => Theme.AccentSoft;
        public override Color ButtonSelectedHighlight => Theme.Hover;
        public override Color ButtonSelectedGradientBegin => Theme.Hover;
        public override Color ButtonSelectedGradientEnd => Theme.Hover;
        public override Color ButtonPressedGradientBegin => Theme.Pressed;
        public override Color ButtonPressedGradientEnd => Theme.Pressed;
        public override Color StatusStripGradientBegin => Theme.Window;
        public override Color StatusStripGradientEnd => Theme.Window;
    }

    public sealed class DarkRenderer : ToolStripProfessionalRenderer
    {
        public DarkRenderer() : base(new DarkColors()) { RoundedEdges = false; }

        protected override void OnRenderItemText(ToolStripItemTextRenderEventArgs e)
        {
            e.TextColor = e.Item.Enabled ? Theme.Text : Theme.TextFaint;
            if (e.Item is ToolStripMenuItem mi && mi.ShortcutKeyDisplayString != null && e.Text == mi.ShortcutKeyDisplayString) e.TextColor = Theme.TextFaint;
            base.OnRenderItemText(e);
        }

        protected override void OnRenderArrow(ToolStripArrowRenderEventArgs e)
        {
            e.ArrowColor = Theme.TextDim;
            base.OnRenderArrow(e);
        }

        protected override void OnRenderItemCheck(ToolStripItemImageRenderEventArgs e)
        {
            var r = e.ImageRectangle;
            using (var b = new SolidBrush(Theme.AccentSoft)) e.Graphics.FillRectangle(b, r);
            using (var p = new Pen(Theme.Accent, 2f))
            {
                e.Graphics.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;
                e.Graphics.DrawLines(p, new[] { new PointF(r.Left + r.Width * 0.22f, r.Top + r.Height * 0.52f), new PointF(r.Left + r.Width * 0.42f, r.Top + r.Height * 0.72f), new PointF(r.Left + r.Width * 0.78f, r.Top + r.Height * 0.3f) });
            }
        }

        protected override void OnRenderSeparator(ToolStripSeparatorRenderEventArgs e)
        {
            var r = e.Item.ContentRectangle;
            using (var p = new Pen(Theme.Line)) e.Graphics.DrawLine(p, r.Left + 28, r.Height / 2, r.Right - 4, r.Height / 2);
}   }   }