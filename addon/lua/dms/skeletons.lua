-- Skeletons beyond ValveBiped (ROADMAP M12, docs/adr/0030): which bone of a model is each of NM's joints. The module
-- knows the common rigs by their bones' names (ValveBiped, Biped, Mixamo, Unreal, TF2, VRoid, MMD) and guesses the
-- rest by the names' words and the hierarchy (DMS.ModelClassify); a skeleton it can't map is mapped once in the DMS
-- window's Skeletons tab and kept in data/dms/skeletons/<model>.json (exported and imported as text, so a mapping made
-- on one machine works on another). Models are read where the game has them (mounted Workshop GMAs included):
-- nothing of them is copied or written anywhere.

local DIR = "dms/skeletons"
local FORMAT = 1

file.CreateDir(DIR)

-- A model's mapping file (data/dms/skeletons/<model without models/ and .mdl>.json).
local function path(model)
	local name = model:lower():gsub("\\", "/"):gsub("^models/", ""):gsub("%.mdl$", ""):gsub("[^%w_%-/%.]", "_")
	return DIR .. "/" .. name .. ".json"
end

local saved = {} -- model -> its mapping {[part] = bone}, or false
local classes = {} -- model -> its class (DMS.ModelClassify), or {error}

-- A model's .mdl as the game has it (its own files, a mounted GMA's): bytes, or nil and why.
function DMS_ModelBytes(model)
	if not isstring(model) or not model:lower():match("%.mdl$") then return nil, tostring(model) .. ": not a model" end
	local bytes = file.Read(model, "GAME")
	if not bytes then return nil, model .. ": not found (not mounted?)" end
	return bytes
end

-- A mapping as text (the export): JSON with the model it was made for.
local function encode(model, map, checksum)
	return util.TableToJSON({ dms_skeleton = FORMAT, model = model, checksum = checksum, bones = map }, true)
end

-- A mapping from text (an import or the file): {[part] = bone} and the model it was made for, or nil and why.
function DMS_SkeletonDecode(text)
	local t = isstring(text) and util.JSONToTable(text)
	if not istable(t) or not istable(t.bones) then return nil, "not a DMS skeleton mapping (no bones)" end
	if (tonumber(t.dms_skeleton) or 0) > FORMAT then return nil, "a mapping from a newer DMS (format " .. tostring(t.dms_skeleton) .. ")" end
	local map = {}
	for part, bone in pairs(t.bones) do
		if isstring(part) and isstring(bone) and bone ~= "" then map[part] = bone end
	end
	return map, t.model
end

-- The player's saved mapping of a model, or nil.
function DMS_SkeletonSaved(model)
	if saved[model] == nil then
		local text = file.Read(path(model), "DATA")
		saved[model] = text and DMS_SkeletonDecode(text) or false
	end
	return saved[model] or nil
end

local function forget(model)
	saved[model], classes[model] = nil, nil
	if DMS then DMS.RagdollForget(model) end
	if DMS_ModelChanged then DMS_ModelChanged(model) end
end

-- What a model's skeleton is (DMS.ModelClassify with its saved mapping): the class, or nil and why. Kept a session.
function DMS_ModelClass(model)
	local c = classes[model]
	if not c then
		local bytes, why = DMS_ModelBytes(model)
		if bytes then
			local ok, r = pcall(DMS.ModelClassify, bytes, DMS_SkeletonSaved(model))
			c = ok and r or { error = tostring(r) }
		else
			c = { error = why }
		end
		classes[model] = c
	end
	if c.error then return nil, c.error end
	return c
end

-- Whether NM can drive the model: a humanoid whose joints are all known (by its rig or a saved mapping).
function DMS_ModelMapped(model)
	if not isstring(model) or model == "" or model == DMS_HOMER_MODEL then return false end
	local c = DMS_ModelClass(model)
	return c ~= nil and c.complete
end

