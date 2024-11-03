use clover::State;
use clover::helper::make_reference;

mod io;
mod random;
mod math;
mod helper;
mod map;
mod array;
mod os;
mod net;

pub fn clover_std_inject_to(state: &mut State) {
    state.add_native_function("print", io::print);

    state.add_native_model("IO", make_reference(io::IO {}));
    state.add_native_model("Random", make_reference(random::Random {}));
    state.add_native_model("Math", make_reference(math::Math {}));

    state.add_native_model("Array", make_reference(array::Array {}));
    state.add_native_model("Net", make_reference(net::Net {}));

    state.add_native_model("OS", make_reference(os::Os {}));

    state.add_native_model("Map", make_reference(map::Map {}));
}