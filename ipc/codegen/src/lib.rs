// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Codegen for IPC RPC

#![cfg_attr(feature = "nightly-testing", plugin(clippy))]
#![cfg_attr(feature = "nightly-testing", feature(plugin))]
#![cfg_attr(feature = "nightly-testing", allow(used_underscore_binding))]
#![cfg_attr(not(feature = "with-syntex"), feature(rustc_private, plugin))]
#![cfg_attr(not(feature = "with-syntex"), plugin(quasi_macros))]

extern crate aster;
extern crate quasi;

#[cfg(feature = "with-syntex")]
extern crate syntex;

#[cfg(feature = "with-syntex")]
#[macro_use]
extern crate syntex_syntax as syntax;

#[cfg(not(feature = "with-syntex"))]
#[macro_use]
extern crate syntax;

#[cfg(not(feature = "with-syntex"))]
extern crate rustc_plugin;

#[cfg(not(feature = "with-syntex"))]
use syntax::feature_gate::AttributeType;

#[cfg(feature = "with-syntex")]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(not(feature = "with-syntex"))]
include!("lib.rs.in");

#[cfg(feature = "with-syntex")]
pub fn register(reg: &mut syntex::Registry) {
	use syntax::{ast, fold};

	#[cfg(feature = "with-syntex")]
	fn strip_attributes(krate: ast::Crate) -> ast::Crate {
		struct StripAttributeFolder;
		impl fold::Folder for StripAttributeFolder {
			fn fold_attribute(&mut self, attr: ast::Attribute) -> Option<ast::Attribute> {
				match attr.node.value.node {
					ast::MetaItemKind::List(ref n, _) if n == &"ipc" => { return None; }
					_ => {}
				}

				Some(attr)
			}

			fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac {
				fold::noop_fold_mac(mac, self)
			}
		}

		fold::Folder::fold_crate(&mut StripAttributeFolder, krate)
	}

	reg.add_attr("feature(custom_derive)");
	reg.add_attr("feature(custom_attribute)");

	reg.add_decorator("derive_Ipc", codegen::expand_ipc_implementation);
	reg.add_decorator("derive_Binary", serialization::expand_serialization_implementation);

	reg.add_post_expansion_pass(strip_attributes);
}

#[cfg(not(feature = "with-syntex"))]
pub fn register(reg: &mut rustc_plugin::Registry) {
	reg.register_syntax_extension(
		syntax::parse::token::intern("derive_Ipc"),
		syntax::ext::base::MultiDecorator(
			Box::new(codegen::expand_ipc_implementation)));
	reg.register_syntax_extension(
		syntax::parse::token::intern("derive_Binary"),
		syntax::ext::base::MultiDecorator(
			Box::new(serialization::expand_serialization_implementation)));

	reg.register_attribute("ipc".to_owned(), AttributeType::Normal);
}
