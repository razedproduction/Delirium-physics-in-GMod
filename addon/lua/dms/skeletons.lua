-- Skeletons beyond ValveBiped (ROADMAP M12, docs/adr/0030): which bone of a model is each of NM's joints, and the
-- player's edits of the fit. The module knows the common rigs by their bones' names (ValveBiped, Biped, Mixamo, Unreal,
-- TF2, VRoid, MMD) and guesses the rest by the names' words and the hierarchy (DMS.ModelClassify). In the DMS window's
-- Skeletons tab a model's rig is made by hand: a bone per joint, the joints moved, the parts' sizes, masses and how
-- they lie on the model; kept in data/dms/skeletons/<model>.json (exported and imported as text, so a rig made on one
-- machine works on another), tried on the ragdolls in the world before it's saved. Models are read where the game has
-- them (mounted Workshop GMAs included): nothing of them is copied or written anywhere.
--
-- A rig: {bones = {[part] = bone}, joints = {[part] = {x, y, z}} (units, the model's space), parts = {[part] = {radius,
-- half, box = {x, y, z} (m), mass (kg), offset = {x, y, z} (m, the part's frame), turn = {p, y, r} (degrees)}}}.

local DIR = "dms/skeletons"
local FORMAT = 2

file.CreateDir(DIR)

-- A model's rig file (data/dms/skeletons/<model without models/ and .mdl>.json).
local function path(model)
	local name = model:lower():gsub("\\", "/"):gsub("^models/", ""):gsub("%.mdl$", ""):gsub("[^%w_%-/%.]", "_")
	return DIR .. "/" .. name .. ".json"
end

local saved = {} -- model -> its saved rig, or false
local drafts = {} -- model -> a rig being tried (the editor's "Try it"), till it's saved or dropped
local classes = {} -- model -> its class (DMS.ModelClassify), or {error}

-- A model's .mdl as the game has it (its own files, a mounted GMA's): bytes, or nil and why.
function DMS_ModelBytes(model)
	if not isstring(model) or not model:lower():match("%.mdl$") then return nil, tostring(model) .. ": not a model" end
	local bytes = file.Read(model, "GAME")
	if not bytes then return nil, model .. ": not found (not mounted?)" end
	return bytes
end

local function vec(v)
	if not istable(v) then return nil end
	local x, y, z = tonumber(v[1] or v.x), tonumber(v[2] or v.y), tonumber(v[3] or v.z)
	if not (x and y and z) or x ~= x or y ~= y or z ~= z then return nil end
	return { x, y, z }
end

local function zero(v) return not v or (v[1] == 0 and v[2] == 0 and v[3] == 0) end

-- A rig as it may come (a file, the client, pasted text) made safe: names as strings, numbers as numbers, nothing
-- that doesn't change the fit.
function DMS_RigClean(t)
	t = istable(t) and t or {}
	local rig = { bones = {}, joints = {}, parts = {} }
	for part, bone in pairs(istable(t.bones) and t.bones or {}) do
		if isstring(part) and isstring(bone) and bone ~= "" then rig.bones[part] = bone end
	end
	for part, v in pairs(istable(t.joints) and t.joints or {}) do
		local j = vec(v)
		if isstring(part) and not zero(j) then rig.joints[part] = j end
	end
	for part, p in pairs(istable(t.parts) and t.parts or {}) do
		if isstring(part) and istable(p) then
			local e = {
				radius = tonumber(p.radius), half = tonumber(p.half), box = vec(p.box), mass = tonumber(p.mass),
				offset = vec(p.offset), turn = vec(p.turn),
			}
			if zero(e.offset) then e.offset = nil end
			if zero(e.turn) then e.turn = nil end
			if next(e) then rig.parts[part] = e end
		end
	end
	return rig
end

-- Whether a rig changes anything beyond its bones.
local function edited(rig) return rig and (next(rig.joints) ~= nil or next(rig.parts) ~= nil) end

-- A rig as text (the export and the file): JSON with the model it was made for.
local function encode(model, rig, checksum)
	return util.TableToJSON({ dms_skeleton = FORMAT, model = model, checksum = checksum, bones = rig.bones,
		joints = next(rig.joints) and rig.joints or nil, parts = next(rig.parts) and rig.parts or nil }, true)
end

-- A rig from text (an import or the file; format 1 had bones only): the rig and the model it was made for, or nil
-- and why.
function DMS_SkeletonDecode(text)
	local t = isstring(text) and util.JSONToTable(text)
	if not istable(t) or not istable(t.bones) then return nil, "not a DMS skeleton mapping (no bones)" end
	if (tonumber(t.dms_skeleton) or 0) > FORMAT then return nil, "a mapping from a newer DMS (format " .. tostring(t.dms_skeleton) .. ")" end
	return DMS_RigClean(t), t.model
end

-- The player's saved rig of a model, or nil.
function DMS_SkeletonSaved(model)
	if saved[model] == nil then
		local text = file.Read(path(model), "DATA")
		saved[model] = text and DMS_SkeletonDecode(text) or false
	end
	return saved[model] or nil
end

-- The rig NM uses for a model: the one being tried, else the saved one, else none (the model's own).
function DMS_ModelRig(model)
	return drafts[model] or DMS_SkeletonSaved(model)
end

-- The module's fit arguments of a rig: its bones (nil for the model's own) and its edits (nil for none).
function DMS_RigArgs(rig)
	if not rig then return nil, nil end
	return next(rig.bones) and rig.bones or nil, edited(rig) and rig or nil
end

local function forget(model)
	saved[model], classes[model] = nil, nil
	if DMS then DMS.RagdollForget(model) end
	if DMS_ModelChanged then DMS_ModelChanged(model) end
end

-- What a model's skeleton is (DMS.ModelClassify with its rig's bones): the class, or nil and why. Kept a session.
function DMS_ModelClass(model)
	local c = classes[model]
	if not c then
		local bytes, why = DMS_ModelBytes(model)
		if bytes then
			local ok, r = pcall(DMS.ModelClassify, bytes, (DMS_RigArgs(DMS_ModelRig(model))))
			c = ok and r or { error = tostring(r) }
		else
			c = { error = why }
		end
		classes[model] = c
	end
	if c.error then return nil, c.error end
	return c
end

-- Whether NM can drive the model: a humanoid whose joints are all known (by its rig or a saved one).
function DMS_ModelMapped(model)
	if not isstring(model) or model == "" or model == DMS_HOMER_MODEL then return false end
	local c = DMS_ModelClass(model)
	return c ~= nil and c.complete
end

-- Saves a rig for a model (checked by fitting it first): true, or nil and why. The draft goes.
function DMS_SkeletonSave(model, rig)
	local bytes, why = DMS_ModelBytes(model)
	if not bytes then return nil, why end
	rig = DMS_RigClean(rig)
	local bones, edits = DMS_RigArgs(rig)
	local ok, f = pcall(DMS.FitPreview, model, bytes, bones, edits)
	if not ok then return nil, tostring(f) end
	local s = DMS.ModelSkeleton(bytes)
	file.CreateDir(string.GetPathFromFilename(path(model)))
	file.Write(path(model), encode(model, rig, s.checksum))
	drafts[model] = nil
	forget(model)
	return true
end

function DMS_SkeletonDelete(model)
	file.Delete(path(model))
	drafts[model] = nil
	forget(model)
end

-- NM again on a model's ragdolls with NM, with the rig it has now (a draft tried, a rig saved): each given its own
-- physics and NM anew, balancing. How many.
local function refit(model)
	local n = 0
	for _, ent in pairs(table.Copy(DMS_Agents)) do
		if IsValid(ent) and ent.DMS_Native and ent:GetModel() == model and not ent.DMS_Death and not ent.DMS_LiveOf then
			local new = DMS_DetachRagdoll(ent)
			if IsValid(new) and DMS_AttachRagdoll(new) then
				DMS_Send(new, "bodyBalance start=true")
				n = n + 1
			end
		end
	end
	return n
end

-- Tries a rig on the model's ragdolls in the world (a new one in front of the player if there's none): NM's body made
-- by it, not saved. true and what was done, or nil and why.
function DMS_SkeletonTry(ply, model, rig)
	local bytes, why = DMS_ModelBytes(model)
	if not bytes then return nil, why end
	rig = DMS_RigClean(rig)
	local bones, edits = DMS_RigArgs(rig)
	local ok, err = pcall(DMS.FitPreview, model, bytes, bones, edits)
	if not ok then return nil, tostring(err) end
	drafts[model] = rig
	forget(model)
	DMS_WhenReady(model, ply, function(_, failed)
		if failed then return end
		if refit(model) > 0 or not IsValid(ply) then return end
		local tr = util.TraceLine({ start = ply:EyePos(), endpos = ply:EyePos() + ply:GetAimVector() * 150, filter = ply })
		local ent = DMS_SpawnRagdoll(model, DMS_FloorAt(tr.HitPos, ply), ply:EyeAngles().y + 180)
		if not ent then return end
		undo.Create("DMS ragdoll")
		undo.AddEntity(ent)
		undo.SetPlayer(ply)
		undo.Finish()
		ply:AddCleanup("ragdolls", ent)
		DMS_Send(ent, "bodyBalance start=true")
	end)
	return true
end

-- Drops a model's draft (back to its saved rig or its own) and puts NM on its ragdolls again by it.
local function dropDraft(model)
	if not drafts[model] then return end
	drafts[model] = nil
	forget(model)
	DMS_WhenReady(model, nil, function(_, failed) if not failed then refit(model) end end)
end

-- The mapping editor's requests (the DMS window, tuning.lua's channel; superadmins): `skeleton` the model's bones,
-- class, rig and fit; `skeleton_fit` a rig tried in the window; `skeleton_try` on the ragdolls in the world;
-- `skeleton_save`, `skeleton_delete`, `skeleton_drop` (the draft); `skeleton_import` a rig's text.
local function preview(model, bytes, rig)
	local ok, f = pcall(DMS.FitPreview, model, bytes, DMS_RigArgs(rig))
	if ok then return f end
	return { error = tostring(f) }
