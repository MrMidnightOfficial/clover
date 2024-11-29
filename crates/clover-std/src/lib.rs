use clover::Env;
use clover::helper::make_reference;

mod io;
mod random;
mod math;
mod helper;
mod map;
mod array;
mod os;
mod net;

pub fn clover_std_inject_to(env: &mut Env) {
    env.add_native_function("print", io::print);

    env.add_native_model("IO", make_reference(io::IO {}));
    env.add_native_model("Random", make_reference(random::Random {}));
    env.add_native_model("Math", make_reference(math::Math {}));

    env.add_native_model("Array", make_reference(array::Array {}));
    env.add_native_model("Net", make_reference(net::Net {}));

    env.add_native_model("OS", make_reference(os::Os {}));

    env.add_native_model("Map", make_reference(map::Map {}));
}