using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Net;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;
using System.Xml;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class ActionDef
    {
        public string Id, Title, DefaultKeys;
        public Action Run;
    }

    public sealed class MainForm : Form
    {
        public readonly Calculator Calc;
        public readonly CalcService Svc;
        public readonly AppSettings Settings = new AppSettings();
        readonly MenuStrip menu = new MenuStrip();
        readonly NavRail rail = new NavRail();
        readonly Panel host = new Panel { Dock = DockStyle.Fill, BackColor = Theme.Window };
        public CalcPage CalcView;
        readonly Dictionary<int, Control> pages = new Dictionary<int, Control>();
        public readonly List<ActionDef> Actions = new List<ActionDef>();
        readonly Dictionary<Keys, ActionDef> keyMap = new Dictionary<Keys, ActionDef>();
        bool dirty;
        string lastResultTitle = "";
        readonly Timer saveTimer = new Timer { Interval = 4000 };

        static readonly string[] PageNames = { "Calculator", "Functions", "Variables", "Units", "Data Sets", "Periodic Table", "Plot", "Tools", "Settings" };

        public MainForm()
        {
            Calc = new Calculator();
            Svc = new CalcService(Calc);
            Storage.LoadSettings(Settings, Calc.Opts);
            Theme.SetAccent(AppSettings.ParseColor(Settings.AccentColor, Theme.DefaultAccent));
            Calculator.Current = Calc;
            LoadRates();
            var errs = Storage.LoadDefinitions(Calc);
            Theme.PrepareForm(this);
            Text = "OrangCalc";
            Icon = Theme.AppIcon;
            MinimumSize = new Size(760, 640);
            KeyPreview = true;
            ApplyAppFont();

            menu.Renderer = new DarkRenderer();
            menu.BackColor = Theme.Window;
            menu.ForeColor = Theme.Text;
            menu.Padding = new Padding(6, 3, 0, 3);
            MainMenuStrip = menu;

            rail.Items.AddRange(PageNames);
            rail.GapBefore.Add(1); rail.GapBefore.Add(4); rail.GapBefore.Add(8);
            rail.Dock = DockStyle.Left;
            rail.SelectedChanged += (s, e) => ShowPage(rail.SelectedIndex);

            Controls.Add(host);
            Controls.Add(rail);
            Controls.Add(menu);

            var history = Storage.LoadHistory();
            CalcView = new CalcPage(this, Svc, Settings, history);
            pages[0] = CalcView;
            RegisterActions();
            BuildMenu();
            ApplyShortcuts();
            ShowPage(0);

            if (Settings.WinX >= 0 && Screen.AllScreens.Any(sc => sc.WorkingArea.IntersectsWith(new Rectangle(Settings.WinX, Settings.WinY, Settings.WinW, Settings.WinH))))
            {
                StartPosition = FormStartPosition.Manual;
                Bounds = new Rectangle(Settings.WinX, Settings.WinY, Settings.WinW, Settings.WinH);
            }
            else { Size = new Size(Settings.WinW, Settings.WinH); StartPosition = FormStartPosition.CenterScreen; }
            if (Settings.Maximized) WindowState = FormWindowState.Maximized;
            TopMost = Settings.AlwaysOnTop;

            Funcs.PlotRequested += (expr, a, b) => BeginInvoke((Action)(() => OpenPlot(expr, a, b)));
            Calc.DefinitionsChanged += () => { if (IsHandleCreated) BeginInvoke((Action)DefinitionsChanged); };
            saveTimer.Tick += (s, e) => { if (dirty) SaveAll(); };
            saveTimer.Start();
            Shown += (s, e) =>
            {
                CalcView.FocusExpression();
                if (errs.Count > 0) Ui.Message(this, "Some definitions could not be loaded:\n" + string.Join("\n", errs.Take(10)));
                MaybeUpdateRates();
                if (!string.IsNullOrEmpty(Settings.Workspace) && File.Exists(Settings.Workspace)) UpdateTitle(lastResultTitle);
            };
            UpdateTitle("");
        }

        void ApplyAppFont()
        {
            var f = AppSettings.ParseFont(Settings.AppFont, null);
            if (f != null) { Font = f; Theme.UiFont = f; }
        }

        public void MarkDirty() => dirty = true;

        public void SaveAll()
        {
            dirty = false;
            if (WindowState == FormWindowState.Normal) { Settings.WinX = Left; Settings.WinY = Top; Settings.WinW = Width; Settings.WinH = Height; }
            Settings.Maximized = WindowState == FormWindowState.Maximized;
            Settings.CurrentPage = rail.SelectedIndex;
            Storage.SaveSettings(Settings, Calc.Opts);
            Storage.SaveHistory(Settings.ClearHistoryOnExit && closing ? CalcView.History.Entries.Where(h => h.Protected) : CalcView.History.Entries);
            Storage.SaveDefinitions(Calc);
        }

        bool closing;

        protected override void OnFormClosing(FormClosingEventArgs e)
        {
            if (Settings.AskSaveWorkspace && !string.IsNullOrEmpty(Settings.Workspace) && Ui.Confirm(this, "Do you want to save the current workspace?"))
                SaveWorkspace(Settings.Workspace);
            closing = true;
            SaveAll();
            base.OnFormClosing(e);
        }

        protected override void OnFormClosed(FormClosedEventArgs e)
        {
            Svc.Dispose();
            base.OnFormClosed(e);
        }

        public void UpdateTitle(string result)
        {
            lastResultTitle = result ?? "";
            string ws = string.IsNullOrEmpty(Settings.Workspace) ? "" : Path.GetFileNameWithoutExtension(Settings.Workspace);
            switch (Settings.Title)
            {
                case TitleMode.AppNameAndResult: Text = lastResultTitle.Length > 0 ? "OrangCalc – " + Trim(lastResultTitle) : "OrangCalc"; break;
                case TitleMode.AppNameAndWorkspace: Text = ws.Length > 0 ? "OrangCalc – " + ws : "OrangCalc"; break;
                case TitleMode.ResultOnly: Text = lastResultTitle.Length > 0 ? Trim(lastResultTitle) : "OrangCalc"; break;
                default: Text = "OrangCalc"; break;
            }
        }

        static string Trim(string s) => s.Length > 80 ? s.Substring(0, 80) + "…" : s;

        public void ShowPage(int i)
        {
            if (rail.SelectedIndex != i) { rail.SelectedIndex = i; return; }
            if (!pages.TryGetValue(i, out var page))
            {
                switch (i)
                {
                    case 1: page = new ItemsPage(this, ItemKind.Function); break;
                    case 2: page = new ItemsPage(this, ItemKind.Variable); break;
                    case 3: page = new ItemsPage(this, ItemKind.Unit); break;
                    case 4: page = new DataSetsPage(this); break;
                    case 5: page = new PeriodicPage(this); break;
                    case 6: page = new PlotPage(this); break;
                    case 7: page = new ToolsPage(this); break;
                    case 8: page = new SettingsPage(this); break;
                    default: page = CalcView; break;
                }
                page.Dock = DockStyle.Fill;
                pages[i] = page;
            }
            host.SuspendLayout();
            host.Controls.Clear();
            host.Controls.Add(page);
            host.ResumeLayout();
            if (page is IPage p) p.Activated();
            if (i == 0) CalcView.FocusExpression();
        }

        public T Page<T>(int index) where T : Control
        {
            ShowPage(index);
            return pages[index] as T;
        }

        public void InsertIntoExpression(string text, bool calculate = false)
        {
            ShowPage(0);
            CalcView.Insert(text);
            if (calculate) CalcView.Calculate();
        }

        public void DefinitionsChanged()
        {
            foreach (var p in pages.Values.OfType<IPage>()) p.DefinitionsChanged();
            MarkDirty();
        }

        public void OptionsChanged()
        {
            Calc.ApplyPrecision();
            CalcView.RefreshModes();
            foreach (var p in pages.Values.OfType<IPage>()) p.OptionsChanged();
            MarkDirty();
        }

        public void AddRecent(List<string> list, string name)
        {
            list.Remove(name);
            list.Insert(0, name);
            if (list.Count > 8) list.RemoveAt(list.Count - 1);
        }

        void A(string id, string title, string keys, Action run) => Actions.Add(new ActionDef { Id = id, Title = title, DefaultKeys = keys, Run = run });

        void RegisterActions()
        {
            A("calculate", "Calculate expression", "", () => CalcView.Calculate());
            A("clear", "Clear expression", "", () => CalcView.ClearExpression());
            A("copy", "Copy result", "Ctrl+Shift+C", () => CalcView.CopyResult(false));
            A("copyascii", "Copy unformatted ASCII result", "", () => CalcView.CopyResult(true));
            A("store", "Store result as variable", "Ctrl+S", () => CalcView.StoreResult());
            A("newfunction", "New function", "Ctrl+Shift+F", () => NewFunction());
            A("newvariable", "New variable", "Ctrl+Shift+V", () => NewVariable());
            A("newunknown", "New unknown variable", "", () => NewUnknown());
            A("newunit", "New unit", "Ctrl+Shift+U", () => NewUnit());
            A("newmatrix", "New matrix", "Ctrl+Shift+M", () => CalcView.OpenMatrixEditor());
            A("insertdate", "Insert date", "Ctrl+Shift+D", () => CalcView.InsertDate());
            A("wsopen", "Open workspace", "Ctrl+O", () => OpenWorkspaceDialog());
            A("wssave", "Save workspace", "", () => { if (string.IsNullOrEmpty(Settings.Workspace)) SaveWorkspaceAs(); else SaveWorkspace(Settings.Workspace); });
            A("wssaveas", "Save workspace as", "", () => SaveWorkspaceAs());
            A("wsdefault", "Open default workspace", "", () => OpenDefaultWorkspace());
            A("csvimport", "Import CSV file", "", () => CsvTools.Import(this));
            A("csvexport", "Export CSV file", "", () => CsvTools.Export(this));
            A("calculator", "Show calculator", "Ctrl+1", () => ShowPage(0));
            A("functions", "Functions", "Ctrl+F", () => ShowPage(1));
            A("variables", "Variables and constants", "Ctrl+Shift+K", () => ShowPage(2));
            A("units", "Units", "Ctrl+U", () => ShowPage(3));
            A("datasets", "Data sets", "", () => ShowPage(4));
            A("periodic", "Periodic table", "Ctrl+Shift+P", () => ShowPage(5));
            A("plot", "Plot functions and data", "Ctrl+P", () => ShowPage(6));
            A("plotexpr", "Plot current expression", "", () => OpenPlot(CalcView.Expr.Box.Text.Trim().Length > 0 ? CalcView.Expr.Box.Text : "x", -10, 10));
            A("tools", "Tools", "", () => ShowPage(7));
            A("percentage", "Percentage calculation tool", "", () => Page<ToolsPage>(7).Select(0));
            A("calendar", "Calendar conversion", "", () => Page<ToolsPage>(7).Select(1));
            A("fpconv", "Floating point conversion", "", () => Page<ToolsPage>(7).Select(2));
            A("settings", "Preferences", "Ctrl+Comma", () => ShowPage(8));
            A("shortcuts", "Keyboard shortcuts", "", () => Page<SettingsPage>(8).ShowSection(6));
            A("rates", "Update exchange rates", "", () => UpdateRates(true));
            A("history", "Show history", "Ctrl+H", () => { ShowPage(0); CalcView.SelectSide(0); });
            A("stack", "Show RPN stack", "Ctrl+K", () => { ShowPage(0); CalcView.SelectSide(1); });
            A("bases", "Show number bases", "Ctrl+B", () => { ShowPage(0); CalcView.SelectSide(2); });
            A("keypad", "Toggle keypad", "Ctrl+Shift+Space", () => { Settings.KeypadVisible = !Settings.KeypadVisible; CalcView.ApplyLayout(); });
            A("sidepanel", "Toggle side panel", "Ctrl+Shift+H", () => { Settings.SideVisible = !Settings.SideVisible; CalcView.ApplyLayout(); });
            A("rpn", "RPN mode", "Ctrl+R", () => { Settings.RpnMode = !Settings.RpnMode; if (Settings.RpnMode && Calc.Opts.Parse == ParseMode.Chain) Calc.Opts.Parse = ParseMode.Adaptive; OptionsChanged(); if (Settings.RpnMode) CalcView.SelectSide(1); });
            A("chain", "Chain mode", "", () => { Settings.RpnMode = false; Calc.Opts.Parse = Calc.Opts.Parse == ParseMode.Chain ? ParseMode.Adaptive : ParseMode.Chain; OptionsChanged(); });
            A("normal", "Normal mode", "", () => { Settings.RpnMode = false; if (Calc.Opts.Parse == ParseMode.Chain) Calc.Opts.Parse = ParseMode.Adaptive; OptionsChanged(); });
            A("degrees", "Degrees", "", () => { Calc.Opts.Angle = AngleMode.Degrees; OptionsChanged(); });
            A("radians", "Radians", "", () => { Calc.Opts.Angle = AngleMode.Radians; OptionsChanged(); });
            A("gradians", "Gradians", "", () => { Calc.Opts.Angle = AngleMode.Gradians; OptionsChanged(); });
            A("exact", "Exact mode", "Ctrl+E", () => { Calc.Opts.Approx = Calc.Opts.Approx == ApproxMode.Exact ? ApproxMode.Auto : ApproxMode.Exact; OptionsChanged(); });
            A("fraction", "Toggle fraction display", "", () => { Calc.Opts.Fraction = Calc.Opts.Fraction == FractionMode.Fraction ? FractionMode.DecimalExact : FractionMode.Fraction; OptionsChanged(); });
            A("scientific", "Scientific notation", "", () => { Calc.Opts.Display = Calc.Opts.Display == NumberDisplay.Scientific ? NumberDisplay.Normal : NumberDisplay.Scientific; OptionsChanged(); });
            A("tohex", "Convert result to hexadecimal", "", () => CalcView.ConvertTo("hex"));
            A("tobin", "Convert result to binary", "", () => CalcView.ConvertTo("bin"));
            A("todec", "Convert result to decimal", "", () => CalcView.ConvertTo("dec"));
            A("tofraction", "Convert result to fraction", "", () => CalcView.ConvertTo("fraction"));
            A("tofactors", "Factorize result", "", () => CalcView.ConvertTo("factors"));
            A("expand", "Expand result", "", () => CalcView.ConvertTo("expand"));
            A("tobase", "Convert result to base units", "", () => CalcView.ConvertTo("base"));
            A("tooptimal", "Convert result to optimal unit", "", () => CalcView.ConvertTo("optimal"));
            A("clearhistory", "Clear history", "", () => CalcView.History.ClearUnprotected());
            A("clearstack", "Clear RPN stack", "", () => CalcView.StackOp("clear"));
            A("swap", "RPN swap", "", () => CalcView.StackOp("swap"));
            A("lastx", "RPN last x", "", () => CalcView.StackOp("lastx"));
            A("help", "Help", "F1", () => { using (var h = new HelpDialog()) h.ShowDialog(this); });
            A("about", "About", "", () => { using (var a = new AboutDialog()) a.ShowDialog(this); });
            A("bug", "Report a bug", "", () => OpenUrl("https://github.com/Orang-Studio/OrangCalc/issues"));
            A("settingsfolder", "Open settings folder", "", () => OpenUrl(Storage.Dir));
            A("quit", "Quit", "Ctrl+Q", () => Close());
            A("topmost", "Keep above other windows", "", () => { Settings.AlwaysOnTop = !Settings.AlwaysOnTop; TopMost = Settings.AlwaysOnTop; MarkDirty(); });
            A("undo", "Undo", "", () => CalcView.Expr.Undo());
            A("redo", "Redo", "", () => CalcView.Expr.Redo());
        }

        public void RunAction(string id)
        {
            var a = Actions.FirstOrDefault(x => x.Id == id);
            a?.Run();
        }

        public string ShortcutFor(ActionDef a) => Settings.Shortcuts.TryGetValue(a.Id, out var k) ? k : a.DefaultKeys;

        public void ApplyShortcuts()
        {
            keyMap.Clear();
            var conv = new KeysConverter();
            foreach (var a in Actions)
            {
                var s = ShortcutFor(a);
                if (string.IsNullOrEmpty(s)) continue;
                try
                {
                    var k = (Keys)conv.ConvertFromInvariantString(s.Replace("Comma", "Oemcomma"));
                    keyMap[k] = a;
                }
                catch (Exception) { }
            }
        }

        protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
        {
            if (keyMap.TryGetValue(keyData, out var a)) { a.Run(); return true; }
            return base.ProcessCmdKey(ref msg, keyData);
        }

        ToolStripMenuItem Item(string text, string action)
        {
            var a = Actions.FirstOrDefault(x => x.Id == action);
            var mi = new ToolStripMenuItem(text);
            mi.Click += (s, e) => RunAction(action);
            if (a != null) mi.ShortcutKeyDisplayString = ShortcutFor(a);
            return mi;
        }

        void BuildMenu()
        {
            menu.Items.Clear();
            var file = new ToolStripMenuItem("File");
            var nw = new ToolStripMenuItem("New");
            nw.DropDownItems.AddRange(new ToolStripItem[] { Item("Function…", "newfunction"), Item("Variable/Constant…", "newvariable"), Item("Unknown Variable…", "newunknown"), Item("Matrix…", "newmatrix"), Item("Unit…", "newunit") });
            file.DropDownItems.Add(nw);
            var ws = new ToolStripMenuItem("Workspaces");
            ws.DropDownOpening += (s, e) =>
            {
                ws.DropDownItems.Clear();
                ws.DropDownItems.AddRange(new ToolStripItem[] { Item("Open…", "wsopen"), Item("Open Default", "wsdefault"), new ToolStripSeparator(), Item("Save", "wssave"), Item("Save As…", "wssaveas") });
                if (Settings.RecentWorkspaces.Count > 0)
                {
                    ws.DropDownItems.Add(new ToolStripSeparator());
                    foreach (var r in Settings.RecentWorkspaces.Where(File.Exists).Take(8))
                    {
                        var path = r;
                        ws.DropDownItems.Add(new ToolStripMenuItem(Path.GetFileName(path), null, (o, a) => OpenWorkspace(path)));
                    }
                }
            };
            ws.DropDownItems.Add("…");
            file.DropDownItems.Add(ws);
            file.DropDownItems.Add(new ToolStripSeparator());
            file.DropDownItems.Add(Item("Import CSV File…", "csvimport"));
            file.DropDownItems.Add(Item("Export CSV File…", "csvexport"));
            file.DropDownItems.Add(new ToolStripSeparator());
            file.DropDownItems.Add(Item("Store Result…", "store"));
            file.DropDownItems.Add(new ToolStripSeparator());
            file.DropDownItems.Add(Item("Quit", "quit"));

            var edit = new ToolStripMenuItem("Edit");
            edit.DropDownItems.AddRange(new ToolStripItem[] { Item("Undo", "undo"), Item("Redo", "redo"), new ToolStripSeparator(), Item("Copy Result", "copy"), Item("Copy Unformatted ASCII", "copyascii"), new ToolStripSeparator(), Item("Insert Date…", "insertdate"), Item("Insert Matrix…", "newmatrix"), new ToolStripSeparator(), Item("Clear Expression", "clear"), Item("Clear History", "clearhistory"), Item("Clear RPN Stack", "clearstack") });

            var mode = new ToolStripMenuItem("Mode");
            mode.DropDownOpening += (s, e) =>
            {
                mode.DropDownItems.Clear();
                foreach (var id in new[] { "mode", "angle", "approx", "display", "base", "inbase", "fraction", "complex", "assumptions", "keys" })
                {
                    var sub = BuildModeMenu(id);
                    var mi = new ToolStripMenuItem(ModeTitle(id));
                    foreach (var it in sub.Items.Cast<ToolStripItem>().ToList()) mi.DropDownItems.Add(it);
                    mode.DropDownItems.Add(mi);
                }
                mode.DropDownItems.Add(new ToolStripSeparator());
                var top = Item("Keep Above Other Windows", "topmost");
                top.Checked = Settings.AlwaysOnTop;
                mode.DropDownItems.Add(top);
                mode.DropDownItems.Add(Item("Keyboard Shortcuts", "shortcuts"));
                mode.DropDownItems.Add(Item("Preferences", "settings"));
            };
            mode.DropDownItems.Add("…");

            var functions = new ToolStripMenuItem("Functions");
            functions.DropDownOpening += (s, e) => { functions.DropDownItems.Clear(); foreach (var it in BuildFunctionsMenu().Items.Cast<ToolStripItem>().ToList()) functions.DropDownItems.Add(it); };
            functions.DropDownItems.Add("…");
            var variables = new ToolStripMenuItem("Variables");
            variables.DropDownOpening += (s, e) => { variables.DropDownItems.Clear(); foreach (var it in BuildVariablesMenu(v => InsertIntoExpression(v)).Items.Cast<ToolStripItem>().ToList()) variables.DropDownItems.Add(it); };
            variables.DropDownItems.Add("…");
            var units = new ToolStripMenuItem("Units");
            units.DropDownOpening += (s, e) => { units.DropDownItems.Clear(); foreach (var it in BuildUnitsMenu(u => InsertIntoExpression(u)).Items.Cast<ToolStripItem>().ToList()) units.DropDownItems.Add(it); };
            units.DropDownItems.Add("…");

            var tools = new ToolStripMenuItem("Tools");
            tools.DropDownItems.AddRange(new ToolStripItem[] {
                Item("Functions", "functions"), Item("Variables and Constants", "variables"), Item("Units", "units"), Item("Data Sets", "datasets"), new ToolStripSeparator(),
                Item("Plot Functions/Data", "plot"), Item("Floating Point Conversion (IEEE 754)", "fpconv"), Item("Calendar Conversion", "calendar"), Item("Percentage Calculation Tool", "percentage"), Item("Periodic Table", "periodic"), new ToolStripSeparator(),
                Item("Update Exchange Rates", "rates"), new ToolStripSeparator(), Item("Open Settings Folder", "settingsfolder") });

            var help = new ToolStripMenuItem("Help");
            help.DropDownItems.AddRange(new ToolStripItem[] { Item("Help", "help"), Item("Report a Bug", "bug"), new ToolStripSeparator(), Item("About OrangCalc", "about") });

            menu.Items.AddRange(new ToolStripItem[] { file, edit, mode, functions, variables, units, tools, help });
            foreach (ToolStripMenuItem t in menu.Items) t.ForeColor = Theme.Text;
        }

        static string ModeTitle(string id)
        {
            switch (id)
            {
                case "mode": return "Calculation Mode";
                case "angle": return "Angle Unit";
                case "approx": return "Approximation";
                case "display": return "Number Display";
                case "base": return "Result Base";
                case "inbase": return "Expression Base";
                case "fraction": return "Fractions";
                case "complex": return "Complex Form";
                case "assumptions": return "Assumptions";
                case "keys": return "Keypad";
                default: return id;
            }
        }

        ToolStripMenuItem Radio(string text, bool check, Action a)
        {
            var mi = new ToolStripMenuItem(text) { Checked = check };
            mi.Click += (s, e) => { a(); OptionsChanged(); };
            return mi;
        }

        public ContextMenuStrip BuildModeMenu(string id)
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            var o = Calc.Opts;
            switch (id)
            {
                case "mode":
                    m.Items.Add(Radio("Normal Mode", !Settings.RpnMode && o.Parse != ParseMode.Chain, () => { Settings.RpnMode = false; if (o.Parse == ParseMode.Chain) o.Parse = ParseMode.Adaptive; }));
                    m.Items.Add(Radio("RPN Mode", Settings.RpnMode, () => { Settings.RpnMode = true; if (o.Parse == ParseMode.Chain) o.Parse = ParseMode.Adaptive; CalcView.SelectSide(1); }));
                    m.Items.Add(Radio("Chain Mode", o.Parse == ParseMode.Chain, () => { Settings.RpnMode = false; o.Parse = ParseMode.Chain; }));
                    m.Items.Add(new ToolStripSeparator());
                    var pm = new ToolStripMenuItem("Parsing Mode");
                    pm.DropDownItems.Add(Radio("Adaptive", o.Parse == ParseMode.Adaptive, () => o.Parse = ParseMode.Adaptive));
                    pm.DropDownItems.Add(Radio("Conventional", o.Parse == ParseMode.Conventional, () => o.Parse = ParseMode.Conventional));
                    pm.DropDownItems.Add(Radio("Implicit Multiplication First", o.Parse == ParseMode.ImplicitFirst, () => o.Parse = ParseMode.ImplicitFirst));
                    pm.DropDownItems.Add(Radio("RPN Syntax", o.Parse == ParseMode.Rpn, () => o.Parse = ParseMode.Rpn));
                    m.Items.Add(pm);
                    var tm = new ToolStripMenuItem("Temperature Calculation");
                    tm.DropDownItems.Add(Radio("Hybrid", o.Temperature == TemperatureMode.Hybrid, () => o.Temperature = TemperatureMode.Hybrid));
                    tm.DropDownItems.Add(Radio("Absolute", o.Temperature == TemperatureMode.Absolute, () => o.Temperature = TemperatureMode.Absolute));
                    tm.DropDownItems.Add(Radio("Relative", o.Temperature == TemperatureMode.Relative, () => o.Temperature = TemperatureMode.Relative));
                    m.Items.Add(tm);
                    var pc = new ToolStripMenuItem("Percentage Interpretation");
                    pc.DropDownItems.Add(Radio("Add percentage of original value (100 + 10% = 110)", o.SimplifiedPercent, () => o.SimplifiedPercent = true));
                    pc.DropDownItems.Add(Radio("Add percentage multiplied by 1/100 (100 + 10% = 100.1)", !o.SimplifiedPercent, () => o.SimplifiedPercent = false));
                    m.Items.Add(pc);
                    var iv = new ToolStripMenuItem("Interval Calculation");
                    iv.DropDownItems.Add(Radio("Variance Formula", o.Interval == IntervalCalc.VarianceFormula, () => o.Interval = IntervalCalc.VarianceFormula));
                    iv.DropDownItems.Add(Radio("Interval Arithmetic", o.Interval == IntervalCalc.IntervalArithmetic, () => o.Interval = IntervalCalc.IntervalArithmetic));
                    iv.DropDownItems.Add(Radio("Simple", o.Interval == IntervalCalc.Simple, () => o.Interval = IntervalCalc.Simple));
                    m.Items.Add(iv);
                    var st = new ToolStripMenuItem("Result Structure");
                    st.DropDownItems.Add(Radio("Simplify", o.Structure == StructMode.Simplify, () => o.Structure = StructMode.Simplify));
                    st.DropDownItems.Add(Radio("Factorize", o.Structure == StructMode.Factorize, () => o.Structure = StructMode.Factorize));
                    st.DropDownItems.Add(Radio("Expand", o.Structure == StructMode.Expand, () => o.Structure = StructMode.Expand));
                    st.DropDownItems.Add(Radio("None", o.Structure == StructMode.None, () => o.Structure = StructMode.None));
                    m.Items.Add(st);
                    break;
                case "angle":
                    m.Items.Add(Radio("Radians", o.Angle == AngleMode.Radians, () => o.Angle = AngleMode.Radians));
                    m.Items.Add(Radio("Degrees", o.Angle == AngleMode.Degrees, () => o.Angle = AngleMode.Degrees));
                    m.Items.Add(Radio("Gradians", o.Angle == AngleMode.Gradians, () => o.Angle = AngleMode.Gradians));
                    m.Items.Add(Radio("None", o.Angle == AngleMode.None, () => o.Angle = AngleMode.None));
                    break;
                case "approx":
                    m.Items.Add(Radio("Automatic", o.Approx == ApproxMode.Auto, () => o.Approx = ApproxMode.Auto));
                    m.Items.Add(Radio("Dual", o.Approx == ApproxMode.Dual, () => o.Approx = ApproxMode.Dual));
                    m.Items.Add(Radio("Exact", o.Approx == ApproxMode.Exact, () => o.Approx = ApproxMode.Exact));
                    m.Items.Add(Radio("Approximate", o.Approx == ApproxMode.Approximate, () => o.Approx = ApproxMode.Approximate));
                    m.Items.Add(new ToolStripSeparator());
                    m.Items.Add(Radio("Interval Arithmetic", o.Interval == IntervalCalc.IntervalArithmetic, () => o.Interval = o.Interval == IntervalCalc.IntervalArithmetic ? IntervalCalc.VarianceFormula : IntervalCalc.IntervalArithmetic));
                    break;
                case "display":
                    m.Items.Add(Radio("Normal", o.Display == NumberDisplay.Normal, () => o.Display = NumberDisplay.Normal));
                    m.Items.Add(Radio("Scientific", o.Display == NumberDisplay.Scientific, () => o.Display = NumberDisplay.Scientific));
                    m.Items.Add(Radio("Purely Scientific", o.Display == NumberDisplay.PureScientific, () => o.Display = NumberDisplay.PureScientific));
                    m.Items.Add(Radio("Engineering", o.Display == NumberDisplay.Engineering, () => o.Display = NumberDisplay.Engineering));
                    m.Items.Add(Radio("Simple", o.Display == NumberDisplay.Simple, () => o.Display = NumberDisplay.Simple));
                    m.Items.Add(new ToolStripSeparator());
                    var prec = new ToolStripMenuItem("Precision: " + o.Precision);
                    foreach (int p in new[] { 6, 8, 10, 12, 15, 20, 30, 50, 100 }) { int pv = p; prec.DropDownItems.Add(Radio(p.ToString(), o.Precision == p, () => o.Precision = pv)); }
                    prec.DropDownItems.Add(new ToolStripMenuItem("Custom…", null, (s, e) => { var t = InputDialog.Ask(this, "Precision", "Significant digits (2–5000):", o.Precision.ToString()); if (int.TryParse(t, out int v) && v >= 2 && v <= 5000) { o.Precision = v; OptionsChanged(); } }));
                    m.Items.Add(prec);
                    var maxd = new ToolStripMenuItem("Max Decimals: " + (o.MaxDecimals < 0 ? "off" : o.MaxDecimals.ToString()));
                    maxd.DropDownItems.Add(Radio("Off", o.MaxDecimals < 0, () => o.MaxDecimals = -1));
                    foreach (int p in new[] { 0, 1, 2, 3, 4, 6, 8 }) { int pv = p; maxd.DropDownItems.Add(Radio(p.ToString(), o.MaxDecimals == p, () => o.MaxDecimals = pv)); }
                    m.Items.Add(maxd);
                    var mind = new ToolStripMenuItem("Min Decimals: " + o.MinDecimals);
                    foreach (int p in new[] { 0, 1, 2, 3, 4, 6, 8 }) { int pv = p; mind.DropDownItems.Add(Radio(p.ToString(), o.MinDecimals == p, () => o.MinDecimals = pv)); }
                    m.Items.Add(mind);
                    var iv2 = new ToolStripMenuItem("Interval Display");
                    foreach (IntervalDisplay d in Enum.GetValues(typeof(IntervalDisplay))) { var dv = d; iv2.DropDownItems.Add(Radio(ExpressionBox.Words(d.ToString()), o.IntervalOut == d, () => o.IntervalOut = dv)); }
                    m.Items.Add(iv2);
                    break;
                case "base":
                case "inbase":
                    {
                        bool input = id == "inbase";
                        int cur = input ? o.InputBase : o.OutputBase;
                        Action<int> set = b => { if (input) o.InputBase = b; else o.OutputBase = b; };
                        foreach (var b in new[] { (2, "Binary"), (8, "Octal"), (10, "Decimal"), (16, "Hexadecimal") }) { var bv = b.Item1; m.Items.Add(Radio(b.Item2, cur == bv, () => set(bv))); }
                        var other = new ToolStripMenuItem("Other");
                        var list = input ? new[] { (12, "Duodecimal"), (Bases.Roman, "Roman Numerals") } :
                            new[] { (12, "Duodecimal"), (Bases.Sexagesimal, "Sexagesimal"), (Bases.Time, "Time Format"), (Bases.Roman, "Roman Numerals"), (Bases.Unicode, "Unicode"), (Bases.Bijective26, "Bijective Base-26"), (Bases.Bcd, "Binary-Coded Decimal (BCD)"), (Bases.Fp32, "Float (32-bit)"), (Bases.Fp64, "Double (64-bit)"), (Bases.Golden, "Golden Ratio (φ)"), (Bases.Supergolden, "Supergolden Ratio (ψ)"), (Bases.PiBase, "π"), (Bases.EBase, "e"), (Bases.Sqrt2, "√2") };
                        foreach (var b in list) { var bv = b.Item1; other.DropDownItems.Add(Radio(b.Item2, cur == bv, () => set(bv))); }
                        other.DropDownItems.Add(new ToolStripMenuItem("Custom…", null, (s, e) =>
                        {
                            var t = InputDialog.Ask(this, input ? "Expression Base" : "Result Base", "Base (2–36, negative, or an expression like 1.5 or pi):", "");
                            if (string.IsNullOrWhiteSpace(t)) return;
                            if (int.TryParse(t.Trim(), out int v) && Math.Abs(v) >= 2) set(v);
                            else if (!input) { o.OutputBase = Bases.Custom; o.OutputBaseCustom = t.Trim(); }
                            OptionsChanged();
                        }));
                        m.Items.Add(other);
                        if (!input)
                        {
                            m.Items.Add(new ToolStripSeparator());
                            var tc = new ToolStripMenuItem("Two's Complement");
                            tc.DropDownItems.Add(Radio("Off", o.TwosOut == TwosComplement.Off, () => o.TwosOut = TwosComplement.Off));
                            tc.DropDownItems.Add(Radio("Binary", o.TwosOut == TwosComplement.Binary, () => o.TwosOut = TwosComplement.Binary));
                            tc.DropDownItems.Add(Radio("Binary and Hexadecimal", o.TwosOut == TwosComplement.BinaryHex, () => o.TwosOut = TwosComplement.BinaryHex));
                            m.Items.Add(tc);
                            var bits = new ToolStripMenuItem("Binary Bits: " + (o.BinaryBits == 0 ? "auto" : o.BinaryBits.ToString()));
                            foreach (int bb in new[] { 0, 8, 16, 32, 64, 128 }) { int bv = bb; bits.DropDownItems.Add(Radio(bb == 0 ? "Automatic" : bb.ToString(), o.BinaryBits == bb, () => o.BinaryBits = bv)); }
                            m.Items.Add(bits);
                            m.Items.Add(new ToolStripSeparator());
                            var ib = new ToolStripMenuItem("Expression Base");
                            foreach (var it in BuildModeMenu("inbase").Items.Cast<ToolStripItem>().ToList()) ib.DropDownItems.Add(it);
                            m.Items.Add(ib);
                        }
                        break;
                    }
                case "fraction":
                    m.Items.Add(Radio("Decimal", o.Fraction == FractionMode.Decimal, () => o.Fraction = FractionMode.Decimal));
                    m.Items.Add(Radio("Exact Decimal", o.Fraction == FractionMode.DecimalExact, () => o.Fraction = FractionMode.DecimalExact));
                    m.Items.Add(Radio("Fraction", o.Fraction == FractionMode.Fraction, () => o.Fraction = FractionMode.Fraction));
                    m.Items.Add(Radio("Mixed Fraction", o.Fraction == FractionMode.Combined, () => o.Fraction = FractionMode.Combined));
                    m.Items.Add(Radio("Percent", o.Fraction == FractionMode.Percent, () => o.Fraction = FractionMode.Percent));
                    m.Items.Add(Radio("Per Mille", o.Fraction == FractionMode.Permille, () => o.Fraction = FractionMode.Permille));
                    m.Items.Add(Radio("Per Myriad", o.Fraction == FractionMode.Permyriad, () => o.Fraction = FractionMode.Permyriad));
                    m.Items.Add(new ToolStripSeparator());
                    var rep = new ToolStripMenuItem("Indicate Repeating Decimals");
                    rep.DropDownItems.Add(Radio("Off", o.Repeating == RepeatingMode.None, () => o.Repeating = RepeatingMode.None));
                    rep.DropDownItems.Add(Radio("Ellipsis", o.Repeating == RepeatingMode.Ellipsis, () => o.Repeating = RepeatingMode.Ellipsis));
                    rep.DropDownItems.Add(Radio("Overline", o.Repeating == RepeatingMode.Overline, () => o.Repeating = RepeatingMode.Overline));
                    m.Items.Add(rep);
                    break;
                case "complex":
                    m.Items.Add(Radio("Rectangular", o.Complex == ComplexForm.Rectangular, () => o.Complex = ComplexForm.Rectangular));
                    m.Items.Add(Radio("Exponential", o.Complex == ComplexForm.Exponential, () => o.Complex = ComplexForm.Exponential));
                    m.Items.Add(Radio("Polar", o.Complex == ComplexForm.Polar, () => o.Complex = ComplexForm.Polar));
                    m.Items.Add(Radio("Cis", o.Complex == ComplexForm.Cis, () => o.Complex = ComplexForm.Cis));
                    m.Items.Add(Radio("Angle/Phasor", o.Complex == ComplexForm.Angle, () => o.Complex = ComplexForm.Angle));
                    m.Items.Add(new ToolStripSeparator());
                    m.Items.Add(Radio("Allow Complex Numbers", o.ComplexEnabled, () => o.ComplexEnabled = !o.ComplexEnabled));
                    m.Items.Add(Radio("Allow Infinite Numbers", o.InfiniteEnabled, () => o.InfiniteEnabled = !o.InfiniteEnabled));
                    break;
                case "assumptions":
                    foreach (AssumeType t in Enum.GetValues(typeof(AssumeType))) { var tv = t; m.Items.Add(Radio(t.ToString(), o.DefaultType == t, () => o.DefaultType = tv)); }
                    m.Items.Add(new ToolStripSeparator());
                    foreach (AssumeSign sg in Enum.GetValues(typeof(AssumeSign))) { var sv = sg; m.Items.Add(Radio(ExpressionBox.Words(sg.ToString()), o.DefaultSign == sg, () => o.DefaultSign = sv)); }
                    m.Items.Add(new ToolStripSeparator());
                    m.Items.Add(new ToolStripMenuItem("Unknown variables (e.g. x, y, z) are by default assumed " + o.DefaultType.ToString().ToLowerInvariant() + ".") { Enabled = false });
                    break;
                case "keys":
                    foreach (KeypadMode km in Enum.GetValues(typeof(KeypadMode)))
                    {
                        var kv = km;
                        m.Items.Add(Radio(km == KeypadMode.NumberPadOnly ? "Number Pad" : km.ToString(), Settings.Keypad == km, () => { Settings.KeypadVisible = kv != KeypadMode.None; CalcView.Keypad.SelectMode(kv == KeypadMode.None ? KeypadMode.General : kv); Settings.Keypad = kv; CalcView.ApplyLayout(); }));
                    }
                    m.Items.Add(new ToolStripSeparator());
                    m.Items.Add(Radio("Always Show Number Pad", Settings.NumpadVisible, () => { Settings.NumpadVisible = !Settings.NumpadVisible; CalcView.ApplyLayout(); }));
                    m.Items.Add(Radio("Side Panel on the Left", Settings.SideLeft, () => { Settings.SideLeft = !Settings.SideLeft; Ui.Message(this, "The side panel position changes after a restart."); }));
                    break;
                default: return null;
            }
            return m;
        }

        public ContextMenuStrip BuildFunctionsMenu()
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            void AddF(ToolStripItemCollection items, MathFunction f)
            {
                var mi = new ToolStripMenuItem(f.DisplayTitle + "  (" + f.Names[0] + ")");
                mi.Click += (s, e) => InsertFunctionWithDialog(f);
                items.Add(mi);
            }
            var fav = Settings.FavoriteFunctions.Select(n => Calc.FindFunction(n)).Where(f => f != null).ToList();
            if (fav.Count > 0) { var sub = new ToolStripMenuItem("Favorites"); foreach (var f in fav) AddF(sub.DropDownItems, f); m.Items.Add(sub); }
            var rec = Settings.RecentFunctions.Select(n => Calc.FindFunction(n)).Where(f => f != null).ToList();
            if (rec.Count > 0) { var sub = new ToolStripMenuItem("Recent"); foreach (var f in rec) AddF(sub.DropDownItems, f); m.Items.Add(sub); }
            m.Items.Add(new ToolStripSeparator());
            foreach (var g in Calc.Functions.Where(f => f.Active && !f.Hidden && !f.Name.StartsWith("_")).GroupBy(f => f.Category.Length == 0 ? "Other" : f.Category).OrderBy(g => g.Key))
            {
                var sub = new ToolStripMenuItem(g.Key);
                foreach (var f in g.OrderBy(f => f.DisplayTitle)) AddF(sub.DropDownItems, f);
                m.Items.Add(sub);
            }
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add(Item("New Function…", "newfunction"));
            m.Items.Add(Item("Show All Functions", "functions"));
            return m;
        }

        public void InsertFunctionWithDialog(MathFunction f)
        {
            AddRecent(Settings.RecentFunctions, f.Names[0]);
            ShowPage(0);
            var sel = CalcView.Expr.Box.SelectedText;
            if (f.MaxArgs == 0) { CalcView.Insert(f.Names[0] + "()"); return; }
            using (var d = new InsertFunctionDialog(Calc, f, sel))
            {
                if (d.ShowDialog(this) != DialogResult.OK) return;
                if (sel.Length > 0) CalcView.Expr.Box.SelectedText = "";
                CalcView.Insert(d.CallText);
                if (d.CalculateNow) CalcView.Calculate();
            }
        }

        public ContextMenuStrip BuildVariablesMenu(Action<string> insert)
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            void AddV(ToolStripItemCollection items, Variable v)
            {
                var mi = new ToolStripMenuItem(v.DisplayTitle + "  (" + v.Names[0] + ")");
                mi.Click += (s, e) => { AddRecent(Settings.RecentVariables, v.Names[0]); insert(v.Names[0]); };
                items.Add(mi);
            }
            var fav = Settings.FavoriteVariables.Select(n => Calc.FindVariable(n)).Where(v => v != null).ToList();
            if (fav.Count > 0) { var sub = new ToolStripMenuItem("Favorites"); foreach (var v in fav) AddV(sub.DropDownItems, v); m.Items.Add(sub); }
            var rec = Settings.RecentVariables.Select(n => Calc.FindVariable(n)).Where(v => v != null).ToList();
            if (rec.Count > 0) { var sub = new ToolStripMenuItem("Recent"); foreach (var v in rec) AddV(sub.DropDownItems, v); m.Items.Add(sub); }
            m.Items.Add(new ToolStripSeparator());
            foreach (var g in Calc.Variables.Where(v => v.Active && !v.Hidden).GroupBy(v => v.Category.Length == 0 ? "Other" : v.Category).OrderBy(g => g.Key))
            {
                var sub = new ToolStripMenuItem(g.Key);
                foreach (var v in g.OrderBy(v => v.DisplayTitle)) AddV(sub.DropDownItems, v);
                m.Items.Add(sub);
            }
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add(Item("New Variable…", "newvariable"));
            m.Items.Add(Item("New Unknown Variable…", "newunknown"));
            m.Items.Add(Item("Show All Variables", "variables"));
            return m;
        }

        public ContextMenuStrip BuildUnitsMenu(Action<string> insert)
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            void AddU(ToolStripItemCollection items, Unit u)
            {
                var mi = new ToolStripMenuItem(u.DisplayTitle + "  (" + u.Abbrev + ")");
                mi.Click += (s, e) => { AddRecent(Settings.RecentUnits, u.Abbrev); insert(u.Abbrev); };
                items.Add(mi);
            }
            var fav = Settings.FavoriteUnits.Select(n => Calc.FindUnit(n, out _, false)).Where(u => u != null).ToList();
            if (fav.Count > 0) { var sub = new ToolStripMenuItem("Favorites"); foreach (var u in fav) AddU(sub.DropDownItems, u); m.Items.Add(sub); }
            var rec = Settings.RecentUnits.Select(n => Calc.FindUnit(n, out _, false)).Where(u => u != null).ToList();
            if (rec.Count > 0) { var sub = new ToolStripMenuItem("Recent"); foreach (var u in rec) AddU(sub.DropDownItems, u); m.Items.Add(sub); }
            m.Items.Add(new ToolStripSeparator());
            foreach (var g in Calc.Units.Where(u => u.Active && !u.Hidden).GroupBy(u => u.Category).OrderBy(g => g.Key))
            {
                var sub = new ToolStripMenuItem(g.Key);
                foreach (var u in g.OrderBy(u => u.DisplayTitle)) AddU(sub.DropDownItems, u);
                m.Items.Add(sub);
            }
            var pre = new ToolStripMenuItem("Prefixes");
            foreach (var p in Units.Prefixes)
            {
                var pp = p;
                pre.DropDownItems.Add(new ToolStripMenuItem(p.Name + " (" + p.Symbol + ", " + (p.Binary ? "2^" : "10^") + p.Exp + ")", null, (s, e) => insert(pp.Symbol)));
            }
            m.Items.Add(pre);
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add(Item("New Unit…", "newunit"));
            m.Items.Add(Item("Show All Units", "units"));
            return m;
        }

        public ContextMenuStrip BuildConversionMenu(Action<string> convert)
        {
            var m = new ContextMenuStrip { Renderer = new DarkRenderer() };
            ToolStripMenuItem C(string text, string target) => new ToolStripMenuItem(text, null, (s, e) => convert(target));
            var bases = new ToolStripMenuItem("Number Base");
            bases.DropDownItems.AddRange(new ToolStripItem[] { C("Binary", "bin"), C("Octal", "oct"), C("Decimal", "dec"), C("Duodecimal", "duo"), C("Hexadecimal", "hex"), new ToolStripSeparator(),
                C("Sexagesimal", "sexagesimal"), C("Time Format", "time"), C("Roman Numerals", "roman"), C("Bijective Base-26", "bijective"), C("Binary-Coded Decimal", "bcd"), C("Unicode", "unicode"),
                C("Latitude", "latitude"), C("Longitude", "longitude"), new ToolStripSeparator(),
                C("16-bit Floating Point", "fp16"), C("32-bit Floating Point", "fp32"), C("64-bit Floating Point", "fp64"), C("80-bit (x86) Floating Point", "fp80"), C("128-bit Floating Point", "fp128"), new ToolStripSeparator(),
                C("All Number Bases", "bases"), new ToolStripMenuItem("Other Base…", null, (s, e) => { var t = InputDialog.Ask(this, "Number Base", "Base:", "3"); if (!string.IsNullOrWhiteSpace(t)) convert("base " + t.Trim()); }) });
            m.Items.Add(bases);
            var fmt = new ToolStripMenuItem("Number Format");
            fmt.DropDownItems.AddRange(new ToolStripItem[] { C("Fraction", "fraction"), C("Mixed Fraction", "mixed"), C("Decimal Fraction", "decimals"), C("Percent", "percent"), new ToolStripSeparator(), C("Scientific Notation", "scientific"), C("Engineering Notation", "engineering"), C("Simple Notation", "simple") });
            m.Items.Add(fmt);
            var alg = new ToolStripMenuItem("Algebra");
            alg.DropDownItems.AddRange(new ToolStripItem[] { C("Factors", "factors"), C("Expanded", "expand"), C("Expanded Partial Fractions", "partial fraction"), C("Exact", "exact"), C("Approximate", "approximate") });
            m.Items.Add(alg);
            var cx = new ToolStripMenuItem("Complex Form");
            cx.DropDownItems.AddRange(new ToolStripItem[] { C("Rectangular", "rectangular"), C("Exponential", "exponential"), C("Polar", "polar"), C("Cis", "cis"), C("Angle/Phasor", "angle") });
            m.Items.Add(cx);
            var tm = new ToolStripMenuItem("Date and Time");
            tm.DropDownItems.AddRange(new ToolStripItem[] { C("Calendars", "calendars"), C("UTC Time Zone", "utc"), new ToolStripMenuItem("Other Time Zone…", null, (s, e) => { var t = InputDialog.Ask(this, "Time Zone", "Offset from UTC (e.g. +3 or -5:30):", "+0"); if (!string.IsNullOrWhiteSpace(t)) convert("utc" + t.Trim()); }) });
            m.Items.Add(tm);
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add(C("Optimal Unit", "optimal"));
            m.Items.Add(C("Base Units", "base"));
            m.Items.Add(C("Optimal Prefix", "prefix"));
            m.Items.Add(C("Mixed Units", "mixed"));
            var units = new ToolStripMenuItem("Unit");
            foreach (var g in Calc.Units.Where(u => u.Active && !u.Hidden).GroupBy(u => u.Category).OrderBy(g => g.Key))
            {
                var sub = new ToolStripMenuItem(g.Key);
                foreach (var u in g.OrderBy(u => u.DisplayTitle)) { var ab = u.Abbrev; sub.DropDownItems.Add(C(u.DisplayTitle + " (" + ab + ")", ab)); }
                units.DropDownItems.Add(sub);
            }
            m.Items.Add(units);
            m.Items.Add(new ToolStripMenuItem("Other…", null, (s, e) => { var t = InputDialog.Ask(this, "Convert", "Convert to (unit or format):", ""); if (!string.IsNullOrWhiteSpace(t)) convert(t.Trim()); }));
            return m;
        }

        public void NewFunction(string formula = "")
        {
            using (var d = new FunctionEditDialog(Calc, null, formula)) if (d.ShowDialog(this) == DialogResult.OK) DefinitionsChanged();
        }

        public void NewVariable()
        {
            var init = CalcView.Last?.Value != null ? CalcView.Last.Ascii : "";
            using (var d = new VariableEditDialog(Calc, null, init)) if (d.ShowDialog(this) == DialogResult.OK) DefinitionsChanged();
        }

        public void NewUnknown()
        {
            using (var d = new UnknownEditDialog(Calc, null)) if (d.ShowDialog(this) == DialogResult.OK) DefinitionsChanged();
        }

        public void NewUnit()
        {
            using (var d = new UnitEditDialog(Calc, null)) if (d.ShowDialog(this) == DialogResult.OK) DefinitionsChanged();
        }

        public void OpenPlot(string expr, double a, double b)
        {
            var p = Page<PlotPage>(6);
            p.AddSeries(expr, a, b);
        }

        public static void OpenUrl(string url)
        {
            try { Process.Start(new ProcessStartInfo(url) { UseShellExecute = true }); } catch (Exception) { }
        }

        // workspaces: definitions, history, modes and stack in one file
        void OpenWorkspaceDialog()
        {
            using (var d = new OpenFileDialog { Filter = "OrangCalc workspace (*.ocws)|*.ocws|All files|*.*" })
                if (d.ShowDialog(this) == DialogResult.OK) OpenWorkspace(d.FileName);
        }

        void SaveWorkspaceAs()
        {
            using (var d = new SaveFileDialog { Filter = "OrangCalc workspace (*.ocws)|*.ocws", DefaultExt = "ocws" })
                if (d.ShowDialog(this) == DialogResult.OK) SaveWorkspace(d.FileName);
        }

        public void SaveWorkspace(string path)
        {
            try
            {
                var doc = new XmlDocument();
                var root = doc.CreateElement("workspace");
                doc.AppendChild(root);
                var opts = doc.CreateElement("options"); root.AppendChild(opts);
                Storage.WriteSettings(doc, opts, null, Calc.Opts);
                var defs = doc.CreateElement("definitions"); root.AppendChild(defs);
                UserItems.Save(Calc, doc, defs);
                var hist = doc.CreateElement("history"); root.AppendChild(hist);
                Storage.WriteHistory(doc, hist, CalcView.History.Entries);
                var st = doc.CreateElement("stack"); root.AppendChild(st);
                foreach (var e in CalcView.Stack) { var it = doc.CreateElement("value"); it.InnerText = Formatter.Ascii(e, Calc.Opts); st.AppendChild(it); }
                doc.Save(path);
                Settings.Workspace = path;
                AddRecent(Settings.RecentWorkspaces, path);
                UpdateTitle(lastResultTitle);
                MarkDirty();
            }
            catch (Exception ex) { Ui.Message(this, "Couldn't save workspace:\n" + ex.Message, "Error"); }
        }

        public void OpenWorkspace(string path)
        {
            try
            {
                var doc = new XmlDocument();
                doc.Load(path);
                var root = doc.DocumentElement;
                if (root.SelectSingleNode("options") is XmlElement opts) Storage.ReadSettings(opts, null, Calc.Opts);
                UserItems.ClearUser(Calc);
                if (root.SelectSingleNode("definitions") is XmlElement defs) UserItems.Load(Calc, defs);
                CalcView.History.Entries.Clear();
                if (root.SelectSingleNode("history") is XmlElement hist) CalcView.History.Entries.AddRange(Storage.ReadHistory(hist));
                CalcView.History.Refill();
                CalcView.Stack.Clear();
                if (root.SelectSingleNode("stack") is XmlElement st)
                    foreach (XmlElement v in st.SelectNodes("value"))
                    {
                        try { CalcView.Stack.Add(Calc.EvalText(v.InnerText)); } catch (CalcException) { }
                    }
                CalcView.Rpn.Refill();
                Settings.Workspace = path;
                AddRecent(Settings.RecentWorkspaces, path);
                OptionsChanged();
                DefinitionsChanged();
                UpdateTitle("");
            }
            catch (Exception ex) { Ui.Message(this, "Failed to open workspace:\n" + ex.Message, "Error"); }
        }

        void OpenDefaultWorkspace()
        {
            Settings.Workspace = "";
            UserItems.ClearUser(Calc);
            Storage.LoadDefinitions(Calc);
            CalcView.History.Entries.Clear();
            CalcView.History.Entries.AddRange(Storage.LoadHistory());
            CalcView.History.Refill();
            DefinitionsChanged();
            UpdateTitle("");
        }

        // exchange rates from the European Central Bank
        void LoadRates()
        {
            try
            {
                if (!File.Exists(Storage.RatesFile)) return;
                var doc = new XmlDocument();
                doc.Load(Storage.RatesFile);
                Units.SetRates(Calc, ParseRates(doc));
            }
            catch (Exception) { }
        }

        static Dictionary<string, double> ParseRates(XmlDocument doc)
        {
            var rates = new Dictionary<string, double>();
            foreach (XmlNode n in doc.GetElementsByTagName("Cube"))
            {
                var e = n as XmlElement;
                if (e == null || !e.HasAttribute("currency")) continue;
                if (double.TryParse(e.GetAttribute("rate"), NumberStyles.Float, CultureInfo.InvariantCulture, out var r)) rates[e.GetAttribute("currency")] = r;
            }
            return rates;
        }

        void MaybeUpdateRates()
        {
            if (Settings.RatesUpdateDays <= 0) return;
            if (DateTime.TryParse(Settings.LastRatesUpdate, CultureInfo.InvariantCulture, DateTimeStyles.None, out var last) && (DateTime.Now - last).TotalDays < Settings.RatesUpdateDays) return;
            UpdateRates(false);
        }

        public void UpdateRates(bool interactive)
        {
            Task.Run(() =>
            {
                ServicePointManager.SecurityProtocol |= (SecurityProtocolType)3072;
                using (var wc = new WebClient())
                {
                    wc.Headers[HttpRequestHeader.UserAgent] = "OrangCalc";
                    var xml = wc.DownloadString("https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml");
                    var doc = new XmlDocument();
                    doc.LoadXml(xml);
                    var rates = ParseRates(doc);
                    try
                    {
                        var btc = wc.DownloadString("https://api.coinbase.com/v2/prices/BTC-EUR/spot");
                        var m = System.Text.RegularExpressions.Regex.Match(btc, "\"amount\"\\s*:\\s*\"([0-9.]+)\"");
                        if (m.Success && double.TryParse(m.Groups[1].Value, NumberStyles.Float, CultureInfo.InvariantCulture, out var p) && p > 0)
                        {
                            var cube = doc.CreateElement("Cube");
                            cube.SetAttribute("currency", "BTC");
                            cube.SetAttribute("rate", (1 / p).ToString("R", CultureInfo.InvariantCulture));
                            doc.DocumentElement.AppendChild(cube);
                            rates["BTC"] = 1 / p;
                        }
                    }
                    catch (Exception) { }
                    doc.Save(Storage.RatesFile);
                    return rates;
                }
            }).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                if (t.IsFaulted)
                {
                    if (interactive) Ui.Message(this, "Failed to download exchange rates:\n" + (t.Exception?.InnerException?.Message ?? ""), "Error");
                    return;
                }
                Svc.Run(() => { Units.SetRates(Calc, t.Result); return true; });
                Settings.LastRatesUpdate = DateTime.Now.ToString("s", CultureInfo.InvariantCulture);
                MarkDirty();
                if (interactive) Ui.Message(this, "Exchange rates were updated (" + t.Result.Count + " currencies).");
            })));
        }
    }

    public interface IPage
    {
        void Activated();
        void DefinitionsChanged();
        void OptionsChanged();
    }
}