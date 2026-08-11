-- The BizHawk side of tools/verify-runner.sh. Plays the movie EmuHawk was started with, and at
-- the end dumps EWRAM followed by IWRAM so the host can sha1 it and compare against the tier-1
-- fingerprint (`frlg run --ram-hash`, docs/harness.md: sha1 over EWRAM 0x02000000 256K, then
-- IWRAM 0x03000000 32K). Same bytes in the same order, so the two tiers produce one number that
-- can be compared rather than two that cannot.
--
-- When the request ships a trace (FRLG_VERIFY_TRACE, written by `frlg route export`), every
-- frame is also compared against tier 1's per-frame probe -- gRngValue, which the game advances
-- once per VBlank -- and the first mismatching frame is reported as desync_frame=. That is the
-- difference between "it desynced" and "it desynced at frame N".
--
-- Everything is driven from event.onframeend rather than emu.frameadvance(): a script that
-- advances frames itself blocks forever when the emulator is paused, which is how EmuHawk sits
-- when a movie fails to attach.
--
-- NOT YET EXERCISED end to end. The domain names and the read API below are the parts most
-- likely to need a one-line fix on the first real run -- fix them here (or in the artifacts-side
-- override copy, which wins), do not work around them in the shell.

local dump_path   = assert(os.getenv("FRLG_VERIFY_DUMP"), "FRLG_VERIFY_DUMP is unset")
local status_path = assert(os.getenv("FRLG_VERIFY_STATUS"), "FRLG_VERIFY_STATUS is unset")
local want_frames = tonumber(os.getenv("FRLG_VERIFY_FRAMES") or "0") or 0

-- The per-frame probe trace, if the request shipped one: little-endian u32 per frame.
local trace_domain = os.getenv("FRLG_VERIFY_TRACE_DOMAIN") or "IWRAM"
local trace_offset = tonumber(os.getenv("FRLG_VERIFY_TRACE_OFFSET") or "")
local trace, trace_frames = nil, 0
do
  local trace_path = os.getenv("FRLG_VERIFY_TRACE")
  if trace_path and trace_offset then
    local f = io.open(trace_path, "rb")
    if f then
      trace = f:read("*a")
      f:close()
      trace_frames = math.floor(#trace / 4)
    end
  end
end
local desync_frame, desync_got, desync_want = nil, nil, nil

local function status(played, frames, note)
  local f = assert(io.open(status_path, "w"))
  f:write("played=", played, "\n")
  f:write("frames=", tostring(frames), "\n")
  f:write("mode=", tostring(movie.mode()), "\n")
  f:write("trace_frames=", tostring(trace_frames), "\n")
  if desync_frame then
    f:write("desync_frame=", tostring(desync_frame), "\n")
    f:write(string.format("desync_got=%08x\n", desync_got))
    f:write(string.format("desync_want=%08x\n", desync_want))
  end
  if note then f:write("note=", note, "\n") end
  f:close()
end

-- After a frame completes, emu.framecount() has already been incremented
-- (MGBAHawk.IEmulator.cs:83 does Frame++ inside FrameAdvance), so the frame
-- that just ran is framecount-1 -- the same 0-based index tier 1's trace uses.
local function trace_check()
  if not trace or desync_frame then return end
  local idx = emu.framecount() - 1
  if idx < 0 or idx >= trace_frames then return end
  local got = memory.read_u32_le(trace_offset, trace_domain)
  local base = idx * 4
  local want = string.byte(trace, base + 1)
      + string.byte(trace, base + 2) * 0x100
      + string.byte(trace, base + 3) * 0x10000
      + string.byte(trace, base + 4) * 0x1000000
  if got ~= want then
    desync_frame, desync_got, desync_want = idx, got, want
  end
end

-- 256K + 32K in one string.char() call would blow the argument limit, so it goes out in chunks.
local function dump_domain(out, domain, base, length)
  local step = 4096
  for offset = 0, length - 1, step do
    local n = math.min(step, length - offset)
    local bytes = memory.readbyterange(base + offset, n, domain)
    local chunk = {}
    for i = 0, n - 1 do chunk[i + 1] = string.char(bytes[i] % 256) end
    out:write(table.concat(chunk))
  end
end

local function finish()
  local frames = emu.framecount()
  local ok, err = pcall(function()
    local out = assert(io.open(dump_path, "wb"))
    dump_domain(out, "EWRAM", 0, 256 * 1024)
    dump_domain(out, "IWRAM", 0, 32 * 1024)
    out:close()
  end)
  status(ok and "yes" or "no", frames, ok and nil or tostring(err))
  client.exit()
end

event.onframeend(function()
  local ok, err = pcall(trace_check)
  if not ok and not desync_frame then
    -- A broken read API must not kill the replay; report it instead.
    desync_frame, desync_got, desync_want = -1, 0, 0
    trace = nil
  end
  local mode = movie.mode()
  -- FINISHED is what a .bk2 that played all the way through reports; PLAY means still going.
  if mode == "FINISHED" then return finish() end
  if want_frames > 0 and emu.framecount() >= want_frames then return finish() end
  -- Neither playing nor finished means the movie never attached -- report rather than spin.
  if mode ~= "PLAY" then
    status("no", emu.framecount(), "movie mode is " .. tostring(mode) .. ", expected PLAY")
    client.exit()
  end
end)
