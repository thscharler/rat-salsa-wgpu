use crossterm::event::{MediaKeyCode, ModifierKeyCode};

#[allow(dead_code)]
pub fn to_crossterm_event(
    event: winit::event::WindowEvent,
    modifiers: winit::event::Modifiers,
) -> Option<crossterm::event::Event> {
    match event {
        winit::event::WindowEvent::Focused(v) => {
            if v {
                Some(crossterm::event::Event::FocusGained)
            } else {
                Some(crossterm::event::Event::FocusLost)
            }
        }
        winit::event::WindowEvent::KeyboardInput {
            event:
                winit::event::KeyEvent {
                    logical_key,
                    location,
                    state,
                    repeat,
                    ..
                },
            ..
        } => {
            let mut m = crossterm::event::KeyModifiers::empty();
            if modifiers.state().control_key() {
                m |= crossterm::event::KeyModifiers::CONTROL;
            }
            if modifiers.state().shift_key() {
                m |= crossterm::event::KeyModifiers::SHIFT;
            }
            if modifiers.state().alt_key() {
                m |= crossterm::event::KeyModifiers::ALT;
            }
            if modifiers.state().super_key() {
                m |= crossterm::event::KeyModifiers::SUPER;
            }

            let mut s = match state {
                winit::event::ElementState::Pressed => crossterm::event::KeyEventKind::Press,
                winit::event::ElementState::Released => crossterm::event::KeyEventKind::Release,
            };
            if repeat {
                s = crossterm::event::KeyEventKind::Repeat;
            }

            let l = match location {
                winit::keyboard::KeyLocation::Standard => crossterm::event::KeyEventState::NONE,
                winit::keyboard::KeyLocation::Left => crossterm::event::KeyEventState::NONE,
                winit::keyboard::KeyLocation::Right => crossterm::event::KeyEventState::NONE,
                winit::keyboard::KeyLocation::Numpad => crossterm::event::KeyEventState::KEYPAD,
            };

            let ke = match logical_key {
                winit::keyboard::Key::Character(c) => {
                    let c = c.as_str().chars().next().expect("char");
                    crossterm::event::KeyEvent::new_with_kind_and_state(
                        crossterm::event::KeyCode::Char(c),
                        m,
                        s,
                        l,
                    )
                }
                winit::keyboard::Key::Named(nk) => {
                    let kc = match nk {
                        winit::keyboard::NamedKey::Enter => crossterm::event::KeyCode::Enter,
                        winit::keyboard::NamedKey::Tab => {
                            if modifiers.state().shift_key() {
                                crossterm::event::KeyCode::BackTab
                            } else {
                                crossterm::event::KeyCode::Tab
                            }
                        }
                        winit::keyboard::NamedKey::Space => crossterm::event::KeyCode::Char(' '),
                        winit::keyboard::NamedKey::ArrowDown => crossterm::event::KeyCode::Down,
                        winit::keyboard::NamedKey::ArrowLeft => crossterm::event::KeyCode::Left,
                        winit::keyboard::NamedKey::ArrowRight => crossterm::event::KeyCode::Right,
                        winit::keyboard::NamedKey::ArrowUp => crossterm::event::KeyCode::Up,
                        winit::keyboard::NamedKey::End => crossterm::event::KeyCode::End,
                        winit::keyboard::NamedKey::Home => crossterm::event::KeyCode::Home,
                        winit::keyboard::NamedKey::PageDown => crossterm::event::KeyCode::PageDown,
                        winit::keyboard::NamedKey::PageUp => crossterm::event::KeyCode::PageUp,
                        winit::keyboard::NamedKey::Backspace => {
                            crossterm::event::KeyCode::Backspace
                        }
                        winit::keyboard::NamedKey::Delete => crossterm::event::KeyCode::Delete,
                        winit::keyboard::NamedKey::Insert => crossterm::event::KeyCode::Insert,
                        winit::keyboard::NamedKey::Escape => crossterm::event::KeyCode::Esc,
                        winit::keyboard::NamedKey::F1 => crossterm::event::KeyCode::F(1),
                        winit::keyboard::NamedKey::F2 => crossterm::event::KeyCode::F(2),
                        winit::keyboard::NamedKey::F3 => crossterm::event::KeyCode::F(3),
                        winit::keyboard::NamedKey::F4 => crossterm::event::KeyCode::F(4),
                        winit::keyboard::NamedKey::F5 => crossterm::event::KeyCode::F(5),
                        winit::keyboard::NamedKey::F6 => crossterm::event::KeyCode::F(6),
                        winit::keyboard::NamedKey::F7 => crossterm::event::KeyCode::F(7),
                        winit::keyboard::NamedKey::F8 => crossterm::event::KeyCode::F(8),
                        winit::keyboard::NamedKey::F9 => crossterm::event::KeyCode::F(9),
                        winit::keyboard::NamedKey::F10 => crossterm::event::KeyCode::F(10),
                        winit::keyboard::NamedKey::F11 => crossterm::event::KeyCode::F(11),
                        winit::keyboard::NamedKey::F12 => crossterm::event::KeyCode::F(12),
                        winit::keyboard::NamedKey::F13 => crossterm::event::KeyCode::F(13),
                        winit::keyboard::NamedKey::F14 => crossterm::event::KeyCode::F(14),
                        winit::keyboard::NamedKey::F15 => crossterm::event::KeyCode::F(15),
                        winit::keyboard::NamedKey::F16 => crossterm::event::KeyCode::F(16),
                        winit::keyboard::NamedKey::F17 => crossterm::event::KeyCode::F(17),
                        winit::keyboard::NamedKey::F18 => crossterm::event::KeyCode::F(18),
                        winit::keyboard::NamedKey::F19 => crossterm::event::KeyCode::F(19),
                        winit::keyboard::NamedKey::F20 => crossterm::event::KeyCode::F(20),
                        winit::keyboard::NamedKey::F21 => crossterm::event::KeyCode::F(21),
                        winit::keyboard::NamedKey::F22 => crossterm::event::KeyCode::F(22),
                        winit::keyboard::NamedKey::F23 => crossterm::event::KeyCode::F(23),
                        winit::keyboard::NamedKey::F24 => crossterm::event::KeyCode::F(24),
                        winit::keyboard::NamedKey::F25 => crossterm::event::KeyCode::F(25),
                        winit::keyboard::NamedKey::F26 => crossterm::event::KeyCode::F(26),
                        winit::keyboard::NamedKey::F27 => crossterm::event::KeyCode::F(27),
                        winit::keyboard::NamedKey::F28 => crossterm::event::KeyCode::F(28),
                        winit::keyboard::NamedKey::F29 => crossterm::event::KeyCode::F(29),
                        winit::keyboard::NamedKey::F30 => crossterm::event::KeyCode::F(30),
                        winit::keyboard::NamedKey::F31 => crossterm::event::KeyCode::F(31),
                        winit::keyboard::NamedKey::F32 => crossterm::event::KeyCode::F(32),
                        winit::keyboard::NamedKey::F33 => crossterm::event::KeyCode::F(33),
                        winit::keyboard::NamedKey::F34 => crossterm::event::KeyCode::F(34),
                        winit::keyboard::NamedKey::F35 => crossterm::event::KeyCode::F(35),
                        winit::keyboard::NamedKey::CapsLock => crossterm::event::KeyCode::CapsLock,
                        winit::keyboard::NamedKey::ScrollLock => {
                            crossterm::event::KeyCode::ScrollLock
                        }
                        winit::keyboard::NamedKey::NumLock => crossterm::event::KeyCode::NumLock,
                        winit::keyboard::NamedKey::PrintScreen => {
                            crossterm::event::KeyCode::PrintScreen
                        }
                        winit::keyboard::NamedKey::Pause => crossterm::event::KeyCode::Pause,
                        winit::keyboard::NamedKey::ContextMenu => crossterm::event::KeyCode::Menu,
                        winit::keyboard::NamedKey::MediaPlay => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::Play)
                        }
                        winit::keyboard::NamedKey::MediaPause => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::Pause)
                        }
                        winit::keyboard::NamedKey::MediaPlayPause => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::PlayPause)
                        }
                        winit::keyboard::NamedKey::MediaStop => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::Stop)
                        }
                        winit::keyboard::NamedKey::MediaFastForward => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::FastForward)
                        }
                        winit::keyboard::NamedKey::MediaRewind => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::Rewind)
                        }
                        winit::keyboard::NamedKey::MediaTrackNext => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::TrackNext)
                        }
                        winit::keyboard::NamedKey::MediaTrackPrevious => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::TrackPrevious)
                        }
                        winit::keyboard::NamedKey::MediaRecord => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::Record)
                        }
                        winit::keyboard::NamedKey::AudioVolumeDown => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::LowerVolume)
                        }
                        winit::keyboard::NamedKey::AudioVolumeUp => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::RaiseVolume)
                        }
                        winit::keyboard::NamedKey::AudioVolumeMute => {
                            crossterm::event::KeyCode::Media(MediaKeyCode::MuteVolume)
                        }
                        winit::keyboard::NamedKey::Shift => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftShift)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightShift)
                            }
                        }
                        winit::keyboard::NamedKey::Control => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftControl)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightControl)
                            }
                        }
                        winit::keyboard::NamedKey::Alt => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftAlt)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightAlt)
                            }
                        }
                        winit::keyboard::NamedKey::Super => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftSuper)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightSuper)
                            }
                        }
                        winit::keyboard::NamedKey::Meta => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftMeta)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightMeta)
                            }
                        }
                        winit::keyboard::NamedKey::Hyper => {
                            if location == winit::keyboard::KeyLocation::Left {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::LeftHyper)
                            } else {
                                crossterm::event::KeyCode::Modifier(ModifierKeyCode::RightHyper)
                            }
                        }
                        _ => return None,
                    };
                    crossterm::event::KeyEvent::new_with_kind_and_state(kc, m, s, l)
                }
                _ => return None,
            };
            Some(crossterm::event::Event::Key(ke))
        }
        winit::event::WindowEvent::CursorMoved { .. } => None,
        winit::event::WindowEvent::CursorEntered { .. } => None,
        winit::event::WindowEvent::CursorLeft { .. } => None,
        winit::event::WindowEvent::MouseWheel { .. } => None,
        winit::event::WindowEvent::MouseInput { .. } => None,
        _ => None,
    }
}