-- Saves a mapping for a model (checked by fitting it first): true, or nil and why.
function DMS_SkeletonSave(model, map)
	local bytes, why = DMS_ModelBytes(model)
	if not bytes then return nil, why end
	local ok, f = pcall(DMS.FitPreview, model, bytes, map)
	if not ok then return nil, tostring(f) end
	local s = DMS.ModelSkeleton(bytes)
	file.CreateDir(string.GetPathFromFilename(path(model)))
	file.Write(path(model), encode(model, map, s.checksum))
	forget(model)
	return true
end

function DMS_SkeletonDelete(model)
	file.Delete(path(model))
	forget(model)
end

-- The mapping editor's requests (the DMS window, tuning.lua's channel; superadmins): `skeleton` the model's bones, class
-- and fit; `skeleton_fit` a mapping tried; `skeleton_save`, `skeleton_delete`; `skeleton_import` a mapping's text.
local function preview(model, bytes, map)
	local ok, f = pcall(DMS.FitPreview, model, bytes, map)
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
	if k == "skeleton" or k == "skeleton_import" or k == "skeleton_delete" or k == "skeleton_save" then
		local note
		if k == "skeleton_delete" then
			DMS_SkeletonDelete(model)
			note = "the saved mapping deleted: the rig's own (if any) is used"
		elseif k == "skeleton_save" then
			local ok, err = DMS_SkeletonSave(model, istable(req.map) and req.map or {})
			note = ok and ("saved in data/" .. path(model)) or ("not saved: " .. tostring(err))
		end
		local s = DMS.ModelSkeleton(bytes)
		local okc, class = pcall(DMS.ModelClassify, bytes)
		local map = not req.own and DMS_SkeletonSaved(model) or nil
		local imported
		if k == "skeleton_import" then
			local m, from = DMS_SkeletonDecode(req.text)
			if not m then return reply(ply, "skeleton", { model = model, error = "import: " .. tostring(from) }) end
			imported, map = from or "?", m
		end
		local bones = {}
		for i, b in ipairs(s.bones) do
			bones[i] = { name = b.name, parent = b.parent, pos = b.mesh_pos, rot = b.mesh_rot, physics = b.physics }
		end
		local use = map or (okc and class.complete and class.map) or (okc and class.map) or {}
		return reply(ply, "skeleton", {
			model = model, bones = bones, class = okc and class or { error = tostring(class) }, saved = DMS_SkeletonSaved(model),
			map = use, imported = imported, note = note, preview = preview(model, bytes, use),
			text = encode(model, use, s.checksum),
		})
	elseif k == "skeleton_fit" then
		local map = istable(req.map) and req.map or {}
		local s = DMS.ModelSkeleton(bytes)
		return reply(ply, "skeleton_fit", { model = model, map = map, preview = preview(model, bytes, map), text = encode(model, map, s.checksum) })
	end
end

-- dms_skeleton <model>: what the model's skeleton is to DMS (its rig, the joints found, what's missing).
concommand.Add("dms_skeleton", function(ply, _, args)
	if IsValid(ply) and not ply:IsSuperAdmin() then return end
	local model = args[1] or (IsValid(ply) and ply:GetModel()) or ""
	classes[model] = nil
	local c, why = DMS_ModelClass(model)
	if not c then return DMS_Say(ply, "[DMS] dms_skeleton: " .. tostring(why)) end
	local lines = { string.format("[DMS] %s: %s, %s%s", model, c.rig, c.complete and "NM can drive it" or "can't be fitted",
		DMS_SkeletonSaved(model) and " (a saved mapping)" or "") }
	for _, part in ipairs(c.parts) do
		lines[#lines + 1] = string.format("  %-16s %s", part, c.map[part] or (c.optional[part] and "-" or "MISSING"))
	end
	for _, p in ipairs(c.problems) do lines[#lines + 1] = "  problem: " .. p end
	for _, r in ipairs(c.report) do lines[#lines + 1] = "  reader: " .. r end
	if not c.complete then lines[#lines + 1] = "  " .. c.why end
	DMS_Say(ply, table.concat(lines, "\n"))
end)
