use util::*;
use crypto::sha2::Sha256;
use crypto::ripemd160::Ripemd160;
use crypto::digest::Digest;

/// Definition of a contract whose implementation is built-in. 
pub struct Builtin {
	/// The gas cost of running this built-in for the given size of input data.
	pub cost: Box<Fn(usize) -> U256>,	// TODO: U256 should be bignum.
	/// Run this built-in function with the input being the first argument and the output
	/// being placed into the second.
	pub execute: Box<Fn(&[u8], &mut [u8])>,
}

// Rust does not mark closurer that do not capture as Sync
// We promise that all builtins are thread safe since they only operate on given input.
unsafe impl Sync for Builtin {}
unsafe impl Send for Builtin {}

impl fmt::Debug for Builtin {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "<Builtin>")
	}
}

impl Builtin {
	/// Create a new object from components.
	pub fn new(cost: Box<Fn(usize) -> U256>, execute: Box<Fn(&[u8], &mut [u8])>) -> Builtin {
		Builtin {cost: cost, execute: execute}
	}

	/// Create a new object from a builtin-function name with a linear cost associated with input size.
	pub fn from_named_linear(name: &str, base_cost: usize, word_cost: usize) -> Option<Builtin> {
		new_builtin_exec(name).map(|b| {
			let cost = Box::new(move|s: usize| -> U256 {
				U256::from(base_cost) + U256::from(word_cost) * U256::from((s + 31) / 32)
			});
			Self::new(cost, b)
		})
	}

	/// Simple forwarder for cost.
	pub fn cost(&self, s: usize) -> U256 { (*self.cost)(s) }

	/// Simple forwarder for execute.
	pub fn execute(&self, input: &[u8], output: &mut[u8]) { (*self.execute)(input, output); }

	/// Create a builtin from JSON.
	///
	/// JSON must be of the form `{ "name": "identity", "linear": {"base": 10, "word": 20} }`.
	pub fn from_json(json: &Json) -> Option<Builtin> {
		// NICE: figure out a more convenient means of handing errors here.
		if let Json::String(ref name) = json["name"] {
			if let Json::Object(ref o) = json["linear"] {
				if let Json::U64(ref word) = o["word"] {
					if let Json::U64(ref base) = o["base"] {
						return Self::from_named_linear(&name[..], *base as usize, *word as usize);
					}
				}
			}
		}
		None
	}
}

/// Copy a bunch of bytes to a destination; if the `src` is too small to fill `dest`,
/// leave the rest unchanged.
pub fn copy_to(src: &[u8], dest: &mut[u8]) {
	// NICE: optimise
	for i in 0..min(src.len(), dest.len()) {
		dest[i] = src[i];
	}
}

/// Create a new builtin executor according to `name`.
/// TODO: turn in to a factory with dynamic registration.
pub fn new_builtin_exec(name: &str) -> Option<Box<Fn(&[u8], &mut [u8])>> {
	match name {
		"identity" => Some(Box::new(move|input: &[u8], output: &mut[u8]| {
			for i in 0..min(input.len(), output.len()) {
				output[i] = input[i];
			}
		})),
		"ecrecover" => Some(Box::new(move|input: &[u8], output: &mut[u8]| {
			#[repr(packed)]
			#[derive(Debug)]
			struct InType {
				hash: H256,
				v: H256,
				r: H256,
				s: H256,
			}
			let mut it: InType = InType { hash: H256::new(), v: H256::new(), r: H256::new(), s: H256::new() };
			it.copy_raw(input);
			if it.v == H256::from(&U256::from(27)) || it.v == H256::from(&U256::from(28)) {
				let s = Signature::from_rsv(&it.r, &it.s, it.v[31] - 27);
				if ec::is_valid(&s) {
					if let Ok(p) = ec::recover(&s, &it.hash) {
						let r = p.as_slice().sha3();
						// NICE: optimise and separate out into populate-like function
						for i in 0..min(32, output.len()) {
							output[i] = if i < 12 {0} else {r[i]};
						}
					}
				}
			}
		})),
		"sha256" => Some(Box::new(move|input: &[u8], output: &mut[u8]| {
			let mut sha = Sha256::new();
			sha.input(input);
			if output.len() >= 32 {
				sha.result(output);
			} else {
				let mut ret = H256::new();
				sha.result(ret.as_slice_mut());
				copy_to(&ret, output);
			}
		})),
		"ripemd160" => Some(Box::new(move|input: &[u8], output: &mut[u8]| {
			let mut sha = Ripemd160::new();
			sha.input(input);
			let mut ret = H256::new();
			sha.result(&mut ret.as_slice_mut()[12..32]);
			copy_to(&ret, output);
		})),
		_ => None
	}
}

