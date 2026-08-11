-- The BizHawk side of tools/verify-runner.sh. Plays the movie EmuHawk was started with, and at
-- the end dumps EWRAM followed by IWRAM so the host can sha1 it and compare against the tier-1
-- fingerprint (`frlg run --ram-hash`, docs/harness.md: sha1 over EWRAM 0x02000000 256K, then
-- IWRAM 0x03000000 32K). Same bytes in the same order, so the two tiers produce one number that
-- can be compared rather than two that cannot.
--
-- Everything is driven from event.onframeend rather than emu.frameadvance(): a script that
-- advances frames itself blocks forever when the emulator is paused, which is how EmuHawk sits
-- when a movie fails to attach.
--
-- NOT YET EXERCISED. Tier 2 has never run: it needs a GBA BIOS the host does not have (see the
-- preflight in verify-runner.sh). The domain names and the read API below are the parts most
-- likely to need a one-line fix on the first real run -- fix them here, do not work around them
-- in the shell.

local dump_path   = assert(os.getenv("FRLG_VERIFY_DUMP"), "FRLG_VERIFY_DUMP is unset")
local status_path = assert(os.getenv("FRLG_VERIFY_STATUS"), "FRLG_VERIFY_STATUS is unset")
local want_frames = tonumber(os.getenv("FRLG_VERIFY_FRAMES") or "0") or 0

local function status(played, frames, note)
  local f = assert(io.open(status_path, "w"))
  f:write("played=", played, "\n")
  f:write("frames=", tostring(frames), "\n")
  f:write("mode=", tostring(movie.mode()), "\n")
  if note then f:write("note=", note, "\n") end
  f:close()
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
