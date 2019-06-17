/*
 * meta_fastdl
 * Copyright (c) 2019 Alik Aslanyan <cplusplus256@gmail.com>
 *
 *
 *    This program is free software; you can redistribute it and/or modify it
 *    under the terms of the GNU General Public License as published by the
 *    Free Software Foundation; either version 3 of the License, or (at
 *    your option) any later version.
 *
 *    This program is distributed in the hope that it will be useful, but
 *    WITHOUT ANY WARRANTY; without even the implied warranty of
 *    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *    General Public License for more details.
 *
 *    You should have received a copy of the GNU General Public License
 *    along with this program; if not, write to the Free Software Foundation,
 *    Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 *    In addition, as a special exception, the author gives permission to
 *    link the code of this program with the Half-Life Game Engine ("HL
 *    Engine") and Modified Game Libraries ("MODs") developed by Valve,
 *    L.L.C ("Valve").  You must obey the GNU General Public License in all
 *    respects for all of the code used other than the HL Engine and MODs
 *    from Valve.  If you modify this file, you may extend this exception
 *    to your version of the file, but you are not obligated to do so.  If
 *    you do not wish to do so, delete this exception statement from your
 *    version.
 *
 */

#include "helper/external_api.h"
#include "ffi.h"

#include <string>

#ifdef _WIN32
extern "C" __declspec(dllexport) void __stdcall GiveFnptrsToDll(enginefuncs_t* pengfuncsFromEngine, globalvars_t *pGlobals);
#elif defined __linux__
extern "C" void GiveFnptrsToDll(enginefuncs_t* pengfuncsFromEngine, globalvars_t *pGlobals);
#endif

// Must provide at least one of these..
static META_FUNCTIONS gMetaFunctionTable = {
	nullptr,			// pfnGetEntityAPI		HL SDK; called before game DLL
	nullptr,			// pfnGetEntityAPI_Post		META; called after game DLL
	nullptr,		// pfnGetEntityAPI2		HL SDK2; called before game DLL
	GetEntityAPI2,			// pfnGetEntityAPI2_Post	META; called after game DLL
	nullptr,			// pfnGetNewDLLFunctions	HL SDK2; called before game DLL
	nullptr,			// pfnGetNewDLLFunctions_Post	META; called after game DLL
	GetEngineFunctions,	// pfnGetEngineFunctions	META; called before HL engine
	nullptr,			// pfnGetEngineFunctions_Post	META; called after HL engine
};

plugin_info_t Plugin_info = {
	META_INTERFACE_VERSION,
	"Meta FastDL",
	"0.1.0",
	"2019/05/18",
	"Alik Aslanyan <cplusplus256@gmail.com>",
	"https://github.com/in-line/meta_fastdl",
	"",
	PT_CHANGELEVEL,
	PT_CHANGELEVEL,
};

meta_globals_t *gpMetaGlobals;
gamedll_funcs_t *gpGamedllFuncs;
mutil_funcs_t *gpMetaUtilFuncs;

C_DLLEXPORT int Meta_Query(const char * /*ifvers */, plugin_info_t **pPlugInfo, mutil_funcs_t *pMetaUtilFuncs)
{
	// Give metamod our plugin_info struct
	*pPlugInfo = &Plugin_info;
	// Get metamod utility function table.
	gpMetaUtilFuncs = pMetaUtilFuncs;
	return(TRUE);
}