#[test]
fn identity() {
	let f = new_builtin_exec("identity").unwrap();
	let i = [0u8, 1, 2, 3];

	let mut o2 = [255u8; 2];
	f(&i[..], &mut o2[..]);
	assert_eq!(i[0..2], o2);

	let mut o4 = [255u8; 4];
	f(&i[..], &mut o4[..]);
	assert_eq!(i, o4);

	let mut o8 = [255u8; 8];
	f(&i[..], &mut o8[..]);
	assert_eq!(i, o8[..4]);
	assert_eq!([255u8; 4], o8[4..]);
}

#[test]
fn sha256() {
	use rustc_serialize::hex::FromHex;
	let f = new_builtin_exec("sha256").unwrap();
	let i = [0u8; 0];

	let mut o = [255u8; 32];
	f(&i[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap())[..]);

	let mut o8 = [255u8; 8];
	f(&i[..], &mut o8[..]);
	assert_eq!(&o8[..], &(FromHex::from_hex("e3b0c44298fc1c14").unwrap())[..]);

	let mut o34 = [255u8; 34];
	f(&i[..], &mut o34[..]);
	assert_eq!(&o34[..], &(FromHex::from_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855ffff").unwrap())[..]);
}

#[test]
fn ripemd160() {
	use rustc_serialize::hex::FromHex;
	let f = new_builtin_exec("ripemd160").unwrap();
	let i = [0u8; 0];

	let mut o = [255u8; 32];
	f(&i[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("0000000000000000000000009c1185a5c5e9fc54612808977ee8f548b2258d31").unwrap())[..]);

	let mut o8 = [255u8; 8];
	f(&i[..], &mut o8[..]);
	assert_eq!(&o8[..], &(FromHex::from_hex("0000000000000000").unwrap())[..]);

	let mut o34 = [255u8; 34];
	f(&i[..], &mut o34[..]);
	assert_eq!(&o34[..], &(FromHex::from_hex("0000000000000000000000009c1185a5c5e9fc54612808977ee8f548b2258d31ffff").unwrap())[..]);
}

#[test]
fn ecrecover() {
	use rustc_serialize::hex::FromHex;
	/*let k = KeyPair::from_secret(b"test".sha3()).unwrap();
	let a: Address = From::from(k.public().sha3());
	println!("Address: {}", a);
	let m = b"hello world".sha3();
	println!("Message: {}", m);
	let s = k.sign(&m).unwrap();
	println!("Signed: {}", s);*/

	let f = new_builtin_exec("ecrecover").unwrap();
	let i = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001b650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e03").unwrap();

	let mut o = [255u8; 32];
	f(&i[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("000000000000000000000000c08b5542d177ac6686946920409741463a15dddb").unwrap())[..]);

	let mut o8 = [255u8; 8];
	f(&i[..], &mut o8[..]);
	assert_eq!(&o8[..], &(FromHex::from_hex("0000000000000000").unwrap())[..]);

	let mut o34 = [255u8; 34];
	f(&i[..], &mut o34[..]);
	assert_eq!(&o34[..], &(FromHex::from_hex("000000000000000000000000c08b5542d177ac6686946920409741463a15dddbffff").unwrap())[..]);

	let i_bad = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001a650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e03").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);

	let i_bad = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001b000000000000000000000000000000000000000000000000000000000000001b0000000000000000000000000000000000000000000000000000000000000000").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);

	let i_bad = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001b").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);

	let i_bad = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001bffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff000000000000000000000000000000000000000000000000000000000000001b").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);

	let i_bad = FromHex::from_hex("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001b000000000000000000000000000000000000000000000000000000000000001bffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);

	// TODO: Should this (corrupted version of the above) fail rather than returning some address?
/*	let i_bad = FromHex::from_hex("48173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad000000000000000000000000000000000000000000000000000000000000001b650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e03").unwrap();
	let mut o = [255u8; 32];
	f(&i_bad[..], &mut o[..]);
	assert_eq!(&o[..], &(FromHex::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap())[..]);*/
}

#[test]
fn from_named_linear() {
	let b = Builtin::from_named_linear("identity", 10, 20).unwrap();
	assert_eq!((*b.cost)(0), U256::from(10));
	assert_eq!((*b.cost)(1), U256::from(30));
	assert_eq!((*b.cost)(32), U256::from(30));
	assert_eq!((*b.cost)(33), U256::from(50));

	let i = [0u8, 1, 2, 3];
	let mut o = [255u8; 4];
	(*b.execute)(&i[..], &mut o[..]);
	assert_eq!(i, o);
}

#[test]
fn from_json() {
	let text = "{ \"name\": \"identity\", \"linear\": {\"base\": 10, \"word\": 20} }";
	let json = Json::from_str(text).unwrap();
	let b = Builtin::from_json(&json).unwrap();
	assert_eq!((*b.cost)(0), U256::from(10));
	assert_eq!((*b.cost)(1), U256::from(30));
	assert_eq!((*b.cost)(32), U256::from(30));
	assert_eq!((*b.cost)(33), U256::from(50));

	let i = [0u8, 1, 2, 3];
	let mut o = [255u8; 4];
	(*b.execute)(&i[..], &mut o[..]);
	assert_eq!(i, o);
}