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


use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};

use chashmap::CHashMap;
use futures::sync::oneshot;

use std::io;
use std::sync::Arc;
use std::thread;

use filepath::*;

use hyper_staticfile::{ResolveResult, ResponseBuilder};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::fs::File;

use radix_trie::Trie;

pub struct FileServer {
    join_handle: Option<thread::JoinHandle<()>>,
    file_whitelist: Arc<CHashMap<PathBuf, ()>>,
    cancellation_token: Option<oneshot::Sender<()>>,
    root: PathBuf,
}

impl Drop for FileServer {
    fn drop(&mut self) {
        self.cancellation_token.take().map(|i| i.send(()).ok());
        if let Some(i) = self.join_handle.take() {
            i.join().unwrap()
        }
    }
}

impl FileServer {
    fn resolve_file(
        request: Request<Body>,
        (file, metadata): (tokio::fs::File, std::fs::Metadata),
        file_whitelist: &Arc<CHashMap<PathBuf, ()>>,
        dir_whitelist: &Arc<Trie<PathBuf, ()>>,
    ) -> Result<Response<Body>, io::Error> {
        let file = file.into_std();
        file.path()
            .ok()
            .and_then(|path| {
                file_whitelist
                    .get(&path)
                    .map(|_| {})
                    .or_else(|| {
                        dir_whitelist.get_ancestor(&path).and_then(|i| {
                            use radix_trie::TrieCommon;
                            i.key().and_then(|key| {
                                if path.starts_with(key) {
                                    // Just to be sure
                                    Some(())
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .map(|_| {
                        ResponseBuilder::new()
                            .build(
                                &request,
                                ResolveResult::Found(File::from_std(file), metadata),
                            )
                            .unwrap()
                    })
            })
            .ok_or(())
            .or_else(|_| Response::builder().status(403).body("403 Forbidden".into()))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn resolve_request(
        request: Request<Body>,
        root: &Path,
        file_whitelist: Arc<CHashMap<PathBuf, ()>>,
        dir_whitelist: Arc<Trie<PathBuf, ()>>,
    ) -> impl Future<Item = Response<Body>, Error = io::Error> {
        hyper_staticfile::resolve(&root, &request).then(move |result| match result {
            Ok(result) => match result {
                ResolveResult::Found(file, metadata) => FileServer::resolve_file(
                    request,
                    (file, metadata),
                    &file_whitelist,
                    &dir_whitelist,
                ),
                _ => Response::builder()
                    .status(404)
                    .body("404 Not Found".into())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
            },
            Err(e) => Err(e),
        })
    }

    pub fn new<T>(addr: SocketAddr, root: PathBuf, dir_whitelist_iterator: T) -> FileServer
    where
        T: Iterator<Item = PathBuf>,
    {
        let file_whitelist: Arc<CHashMap<_, ()>> = Default::default();
        let dir_whitelist: Arc<Trie<PathBuf, ()>> =
            Arc::new(dir_whitelist_iterator.map(|i| (i, ())).collect());

        let (cancellation_token, cancellation_receiver) = oneshot::channel();

        let join_handle = thread::spawn({
            clone_all!(root, file_whitelist, dir_whitelist);
            move || {
                let service = {
                    clone_all!(root, file_whitelist, dir_whitelist);
                    move || {
                        service_fn({
                            clone_all!(root, file_whitelist, dir_whitelist);
                            move |request: Request<Body>| {
                                FileServer::resolve_request(
                                    request,
                                    &root,
                                    file_whitelist.clone(),
                                    dir_whitelist.clone(),
                                )
                            }
                        })
                    }
                };

                let server = Server::bind(&addr).serve(service);

                hyper::rt::run(
                    server
                        .select2(cancellation_receiver)
                        .map(|_| ())
                        .map_err(|_| ()),
                );
            }
        });

        FileServer {
            root,
            join_handle: Some(join_handle),
            file_whitelist,
            cancellation_token: Some(cancellation_token),
        }
    }

    #[allow(unused)]
    pub fn join(mut self) {
        self.join_handle.take().unwrap().join().unwrap();
    }

    pub fn insert_to_file_whitelist(&mut self, item: PathBuf) {
        self.file_whitelist.insert(self.root.join(item), ());
    }
}
