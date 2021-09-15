extern crate sdl2;
extern crate swash;
extern crate zeno;

mod text_renderer;
use text_renderer::{GlyphRenderer, FontData};

mod piece_table;


fn main() {

  if true {return};
  let mut font_manager = GlyphRenderer::default();
  let go_mono = FontData::from_file("/usr/share/fonts/TTF/FiraCode-Regular.ttf", 0).unwrap();
  

  let textcolor: [u8; 4] = [30, 30, 30, 255];
  let destcolor: [u8; 4] = [230, 230, 230, 255];
  let fm = &mut font_manager;

  let size = 14.0;
  let mut shape_ctx = swash::shape::ShapeContext::new();
  let mut shaper = shape_ctx.builder(go_mono.as_ref())
    .script(swash::text::Script::Latin)
    .size(size)
    .build();
  
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

  canvas.set_draw_color(sdl2::pixels::Color::RGBA(230, 230, 230, 255));
  canvas.clear();

  shaper.add_str("hello world =>;");
  let mut x = 5.0;
  shaper.shape_with(|gc: &swash::shape::cluster::GlyphCluster<'_>| {
    for glyph in gc.glyphs {
      let mut img = fm.render_glyph(go_mono.as_ref(), size as u32, glyph.id, textcolor, destcolor);
      if img.height == 0 || img.width == 0 {x += glyph.advance; continue}
      let surface = sdl2::surface::Surface::from_data(&mut img.data, img.width, img.height, img.width*4, sdl2::pixels::PixelFormatEnum::ABGR8888).unwrap();
      let texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    
      let sdl2::render::TextureQuery {width, height, ..} = texture.query();
      canvas.copy(&texture, None, sdl2::rect::Rect::new(x.round() as i32 + img.left as i32, 20-img.top as i32, width, height)).unwrap();
      x += glyph.advance;
    }
  });
  
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
