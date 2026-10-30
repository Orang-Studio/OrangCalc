using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Xml;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public enum AfterCalc { Keep, Clear, ReplaceWithResult, ReplaceIfShorter }
    public enum TitleMode { AppName, AppNameAndResult, AppNameAndWorkspace, ResultOnly }
    public enum CompletionMode { Off, Full, Strict, LimitedFull, LimitedStrict }
    public enum KeypadMode { General, Programming, Algebra, Custom, NumberPadOnly, None }
    public enum HistoryExprMode { Entered, Parsed, EnteredAndParsed }

    public sealed class AppSettings
    {
        public int WinX = -1, WinY = -1, WinW = 1180, WinH = 760;
        public bool Maximized;
        public int SideWidth = 340;
        public bool SideVisible = true;
        public bool SideLeft;
        public int SideTab;
        public KeypadMode Keypad = KeypadMode.General;
        public bool NumpadVisible = true;
        public bool KeypadVisible = true;
        public int KeypadHeight = 250;
        public bool AlwaysOnTop;
        public bool MultipleInstances;
        public bool CloseWithEscape;
        public bool ClearHistoryOnExit;
        public int MaxHistory = 300;
        public AfterCalc ExpressionAfter = AfterCalc.Keep;
        public bool AutoCopyResult;
        public bool CopyAsciiDefault;
        public bool CopyAsciiNoUnits;
        public bool ColorizeResult = true;
        public bool ShowStatus = true;
        public bool CalcAsYouType = true;
        public int AsYouTypeDelay = 250;
        public bool AdaptiveDelay = true;
        public bool AutoCalculateSelection;
        public CompletionMode Completion = CompletionMode.Full;
        public int CompletionMinChars = 2;
        public int CompletionDelay = 0;
        public TitleMode Title = TitleMode.AppName;
        public HistoryExprMode HistoryExpression = HistoryExprMode.Entered;
        public bool DisableBlinking;
        public bool RpnKeys = true;
        public bool RpnMode;
        public int RatesUpdateDays = 7;
        public string LastRatesUpdate = "";
        public string DefaultCurrency = "EUR";
        public string AppFont = "";
        public string ExpressionFont = "";
        public string ResultFont = "";
        public string KeypadFont = "";
        public string HistoryFont = "";
        public string StatusFont = "";
        public bool Tooltips = true;
        public bool HideUnitsInConstants;
        public int CurrentPage;
        public bool PercentInNumpad;
        public string LastUnitButton = "m";
        public bool ConfirmQuit;
        public bool AskSaveWorkspace;
        public string AccentColor = "F08A3C";

        public static Color ParseColor(string hex, Color fallback)
        {
            try { return string.IsNullOrEmpty(hex) ? fallback : Color.FromArgb(255, Color.FromArgb(int.Parse(hex.TrimStart('#'), NumberStyles.HexNumber))); }
            catch (Exception) { return fallback; }
        }

        public static string ColorHex(Color c) => c.R.ToString("X2") + c.G.ToString("X2") + c.B.ToString("X2");

        public List<string> FavoriteFunctions = new List<string> { "sqrt", "sin", "ln", "solve", "integrate" };
        public List<string> FavoriteUnits = new List<string> { "m", "kg", "s", "°C" };
        public List<string> FavoriteVariables = new List<string> { "pi", "e", "c" };
        public List<string> RecentFunctions = new List<string>();
        public List<string> RecentUnits = new List<string>();
        public List<string> RecentVariables = new List<string>();
        public List<string> RecentWorkspaces = new List<string>();
        public List<string[]> CustomButtons = new List<string[]>();
        public Dictionary<string, string> Shortcuts = new Dictionary<string, string>();
        public string Workspace = "";

        public static Font ParseFont(string spec, Font fallback)
        {
            if (string.IsNullOrEmpty(spec)) return fallback;
            try
            {
                var parts = spec.Split(';');
                float size = float.Parse(parts[1], CultureInfo.InvariantCulture);
                var style = parts.Length > 2 ? (FontStyle)int.Parse(parts[2]) : FontStyle.Regular;
                return new Font(parts[0], size, style);
            }
            catch (Exception) { return fallback; }
        }

        public static string FontSpec(Font f) => f == null ? "" : f.Name + ";" + f.Size.ToString(CultureInfo.InvariantCulture) + ";" + (int)f.Style;
    }

    public sealed class HistoryEntry
    {
        public string Expression = "";
        public string Parsed = "";
        public string Result = "";
        public string ResultAscii = "";
        public string ExactText = "";
        public List<OutTok> Tokens = new List<OutTok>();
        public bool Approx;
        public bool IsError;
        public bool IsMessage;
        public bool Protected;
        public string Comment = "";
        public DateTime Time = DateTime.Now;
        public Expr Value;
    }

    public static class Storage
    {
        static string dir;

        // %appdata%\OrangStudio\OrangCalc, moved from the old %appdata%\OrangCalc once
        public static string Dir
        {
            get
            {
                if (dir != null) return dir;
                var root = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
                var d = Path.Combine(root, "OrangStudio", "OrangCalc");
                var old = Path.Combine(root, "OrangCalc");
                try
                {
                    if (!Directory.Exists(d) && Directory.Exists(old))
                    {
                        Directory.CreateDirectory(Path.GetDirectoryName(d));
                        try { Directory.Move(old, d); }
                        catch (IOException)
                        {
                            Directory.CreateDirectory(d);
                            foreach (var f in Directory.GetFiles(old)) File.Copy(f, Path.Combine(d, Path.GetFileName(f)), false);
                        }
                    }
                }
                catch (Exception) { }
                Directory.CreateDirectory(d);
                return dir = d;
            }
        }

        public static string SettingsFile => Path.Combine(Dir, "settings.xml");
        public static string HistoryFile => Path.Combine(Dir, "history.xml");
        public static string DefinitionsFile => Path.Combine(Dir, "definitions.xml");
        public static string RatesFile => Path.Combine(Dir, "rates.xml");

        static string ToText(object v)
        {
            switch (v)
            {
                case null: return "";
                case bool b: return b ? "1" : "0";
                case double d: return d.ToString("R", CultureInfo.InvariantCulture);
                case IFormattable f: return f.ToString(null, CultureInfo.InvariantCulture);
                default: return v.ToString();
            }
        }

        static object FromText(Type t, string s)
        {
            if (t == typeof(string)) return s;
            if (t == typeof(bool)) return s == "1" || s.Equals("true", StringComparison.OrdinalIgnoreCase);
            if (t == typeof(int)) return int.Parse(s, CultureInfo.InvariantCulture);
            if (t == typeof(double)) return double.Parse(s, CultureInfo.InvariantCulture);
            if (t.IsEnum) return Enum.Parse(t, s);
            return null;
        }

        public static void WriteFields(XmlDocument doc, XmlElement parent, string tag, object obj)
        {
            foreach (var f in obj.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance))
            {
                var t = f.FieldType;
                if (!(t == typeof(string) || t == typeof(bool) || t == typeof(int) || t == typeof(double) || t.IsEnum)) continue;
                var e = doc.CreateElement(tag);
                e.SetAttribute("name", f.Name);
                e.SetAttribute("value", ToText(f.GetValue(obj)));
                parent.AppendChild(e);
            }
        }

        public static void ReadFields(XmlElement parent, string tag, object obj)
        {
            var type = obj.GetType();
            foreach (XmlElement e in parent.SelectNodes(tag))
            {
                var f = type.GetField(e.GetAttribute("name"), BindingFlags.Public | BindingFlags.Instance);
                if (f == null) continue;
                try
                {
                    var v = FromText(f.FieldType, e.GetAttribute("value"));
                    if (v != null) f.SetValue(obj, v);
                }
                catch (Exception) { }
            }
        }

        static void WriteList(XmlDocument doc, XmlElement parent, string name, IEnumerable<string> items)
        {
            var e = doc.CreateElement("list");
            e.SetAttribute("name", name);
            foreach (var i in items) { var it = doc.CreateElement("item"); it.InnerText = i; e.AppendChild(it); }
            parent.AppendChild(e);
        }

        static List<string> ReadList(XmlElement parent, string name)
        {
            var e = parent.SelectSingleNode("list[@name='" + name + "']") as XmlElement;
            if (e == null) return null;
            return e.SelectNodes("item").Cast<XmlElement>().Select(x => x.InnerText).ToList();
        }

        public static void SaveSettings(AppSettings s, CalcOptions o)
        {
            try
            {
                var doc = new XmlDocument();
                var root = doc.CreateElement("orangcalc");
                doc.AppendChild(root);
                WriteSettings(doc, root, s, o);
                doc.Save(SettingsFile);
            }
            catch (Exception) { }
        }

        public static void WriteSettings(XmlDocument doc, XmlElement root, AppSettings s, CalcOptions o)
        {
            if (s != null)
            {
                WriteFields(doc, root, "ui", s);
                WriteList(doc, root, "FavoriteFunctions", s.FavoriteFunctions);
                WriteList(doc, root, "FavoriteUnits", s.FavoriteUnits);
                WriteList(doc, root, "FavoriteVariables", s.FavoriteVariables);
                WriteList(doc, root, "RecentFunctions", s.RecentFunctions);
                WriteList(doc, root, "RecentUnits", s.RecentUnits);
                WriteList(doc, root, "RecentVariables", s.RecentVariables);
                WriteList(doc, root, "RecentWorkspaces", s.RecentWorkspaces);
                WriteList(doc, root, "CustomButtons", s.CustomButtons.Select(b => string.Join("\u001f", b)));
                WriteList(doc, root, "Shortcuts", s.Shortcuts.Select(kv => kv.Key + "\u001f" + kv.Value));
            }
            if (o != null) WriteFields(doc, root, "calc", o);
        }

        public static void LoadSettings(AppSettings s, CalcOptions o)
        {
            try
            {
                if (!File.Exists(SettingsFile)) return;
                var doc = new XmlDocument();
                doc.Load(SettingsFile);
                ReadSettings(doc.DocumentElement, s, o);
            }
            catch (Exception) { }
        }

        public static void ReadSettings(XmlElement root, AppSettings s, CalcOptions o)
        {
            if (s != null)
            {
                ReadFields(root, "ui", s);
                s.FavoriteFunctions = ReadList(root, "FavoriteFunctions") ?? s.FavoriteFunctions;
                s.FavoriteUnits = ReadList(root, "FavoriteUnits") ?? s.FavoriteUnits;
                s.FavoriteVariables = ReadList(root, "FavoriteVariables") ?? s.FavoriteVariables;
                s.RecentFunctions = ReadList(root, "RecentFunctions") ?? s.RecentFunctions;
                s.RecentUnits = ReadList(root, "RecentUnits") ?? s.RecentUnits;
                s.RecentVariables = ReadList(root, "RecentVariables") ?? s.RecentVariables;
                s.RecentWorkspaces = ReadList(root, "RecentWorkspaces") ?? s.RecentWorkspaces;
                var cb = ReadList(root, "CustomButtons");
                if (cb != null) s.CustomButtons = cb.Select(x => x.Split('\u001f')).Where(a => a.Length >= 2).ToList();
                var sc = ReadList(root, "Shortcuts");
                if (sc != null) s.Shortcuts = sc.Select(x => x.Split('\u001f')).Where(a => a.Length == 2).GroupBy(a => a[0]).ToDictionary(g => g.Key, g => g.First()[1]);
            }
            if (o != null) ReadFields(root, "calc", o);
        }

        public static void WriteHistory(XmlDocument doc, XmlElement parent, IEnumerable<HistoryEntry> entries)
        {
            foreach (var h in entries)
            {
                var e = doc.CreateElement("entry");
                e.SetAttribute("expression", h.Expression);
                e.SetAttribute("parsed", h.Parsed);
                e.SetAttribute("result", h.Result);
                e.SetAttribute("ascii", h.ResultAscii);
                e.SetAttribute("exact", h.ExactText);
                e.SetAttribute("approx", h.Approx ? "1" : "0");
                e.SetAttribute("error", h.IsError ? "1" : "0");
                e.SetAttribute("message", h.IsMessage ? "1" : "0");
                e.SetAttribute("protected", h.Protected ? "1" : "0");
                e.SetAttribute("comment", h.Comment);
                e.SetAttribute("time", h.Time.ToString("o", CultureInfo.InvariantCulture));
                foreach (var t in h.Tokens)
                {
                    var te = doc.CreateElement("t");
                    te.SetAttribute("k", ((int)t.Type).ToString());
                    te.InnerText = t.Text;
                    e.AppendChild(te);
                }
                parent.AppendChild(e);
            }
        }

        public static List<HistoryEntry> ReadHistory(XmlElement parent)
        {
            var list = new List<HistoryEntry>();
            foreach (XmlElement e in parent.SelectNodes("entry"))
            {
                var h = new HistoryEntry
                {
                    Expression = e.GetAttribute("expression"), Parsed = e.GetAttribute("parsed"), Result = e.GetAttribute("result"),
                    ResultAscii = e.GetAttribute("ascii"), ExactText = e.GetAttribute("exact"), Approx = e.GetAttribute("approx") == "1",
                    IsError = e.GetAttribute("error") == "1", IsMessage = e.GetAttribute("message") == "1", Protected = e.GetAttribute("protected") == "1",
                    Comment = e.GetAttribute("comment")
                };
                DateTime.TryParse(e.GetAttribute("time"), CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind, out h.Time);
                foreach (XmlElement te in e.SelectNodes("t"))
                    h.Tokens.Add(new OutTok(te.InnerText, (TokType)int.Parse(te.GetAttribute("k"))));
                list.Add(h);
            }
            return list;
        }

        public static void SaveHistory(IEnumerable<HistoryEntry> entries)
        {
            try
            {
                var doc = new XmlDocument();
                var root = doc.CreateElement("history");
                doc.AppendChild(root);
                WriteHistory(doc, root, entries);
                doc.Save(HistoryFile);
            }
            catch (Exception) { }
        }

        public static List<HistoryEntry> LoadHistory()
        {
            try
            {
                if (!File.Exists(HistoryFile)) return new List<HistoryEntry>();
                var doc = new XmlDocument();
                doc.Load(HistoryFile);
                return ReadHistory(doc.DocumentElement);
            }
            catch (Exception) { return new List<HistoryEntry>(); }
        }

        public static void SaveDefinitions(Calculator c)
        {
            try
            {
                var doc = new XmlDocument();
                var root = doc.CreateElement("definitions");
                doc.AppendChild(root);
                UserItems.Save(c, doc, root);
                doc.Save(DefinitionsFile);
            }
            catch (Exception) { }
        }

        public static List<string> LoadDefinitions(Calculator c)
        {
            try
            {
                if (!File.Exists(DefinitionsFile)) return new List<string>();
                var doc = new XmlDocument();
                doc.Load(DefinitionsFile);
                return UserItems.Load(c, doc.DocumentElement);
            }
            catch (Exception ex) { return new List<string> { ex.Message }; }
        }
    }
}