C_DLLEXPORT int Meta_Attach(PLUG_LOADTIME /* now */,
                            META_FUNCTIONS *pFunctionTable, meta_globals_t *pMGlobals,
                            gamedll_funcs_t *pGamedllFuncs)
{
	using namespace std::literals;

	if(!pMGlobals) {
		LOG_ERROR(PLID, "[Error] Meta_Attach called with nullptr pMGlobals");
		return(FALSE);
	}
	gpMetaGlobals = pMGlobals;
	if(!pFunctionTable) {
		LOG_ERROR(PLID, "[Error] Meta_Attach called with nullptr pFunctionTable");
		return(FALSE);
	}
	memcpy(pFunctionTable, &gMetaFunctionTable, sizeof(META_FUNCTIONS));
	gpGamedllFuncs = pGamedllFuncs;

	std::string pluginDirPath = GET_PLUGIN_PATH(PLID);
	if(size_t lastSlash = pluginDirPath.find_last_of("/"); lastSlash != std::string::npos )
	{
		pluginDirPath.erase(lastSlash, std::string::npos);
	}

	char gameDir[MAX_PATH] = {};
	GET_GAME_DIR(gameDir);

	char downloadURL[PATH_MAX] = {};
	fastdl_init(pluginDirPath.c_str(), gameDir, downloadURL, PATH_MAX);

	SERVER_COMMAND(("sv_downloadurl \""s + downloadURL + "\"\n").c_str());

	return(TRUE);
}

C_DLLEXPORT int Meta_Detach(PLUG_LOADTIME, PL_UNLOAD_REASON)
{
	fastdl_deinit();
	return(TRUE);
}

int precacheModelHook(const char *s) {
	fastdl_insert_to_whitelist("", s);
	RETURN_META_VALUE(MRES_IGNORED, 0);
}

int precacheSoundHook(const char *s) {
	fastdl_insert_to_whitelist("sound", s);
	RETURN_META_VALUE(MRES_IGNORED, 0);
}

int precacheGenericHook(const char *s) {
	fastdl_insert_to_whitelist("", s);
	RETURN_META_VALUE(MRES_IGNORED, 0);
}


enginefuncs_t meta_engfuncs;
C_DLLEXPORT int GetEngineFunctions(enginefuncs_t *pengfuncsFromEngine,
																	 int *interfaceVersion)
{
	memset(&meta_engfuncs, 0, sizeof(enginefuncs_t));

	meta_engfuncs.pfnPrecacheModel = precacheModelHook;
	meta_engfuncs.pfnPrecacheSound = precacheSoundHook;
	meta_engfuncs.pfnPrecacheGeneric = precacheGenericHook;

	if(!pengfuncsFromEngine) {
		LOG_ERROR(PLID, "[Error] GetEngineFunctions called with nullptr pengfuncsFromEngine");
		return(FALSE);
	}
	else if(*interfaceVersion != ENGINE_INTERFACE_VERSION) {
		LOG_ERROR(PLID, "[Error] GetEngineFunctions version mismatch; requested=%d ours=%d", *interfaceVersion, ENGINE_INTERFACE_VERSION);
		*interfaceVersion = ENGINE_INTERFACE_VERSION;
		return(FALSE);
	}
	memcpy(pengfuncsFromEngine, &meta_engfuncs, sizeof(enginefuncs_t));
	return(TRUE);
}


DLL_FUNCTIONS gFunctionTable;
C_DLLEXPORT int GetEntityAPI2(DLL_FUNCTIONS *pFunctionTable, int* /*interfaceVersion*/)
{
	memset(&gFunctionTable, 0, sizeof(DLL_FUNCTIONS));
	memcpy(pFunctionTable, &gFunctionTable, sizeof(DLL_FUNCTIONS));

	return 1;
}

#include <h_export.h>

// From SDK dlls/h_export.cpp:

//! Holds engine functionality callbacks
enginefuncs_t g_engfuncs;
globalvars_t  *gpGlobals;

// Receive engine function table from engine.
// This appears to be the _first_ DLL routine called by the engine, so we
// do some setup operations here.

C_DLLEXPORT void WINAPI GiveFnptrsToDll( enginefuncs_t* pengfuncsFromEngine, globalvars_t *pGlobals )
{
	memcpy(&g_engfuncs, pengfuncsFromEngine, sizeof(enginefuncs_t));
	gpGlobals = pGlobals;
}
