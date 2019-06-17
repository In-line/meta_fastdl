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

use crate::file_server::FileServer;

use libc::{c_char, c_uint};

use std::ffi::CStr;
use std::path::Path;

use hyper::Uri;
use trust_dns_resolver::config::*;
use trust_dns_resolver::Resolver;

use serde_hjson::{Map, Value};

use std::net::{IpAddr, SocketAddr};
use std::{fs::File, io::BufReader};

use ref_thread_local::refmanager;

mod implementation {
    use super::*;
    use ref_thread_local::RefThreadLocal;
    use refmanager::*;

    ref_thread_local! {
        static managed MODULE: Option<Module> = None;
    }

    pub struct Module {
        pub url: String,
        pub server: FileServer,
    }

    impl Module {
        pub fn get<'a>() -> Ref<'a, Option<Module>> {
            MODULE.borrow()
        }

        pub fn get_mut<'a>() -> RefMut<'a, Option<Module>> {
            MODULE.borrow_mut()
        }
    }
}

use implementation::*;

#[no_mangle]
pub unsafe extern "C" fn fastdl_init(
    config_file_dir: *const c_char,
    gamedir: *const c_char,
    out_url: *mut c_char,
    out_size: c_uint,
) {
    if Module::get().is_some() {
        return;
    }

    let path = Path::new(
        CStr::from_ptr(config_file_dir)
            .to_str()
            .expect("Can't create string from config_file_path raw pointer."),
    )
    .join("config.hjson");

    let mut json: Map<String, Value> = serde_hjson::from_reader(BufReader::new(
        File::open(path).expect("Can't open config file."),
    ))
    .expect("Can't parse config file as HJSON");

    let url = json
        .remove("bind_url")
        .expect("There is no `bind_url` in config")
        .to_string()
        .parse::<Uri>()
        .expect("Can't parse `bind_url` value as URI");

    if !url.path().is_empty() && url.path() != "/" {
        panic!("URL: `{}` contains ignored path: `{}`", url, url.path());
    }

    let host = url
        .host()
        .expect("There is no host part in `bind_url` as http://HOST.COM/PATH");

    let bind_addr = {
        let ip = host.parse::<IpAddr>().unwrap_or_else(|_| {
            let resolver = Resolver::new(ResolverConfig::default(), {
                let mut opts = ResolverOpts::default();

                opts.attempts = 10;
                opts.timeout = std::time::Duration::from_secs(3);

                opts
            })
            .unwrap();
            let response = resolver.lookup_ip(host).unwrap();
            response
                .iter()
                .find(|i| i.is_ipv4() && !i.is_loopback())
                .expect("No valid addresses found for `bind_url`")
        });

        let port = url.port_u16().unwrap_or(80);

        SocketAddr::new(ip, port)
    };

    println!("meta_fastdl bound to address: {}", bind_addr);

    let path = std::env::current_dir()
        .expect("Can't get working directory")
        .join(
            CStr::from_ptr(gamedir)
                .to_str()
                .expect("Can't create string from root raw pointer."),
        );

    Module::get_mut().replace(Module {
        url: host.to_owned(),
        server: FileServer::new(
            bind_addr,
            path.clone(),
            match json
                .remove("prefix_whitelist")
                .expect("Can't find `prefix_whitelist` in config file")
            {
                Value::Array(v) => v.into_iter().map(|i| match i {
                    Value::String(i) => path.join(i),
                    v => panic!("Value contained inside whitelist is not string: {:#?}", v),
                }),
                _ => panic!("`prefix_whitelist` is not array of paths"),
            },
        ),
    });

    libc::strncpy(
        out_url,
        std::ffi::CString::new(url.to_string())
            .expect("URL can't contain internal nul byte")
            .as_ptr(),
        (out_size - 1) as usize,
    );
    *out_url.offset((out_size - 1) as isize) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn fastdl_insert_to_whitelist(prefix: *const c_char, path: *const c_char) {
    Module::get_mut()
        .as_mut()
        .unwrap()
        .server
        .insert_to_file_whitelist(
            Path::new(
                CStr::from_ptr(prefix)
                    .to_str()
                    .expect("Can't create string from prefix raw pointer."),
            )
            .join(Path::new(
                CStr::from_ptr(path)
                    .to_str()
                    .expect("Can't create string from path raw pointer."),
            )),
        );
}

#[no_mangle]
pub unsafe extern "C" fn fastdl_deinit() {
    Module::get_mut().take();
}
