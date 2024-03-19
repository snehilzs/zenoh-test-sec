//
// Copyright (c) 2022 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use async_std::net::SocketAddr;
use async_trait::async_trait;
use core::{
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};
use serde::Serialize;
use std::any::{Any, TypeId};
use zenoh_protocol::core::{EndPoint, Locator};
use zenoh_result::ZResult;

pub type LinkManagerUnicast = Arc<dyn LinkManagerUnicastTrait>;
#[async_trait]
pub trait LinkManagerUnicastTrait: Send + Sync {
    async fn new_link(&self, endpoint: EndPoint) -> ZResult<LinkUnicast>;
    async fn new_listener(&self, endpoint: EndPoint) -> ZResult<Locator>;
    async fn del_listener(&self, endpoint: &EndPoint) -> ZResult<()>;
    fn get_listeners(&self) -> Vec<EndPoint>;
    fn get_locators(&self) -> Vec<Locator>;
}
pub type NewLinkChannelSender = flume::Sender<LinkUnicast>;
pub trait ConstructibleLinkManagerUnicast<T>: Sized {
    fn new(new_link_sender: NewLinkChannelSender, config: T) -> ZResult<Self>;
}

#[derive(Clone)]
pub struct LinkUnicast(pub Arc<dyn LinkUnicastTrait>);

#[async_trait]
pub trait LinkUnicastTrait: Send + Sync {
    fn get_mtu(&self) -> u16;
    fn get_src(&self) -> &Locator;
    fn get_dst(&self) -> &Locator;
    fn is_reliable(&self) -> bool;
    fn is_streamed(&self) -> bool;
    fn get_interface_names(&self) -> Vec<String>;
    fn get_auth_identifier(&self) -> AuthIdentifier;
    async fn write(&self, buffer: &[u8]) -> ZResult<usize>;
    async fn write_all(&self, buffer: &[u8]) -> ZResult<()>;
    async fn read(&self, buffer: &mut [u8]) -> ZResult<usize>;
    async fn read_exact(&self, buffer: &mut [u8]) -> ZResult<()>;
    async fn close(&self) -> ZResult<()>;
}

impl Deref for LinkUnicast {
    type Target = Arc<dyn LinkUnicastTrait>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for LinkUnicast {}

impl PartialEq for LinkUnicast {
    fn eq(&self, other: &Self) -> bool {
        self.get_src() == other.get_src() && self.get_dst() == other.get_dst()
    }
}

impl Hash for LinkUnicast {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get_src().hash(state);
        self.get_dst().hash(state);
    }
}

impl fmt::Display for LinkUnicast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} => {}", self.get_src(), self.get_dst())
    }
}

impl fmt::Debug for LinkUnicast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Link")
            .field("src", &self.get_src())
            .field("dst", &self.get_dst())
            .field("mtu", &self.get_mtu())
            .field("is_reliable", &self.is_reliable())
            .field("is_streamed", &self.is_streamed())
            .finish()
    }
}

impl From<Arc<dyn LinkUnicastTrait>> for LinkUnicast {
    fn from(link: Arc<dyn LinkUnicastTrait>) -> LinkUnicast {
        LinkUnicast(link)
    }
}

pub fn get_ip_interface_names(addr: &SocketAddr) -> Vec<String> {
    match zenoh_util::net::get_interface_names_by_addr(addr.ip()) {
        Ok(interfaces) => {
            log::trace!("get_interface_names for {:?}: {:?}", addr.ip(), interfaces);
            interfaces
        }
        Err(e) => {
            log::debug!("get_interface_names for {:?} failed: {:?}", addr.ip(), e);
            vec![]
        }
    }
}

#[derive(Clone, Debug, Serialize, Hash, PartialEq, Eq)]

pub struct AuthIdentifier {
    pub username: Option<String>,
    pub tls_cert_name: Option<String>,
}

pub enum AuthIdType {
    None,
    Tls,
    Username,
}

pub struct AuthId<'a> {
    pub auth_type: &'a Option<AuthIdType>,
    pub auth_value: &'a Option<Box<dyn Any>>, //downcast in interceptor by auth_id_type
}

impl AuthId<'_> {
    pub fn builder() -> AuthIdBuilder {
        AuthIdBuilder::default()
    }
    pub fn get_type() {} //gets the authId type

    pub fn get_value() {} //get the authId value to be used in ACL
}
#[derive(Default)]
pub struct AuthIdBuilder {
    pub auth_type: Option<AuthIdType>,
    pub auth_value: Option<Box<dyn Any>>, //downcast in interceptor by auth_id_type
                                          /*
                                             possible values for auth_value: Vec<String> and String (as of now)
                                          */
}

impl AuthIdBuilder {
    pub fn new(&self) -> Self {
        AuthIdBuilder::default()
    }

    pub fn id_type(&mut self, auth_id_type: impl Into<AuthIdType>) -> &mut Self {
        //adds type
        let _ = self.auth_type.insert(auth_id_type.into());
        self
    }
    pub fn value(&mut self, auth_id_value: impl Into<Box<dyn Any>>) -> &mut Self {
        let _ = auth_id_value;
        //adds type
        self
    }

    pub fn build(&mut self) -> ZResult<AuthId> {
        let auth_type = &self.auth_type;
        let auth_value = &self.auth_value;
        Ok(AuthId {
            auth_type,
            auth_value,
        })
    }
}

// pub trait AuthIdTrait<Id, Builder> {
//     fn builder() -> Builder {
//         Builder::default()
//     }
//     fn get_type() {} //gets the authId type

//     fn get_value() {} //get the authId value to be used in ACL
// }

//
// pub trait AuthIdValue {
//     //define features to restrict auth_value behaviour
// }