end

function DMS_SkeletonRequest(ply, req, reply)
	local model = tostring(req.model or "")
	local bytes, why = DMS_ModelBytes(model)
	if not bytes then return reply(ply, "skeleton", { model = model, error = why }) end
	if not DMS.DataInfo() and not pcall(DMS.LoadData, DMS_GameDir()) then
		return reply(ply, "skeleton", { model = model, error = "NM's data isn't loaded" })
	end
	local k = req.kind
	local s = DMS.ModelSkeleton(bytes)
	if k == "skeleton_fit" or k == "skeleton_try" then
		local rig = DMS_RigClean(req.rig)
		local note
		if k == "skeleton_try" then
			local ok, err = DMS_SkeletonTry(ply, model, rig)
			note = ok and "trying it on the ragdolls in the world (not saved)" or ("can't try it: " .. tostring(err))
		end
		return reply(ply, "skeleton_fit", { model = model, preview = preview(model, bytes, rig), text = encode(model, rig, s.checksum),
			note = note, draft = drafts[model] ~= nil })
	end
	local note
	if k == "skeleton_delete" then
		DMS_SkeletonDelete(model)
		note = "the saved rig deleted: the model's own (if any) is used"
		dropDraft(model)
	elseif k == "skeleton_save" then
		local ok, err = DMS_SkeletonSave(model, req.rig)
		note = ok and ("saved in data/" .. path(model)) or ("not saved: " .. tostring(err))
		if ok then DMS_WhenReady(model, nil, function(_, failed) if not failed then refit(model) end end) end
	elseif k == "skeleton_drop" then
		dropDraft(model)
		note = "the rig tried is dropped"
	end
	local okc, class = pcall(DMS.ModelClassify, bytes)
	local rig = not req.own and DMS_ModelRig(model) or nil
	local imported
	if k == "skeleton_import" then
		local r, from = DMS_SkeletonDecode(req.text)
		if not r then return reply(ply, "skeleton", { model = model, error = "import: " .. tostring(from) }) end
		imported, rig = from or "?", r
	end
	local bones = {}
	for i, b in ipairs(s.bones) do
		bones[i] = { name = b.name, parent = b.parent, pos = b.mesh_pos, rot = b.mesh_rot, physics = b.physics }
	end
	rig = rig and table.Copy(rig) or { bones = {}, joints = {}, parts = {} }
	if not next(rig.bones) and okc then rig.bones = table.Copy(class.map) end
	return reply(ply, "skeleton", {
		model = model, bones = bones, class = okc and class or { error = tostring(class) }, saved = DMS_SkeletonSaved(model) ~= nil,
		draft = drafts[model] ~= nil, rig = rig, imported = imported, note = note, preview = preview(model, bytes, rig),
		text = encode(model, rig, s.checksum),
	})
