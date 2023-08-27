pub mod river_protocols {
    use wayland_client;
    // import objects from the core protocol if needed
    use wayland_client::protocol::*;

    // This module hosts a low-level representation of the protocol objects
    // you will not need to interact with it yourself, but the code generated
    // by the generate_client_code! macro will use it
    // import the interfaces from the core protocol if needed

    #[allow(non_upper_case_globals)]
    pub mod __status {
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("./resources/river-status-unstable-v1.xml");
    }

    #[allow(non_upper_case_globals)]
    pub mod __control {
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("./resources/river-control-unstable-v1.xml");
    }
    use self::__control::*;
    use self::__status::*;

    // This macro generates the actual types that represent the wayland objects of
    // your custom protocol
    wayland_scanner::generate_client_code!("./resources/river-status-unstable-v1.xml");
    wayland_scanner::generate_client_code!("./resources/river-control-unstable-v1.xml");
}
