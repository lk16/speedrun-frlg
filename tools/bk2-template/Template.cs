// Writes and reads route/template.bk2 using BizHawk's own movie code.
//
// The point of the template is to settle two things that are not derivable from any file
// BizHawk ships as data: the Input Log column order (LogKey) and the SyncSettings blob the
// mGBA core expects. Both live in compiled CIL. Rather than recording a movie by hand in the
// GUI and hoping it was recorded with stock settings, this loads the shipped assemblies under
// mono and asks them:
//
//   LogKey        Bk2LogEntryGenerator.GenerateLogKey(MGBAHawk.GBAController)
//   empty frame   Bk2LogEntryGenerator.EmptyEntry(new Bk2Controller(GBAController))
//   SyncSettings  ConfigService.SaveWithType(new MGBAHawk.SyncSettings())   -- stock defaults
//   the container Bk2Movie.Write(), which produces the lumps and the zip itself
//
// so the answer is BizHawk's, byte for byte, and it can be regenerated when BizHawk moves.
//
//   mono template.exe write <bizhawk-dll-dir> <out.bk2> <rom.gba>
//   mono template.exe read  <bizhawk-dll-dir> <in.bk2>
//
// Run it through tools/bk2-template.sh, which knows where those live.

using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

// Bk2Movie.Write reaches through Session.Settings for the zip compression level. That single
// property is the whole of what the write path touches, so a proxy that answers it with a stock
// MovieConfig is enough -- and is much less to get wrong than standing up a real MovieSession,
// which wants an emulator, a rom, and therefore a BIOS (see the note in tools/bk2-template.sh).
public class SessionProxy : DispatchProxy
{
	public object Settings;

	protected override object Invoke(MethodInfo targetMethod, object[] args)
	{
		if (targetMethod.Name == "get_Settings") return Settings;
		if (targetMethod.ReturnType == typeof(bool)) return false;
		return null;
	}
}

public static class Template
{
	private static string _dir;

	private static Assembly Resolve(object sender, ResolveEventArgs e)
	{
		var path = Path.Combine(_dir, new AssemblyName(e.Name).Name + ".dll");
		return File.Exists(path) ? Assembly.LoadFrom(path) : null;
	}

	public static int Main(string[] args)
	{
		if (args.Length < 3)
		{
			Console.Error.WriteLine("usage: template.exe write <dll-dir> <out.bk2> <rom.gba>");
			Console.Error.WriteLine("       template.exe read  <dll-dir> <in.bk2>");
			return 2;
		}

		var mode = args[0];
		_dir = args[1];
		AppDomain.CurrentDomain.AssemblyResolve += Resolve;

		var client = Assembly.LoadFrom(Path.Combine(_dir, "BizHawk.Client.Common.dll"));
		var movieType = client.GetType("BizHawk.Client.Common.Bk2Movie");

		// One proxy serves both directions: Load() reads Session.Settings too.
		var session = typeof(DispatchProxy)
			.GetMethod("Create", BindingFlags.Public | BindingFlags.Static)
			.MakeGenericMethod(client.GetType("BizHawk.Client.Common.IMovieSession"), typeof(SessionProxy))
			.Invoke(null, null);
		((SessionProxy)session).Settings =
			Activator.CreateInstance(client.GetType("BizHawk.Client.Common.MovieConfig"));

		return mode == "write"
			? Write(client, movieType, session, args[2], args[3])
			: Read(client, movieType, session, args[2]);
	}

