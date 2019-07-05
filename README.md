# meta_fastdl [![Build Status](https://travis-ci.com/In-line/meta_fastdl.svg?branch=master)](https://travis-ci.com/In-line/meta_fastdl)

This is implementation of high performance HTTP File Server for HLDS. It's main purpose is for use as FastDL server in `sv_downloadurl`. 
It needs minimum configuration and can be used in production game server.

Module should be installed like any other metamod module.

Configuration file should be stored along module shared library: `meta_fastdl_i386.so`. 
It's HJSON configuration with minimal necessary variables.
```
{
	# Bind URL will be automatically written to `sv_downloadurl`. 
	# Host IP from URL will be automatically bound and used to serve files
	# If domain name is used instead than module will attempt to resolve IP.
	bind_url: http://127.0.0.1:8081
  
	# Module will automatically serve precached files, but user precached files can't be hooked.
	# So instead, files from some folders should be allowed.
	# Bellow is prefix whitelist. It's not necessary list of folders, anything will match. 
	prefix_whitelist: [
		models/
		sound/
		sprites/
	]
}
```
