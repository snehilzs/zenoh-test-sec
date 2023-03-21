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
use crate::{RCodec, WCodec, Zenoh080, Zenoh080Condition, Zenoh080Header};
use zenoh_buffers::{
    reader::{DidntRead, Reader},
    writer::{DidntWrite, Writer},
};
use zenoh_protocol::{
    common::{imsg, ZExtUnit, ZExtUnknown},
    core::WireExpr,
    network::{
        id,
        push::{ext, flag},
        Mapping, Push,
    },
};

impl<W> WCodec<(ext::Destination, bool), &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: (ext::Destination, bool)) -> Self::Output {
        let (_, more) = x;
        let ext: ZExtUnit<{ ext::DST }> = ZExtUnit::new();
        self.write(&mut *writer, (&ext, more))
    }
}

impl<R> RCodec<(ext::Destination, bool), &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<(ext::Destination, bool), Self::Error> {
        let (_, more): (ZExtUnit<{ ext::DST }>, bool) = self.read(&mut *reader)?;
        Ok((ext::Destination::Queryables, more))
    }
}

impl<W> WCodec<&Push, &mut W> for Zenoh080
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &Push) -> Self::Output {
        // Header
        let mut header = id::PUSH;
        let mut n_exts = ((x.ext_qos != ext::QoS::default()) as u8)
            + (x.ext_tstamp.is_some() as u8 + (x.ext_dst != ext::Destination::default()) as u8);
        if n_exts != 0 {
            header |= flag::Z;
        }
        if x.mapping != Mapping::default() {
            header |= flag::M;
        }
        if x.wire_expr.has_suffix() {
            header |= flag::N;
        }
        self.write(&mut *writer, header)?;

        // Body
        self.write(&mut *writer, &x.wire_expr)?;

        // Extensions
        if x.ext_qos != ext::QoS::default() {
            n_exts -= 1;
            self.write(&mut *writer, (x.ext_qos, n_exts != 0))?;
        }
        if let Some(ts) = x.ext_tstamp.as_ref() {
            n_exts -= 1;
            self.write(&mut *writer, (ts, n_exts != 0))?;
        }
        if x.ext_dst != ext::Destination::default() {
            n_exts -= 1;
            self.write(&mut *writer, (x.ext_dst, n_exts != 0))?;
        }

        // Payload
        self.write(&mut *writer, &x.payload)?;

        Ok(())
    }
}

impl<R> RCodec<Push, &mut R> for Zenoh080
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<Push, Self::Error> {
        let header: u8 = self.read(&mut *reader)?;
        let codec = Zenoh080Header::new(header);
        codec.read(reader)
    }
}

impl<R> RCodec<Push, &mut R> for Zenoh080Header
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<Push, Self::Error> {
        if imsg::mid(self.header) != id::PUSH {
            return Err(DidntRead);
        }

        // Body
        let ccond = Zenoh080Condition::new(imsg::has_flag(self.header, flag::N));
        let wire_expr: WireExpr<'static> = ccond.read(&mut *reader)?;
        let mapping = if imsg::has_flag(self.header, flag::M) {
            Mapping::Sender
        } else {
            Mapping::Receiver
        };

        // Extensions
        let mut ext_qos = ext::QoS::default();
        let mut ext_tstamp = None;
        let mut ext_dst = ext::Destination::default();

        let mut has_ext = imsg::has_flag(self.header, flag::Z);
        while has_ext {
            let ext: u8 = self.codec.read(&mut *reader)?;
            let eodec = Zenoh080Header::new(ext);
            match imsg::mid(ext) {
                ext::QOS => {
                    let (q, ext): (ext::QoS, bool) = eodec.read(&mut *reader)?;
                    ext_qos = q;
                    has_ext = ext;
                }
                ext::TSTAMP => {
                    let (t, ext): (ext::Timestamp, bool) = eodec.read(&mut *reader)?;
                    ext_tstamp = Some(t);
                    has_ext = ext;
                }
                ext::DST => {
                    let (d, ext): (ext::Destination, bool) = eodec.read(&mut *reader)?;
                    ext_dst = d;
                    has_ext = ext;
                }
                _ => {
                    let (_, ext): (ZExtUnknown, bool) = eodec.read(&mut *reader)?;
                    has_ext = ext;
                }
            }
        }

        // Payload
        // let payload: ZenohMessage = self.codec.read(&mut *reader)?;
        let payload: u8 = self.codec.read(&mut *reader)?; // @TODO

        Ok(Push {
            wire_expr,
            mapping,
            payload,
            ext_qos,
            ext_tstamp,
            ext_dst,
        })
    }
}