	private static int Write(Assembly client, Type movieType, object session, string outPath, string romPath)
	{
		var cores = Assembly.LoadFrom(Path.Combine(_dir, "BizHawk.Emulation.Cores.dll"));
		var mgba = cores.GetType("BizHawk.Emulation.Cores.Nintendo.GBA.MGBAHawk");

		// Static, so no core instance and therefore no ROM load and no firmware check.
		var controller = mgba
			.GetField("GBAController", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)
			.GetValue(null);

		var logGen = client.GetType("BizHawk.Client.Common.Bk2LogEntryGenerator");
		var logKey = (string)logGen.GetMethod("GenerateLogKey").Invoke(null, new[] { controller });
		var blankController = Activator.CreateInstance(
			client.GetType("BizHawk.Client.Common.Bk2Controller"), new[] { controller });
		var emptyFrame = (string)logGen.GetMethod("EmptyEntry").Invoke(null, new[] { blankController });

		// Stock defaults, deliberately not edited. A template that quietly differs from what a
		// fresh BizHawk produces is worse than no template: the desync it causes looks like a
		// route bug. RTCUseRealTime is the one that looks alarming and is not -- movie playback
		// forces DeterministicEmulationRequested, which overrides it (MGBAHawk's ctor), and
		// AGB-BPRE has no RTC in the cartridge anyway.
		var syncSettingsJson = (string)client.GetType("BizHawk.Client.Common.ConfigService")
			.GetMethod("SaveWithType")
			.Invoke(null, new[] { Activator.CreateInstance(mgba.GetNestedType("SyncSettings")) });

		string emuVersion;
		var versionInfo = Assembly.LoadFrom(Path.Combine(_dir, "BizHawk.Common.dll"))
			.GetType("BizHawk.Common.VersionInfo");
		var getEmuVersion = versionInfo.GetMethod("GetEmuVersion", BindingFlags.Public | BindingFlags.Static);
		emuVersion = getEmuVersion != null
			? (string)getEmuVersion.Invoke(null, null)
			: "Version " + versionInfo.GetField("MainVersion").GetValue(null);

		string sha1;
		using (var hash = System.Security.Cryptography.SHA1.Create())
			sha1 = BitConverter.ToString(hash.ComputeHash(File.ReadAllBytes(romPath))).Replace("-", "");

		var movie = Activator.CreateInstance(
			movieType,
			BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
			null,
			new[] { session, outPath },
			null);

		Action<string, object> set = (name, value) => movieType
			.GetProperty(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
			.SetValue(movie, value);

		set("SystemID", "GBA");
		set("GameName", Path.GetFileNameWithoutExtension(romPath));
		set("Hash", sha1);
		set("Core", "mGBA");
		set("EmulatorVersion", emuVersion);
		set("Author", "");
		set("Rerecords", (ulong)0);
		set("SyncSettingsJson", syncSettingsJson);
		set("LogKey", logKey);

		Invoke(movieType, movie, "CopyLog", new object[] { new List<string> { emptyFrame } });

		var comments = (IList<string>)movieType
			.GetProperty("Comments", BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
			.GetValue(movie);
		comments.Add("speedrun-frlg tier-2 template: one empty frame, stock SyncSettings.");
		comments.Add("Regenerate with tools/bk2-template.sh -- do not hand-edit.");

		Invoke(movieType, movie, "Write", new object[] { outPath, false });
		Console.WriteLine("wrote " + outPath + " (" + new FileInfo(outPath).Length + " bytes)");
		return 0;
	}

	private static int Read(Assembly client, Type movieType, object session, string inPath)
	{
		var movie = Activator.CreateInstance(
			movieType,
			BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
			null,
			new[] { session, inPath },
			null);

		if (!(bool)Invoke(movieType, movie, "Load", new object[0]))
		{
			Console.Error.WriteLine("BizHawk could not load " + inPath);
			return 1;
		}

		Func<string, object> get = name => movieType
			.GetProperty(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
			.GetValue(movie);

		foreach (var kv in (IDictionary<string, string>)get("HeaderEntries"))
			Console.WriteLine("header  " + kv.Key + " = " + kv.Value);
		Console.WriteLine("logkey  " + get("LogKey"));
		Console.WriteLine("frames  " + get("FrameCount"));
		foreach (var line in (IEnumerable)get("Log")) Console.WriteLine("input   " + line);
		Console.WriteLine("sync    " + get("SyncSettingsJson"));
		return 0;
	}

	private static object Invoke(Type type, object target, string name, object[] args) => type
		.GetMethod(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
		.Invoke(target, args);
}
