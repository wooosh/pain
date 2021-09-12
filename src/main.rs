extern crate sdl2;
extern crate swash;
extern crate zeno;

mod text_renderer;
use text_renderer::{FontManager, FontData};


fn main() {
  let mut font_manager = FontManager::default();
  let go_mono = FontData::from_file("/usr/share/fonts/TTF/Go-Mono.ttf", 0).unwrap();

  let textcolor: [u8; 4] = [30, 30, 30, 255];
  let destcolor: [u8; 4] = [230, 230, 230, 255];
  let fm = &mut font_manager;

  let mut img = fm.render_glyph(go_mono.as_ref(), 32, go_mono.charmap().map('q'), textcolor, destcolor);
  /*
  let font = Font::from_file("/usr/share/fonts/TTF/Go-Mono.ttf", 0).unwrap();
  let mut scaleContext = scale::ScaleContext::new();
  let mut scaler = scaleContext.builder(font.as_ref())
    .size(18.)
    .hint(false)
    .build();

  draw_text(&mut scaler, font.charmap().map('r'));*/
  
  let sdl = sdl2::init().unwrap();

  let video_subsystem = sdl.video().unwrap();

  let window = video_subsystem
    .window("editor", 1000, 1000)
    .resizable()
    .build()
    .unwrap();

  video_subsystem.text_input().start();

  let mut canvas = window.into_canvas().build().unwrap();
  let texture_creator = canvas.texture_creator();

  let surface = sdl2::surface::Surface::from_data(&mut img.data, img.width, img.height, img.width*4, sdl2::pixels::PixelFormatEnum::ARGB8888).unwrap();
  let texture = texture_creator
    .create_texture_from_surface(&surface)
    .unwrap();
  //let tr = text_renderer::TextRenderer::new(&texture_creator);

  canvas.set_draw_color(sdl2::pixels::Color::RGBA(230, 230, 230, 255));
  canvas.clear();


  //let texture = draw_text(&texture_creator, &mut scaler, font.charmap().map('r'));
  let sdl2::render::TextureQuery {width, height, ..} = texture.query();
  canvas.copy(&texture, None, sdl2::rect::Rect::new(5, 5, width, height)).unwrap();
  /*
  
  let texture = draw_text(&texture_creator, &mut scaler, font.charmap().map('g'));
  let sdl2::render::TextureQuery {width, height, ..} = texture.query();
  canvas.copy(&texture, None, sdl2::rect::Rect::new(5+10, 5, width, height)).unwrap();

  let texture = draw_text(&texture_creator, &mut scaler, font.charmap().map('b'));
  let sdl2::render::TextureQuery {width, height, ..} = texture.query();
  canvas.copy(&texture, None, sdl2::rect::Rect::new(5+10*2, 5, width, height)).unwrap();
  */

  
  canvas.present(); 


  
  
  let mut buffer: Vec<u8> = Vec::new();

  let mut event_pump = sdl.event_pump().unwrap();
  'main: loop {
    for event in event_pump.poll_iter() {
      match event {
        sdl2::event::Event::KeyDown { keycode: Some(keycode), .. } => {
          println!("keydown");
        },
        sdl2::event::Event::TextInput { text, .. } => {
          buffer.extend(text.as_bytes());
          println!("textinput: {}", text);
          println!("buffer: {:?}", buffer);
        },
        sdl2::event::Event::Quit {..} => break 'main,
        _ => {},
      }
    }
  }
}