end

-- dms_rig <model>: what the model's skeleton is to DMS (its rig, the joints found, what's missing).
concommand.Add("dms_rig", function(ply, _, args)
	if IsValid(ply) and not ply:IsSuperAdmin() then return end
	local model = args[1] or (IsValid(ply) and ply:GetModel()) or ""
	classes[model] = nil
	local c, why = DMS_ModelClass(model)
	if not c then return DMS_Say(ply, "[DMS] dms_rig: " .. tostring(why)) end
	local rig = DMS_ModelRig(model)
	local lines = { string.format("[DMS] %s: %s, %s%s", model, c.rig, c.complete and "NM can drive it" or "can't be fitted",
		drafts[model] and " (a rig being tried)" or rig and (" (a saved rig" .. (edited(rig) and ", edited" or "") .. ")") or "") }
	for _, part in ipairs(c.parts) do
		lines[#lines + 1] = string.format("  %-16s %s", part, c.map[part] or (c.optional[part] and "-" or "MISSING"))
	end
	for _, p in ipairs(c.problems) do lines[#lines + 1] = "  problem: " .. p end
	for _, r in ipairs(c.report) do lines[#lines + 1] = "  reader: " .. r end
	if not c.complete then lines[#lines + 1] = "  " .. c.why end
	DMS_Say(ply, table.concat(lines, "\n"))
end)
