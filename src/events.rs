
macro_rules! struct_events {
    (
        keyboard: { $( $k_alias:ident : $k_sdl:ident ),* },

        mouse: { $( $m_alias:ident : $m_sdl:ident),* },

        else: { $( $e_alias:ident : $e_sdl:pat ),* }
    )
    => {
        use sdl2::EventPump;

        #[derive(Clone, Copy)]
        pub struct ImmediateEvents {
            $( pub $k_alias: Option<bool> ,)*
            $( pub $m_alias: bool,)*
            $( pub $e_alias: bool, )*
            resize: Option<(u32, u32)>,
            pub mouse_move: Option<(i32, i32)>,
        }

        impl ImmediateEvents {
            pub fn new() -> ImmediateEvents {
                ImmediateEvents {
                    $( $k_alias: None, ) *
                    $( $m_alias: false ,) *
                    $( $e_alias: false, ) *
                    resize: None,
                    mouse_move: None,
                }
            }
        }

        use sdl2::rect::Point;

        pub struct Events {
            pump: EventPump,

            pub mouse_pos: Point,

            pub now: ImmediateEvents,
            $( pub $k_alias: bool),*
        }

        impl Events {
            pub fn new(pump: EventPump) -> Events {
                Events {
                    pump: pump,
                    now: ImmediateEvents::new(),
                    mouse_pos: Point::new(0,0),

                    $( $k_alias: false), *
                }
            }

            pub fn pump(&mut self, renderer: &mut ::sdl2::render::Renderer) {
                self.now = ImmediateEvents::new();

                for event in self.pump.poll_iter() {
                    use sdl2::event::Event::*;
                    use sdl2::event::WindowEventId::Resized;
                    use sdl2::keyboard::Keycode::*;
                    use sdl2::mouse::Mouse;

                    match event {
                        Window { win_event_id: Resized, ..} => {
                            self.now.resize = Some(renderer.output_size().unwrap());
                        },
                        KeyDown { keycode, .. } => match keycode {
                            $(
                                Some($k_sdl) => {
                                    if !self.$k_alias {
                                        self.now.$k_alias = Some(true);
                                    }

                                    self.$k_alias = true;
                                }
                            ),*
                            _ => {}
                        },

                        KeyUp { keycode, .. } => match keycode {
                            $(
                                Some($k_sdl) => {
                                    self.now.$k_alias = Some(false);
                                    self.$k_alias = false;
                                }
                            ),*
                            _ => {}
                        },

                        MouseMotion { x, y, .. } => {
                            self.now.mouse_move = Some((x, y));
                            self.mouse_pos = Point::new(x,y);
                        },

                        MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                            $(
                                Mouse::$m_sdl => {
                                   self.now.$m_alias = true;
                                }
                            ), *
                            _ => {}
                        },

                        MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                            $(
                                Mouse::$m_sdl => {
                                    self.now.$m_alias = false;
                                }
                            ), *
                                _ => {}
                        },

                        $(
                            $e_sdl => {
                                self.now.$e_alias = true;
                            }
                        )*,
                        _ => {}
                    }
                }
            }


        }
    }
}
