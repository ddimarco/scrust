

macro_rules! struct_events {
    (
        mouse: { $( $m_alias:ident : $m_sdl:ident),* },

        else: { $( $e_alias:ident : $e_sdl:pat ),* }
    )
    => {
        use sdl2::EventPump;

        #[derive(Clone)]
        pub struct ImmediateEvents {
            $( pub $m_alias: bool,)*
            $( pub $e_alias: bool, )*
            resize: Option<(u32, u32)>,
            pub mouse_move: Option<(i32, i32)>,

            pressed_keys: HashSet<Keycode>,
        }

        impl ImmediateEvents {
            pub fn new() -> ImmediateEvents {
                ImmediateEvents {
                    $( $m_alias: false ,) *
                    $( $e_alias: false, ) *
                    resize: None,
                    mouse_move: None,
                    pressed_keys: HashSet::<Keycode>::new(),
                }
            }
            pub fn is_key_pressed(&self, key: &sdl2::keyboard::Keycode) -> bool {
                self.pressed_keys.contains(key)
            }

        }

        use sdl2::rect::Point;

        pub struct Events {
            pump: EventPump,

            pub mouse_pos: Point,
            pub mouse_down_pos: Option<(i32, i32)>,

            pub now: ImmediateEvents,
        }

        use sdl2::event::Event::*;
        // use sdl2::event::WindowEventId::Resized;
        use sdl2::keyboard::Keycode;
        use sdl2::mouse::MouseButton;

        impl Events {
            pub fn new(pump: EventPump) -> Events {
                Events {
                    pump: pump,
                    now: ImmediateEvents::new(),
                    mouse_pos: Point::new(0,0),
                    mouse_down_pos: None,
                }
            }

            pub fn pump(&mut self, _: &mut ::sdl2::render::Renderer) {
                self.now = ImmediateEvents::new();

                self.now.pressed_keys = self.pump.keyboard_state().pressed_scancodes().filter_map(Keycode::from_scancode).collect();

                for event in self.pump.poll_iter() {
                    match event {
                        // Window { win_event_id: Resized, ..} => {
                            // self.now.resize = Some(renderer.output_size().unwrap());
                        // },
                        MouseMotion { x, y, .. } => {
                            self.now.mouse_move = Some((x, y));
                            self.mouse_pos = Point::new(x,y);

                        },

                        MouseButtonDown { mouse_btn, x, y, .. } => match mouse_btn {
                            $(
                                MouseButton::$m_sdl => {
                                   self.now.$m_alias = true;
                                    self.mouse_down_pos = Some((x,y));
                                }
                            ), *
                            _ => {}
                        },

                        MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                            $(
                                MouseButton::$m_sdl => {
                                    self.now.$m_alias = false;
                                    self.mouse_down_pos = None;
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
