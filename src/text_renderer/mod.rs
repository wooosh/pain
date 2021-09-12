extern crate sdl2;
extern crate swash;
extern crate lru;

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use::core::arch::x86_64::*;
use std::mem;

use swash::{Attributes, CacheKey, Charmap, FontRef, scale};

// max number of glyphs in glyph cache
const GLYPH_CACHE_SIZE: usize = 256;
// TODO: profile latency of creating a scaler and the memory usage, and maybe scale this based on the number of fonts
const SCALER_CACHE_SIZE: usize = 16;

pub type FontId = swash::CacheKey;
// pixels per em
pub type FontSize = u32;

pub struct FontData {
  // Font file
  data:  Vec<u8>,
  // Offset to the table directory
  offset: u32,
  // Cache key
  key: swash::CacheKey,
  // Scaler cache

}

impl FontData {
  pub fn from_file(path: &str, index: usize) -> Option<Self> {
      // Read the full font file
      let data = std::fs::read(path).ok()?;
      // Create a temporary font reference for the first font in the file.
      // This will do some basic validation, compute the necessary offset
      // and generate a fresh cache key for us.
      let font = FontRef::from_index(&data, index)?;
      let (offset, key) = (font.offset, font.key);
      // Return our struct with the original file data and copies of the
      // offset and key from the font reference
      Some(Self { data, offset, key })
  }

  // As a convenience, you may want to forward some methods.
  pub fn attributes(&self) -> Attributes {
      self.as_ref().attributes()
  }

  pub fn charmap(&self) -> Charmap {
      self.as_ref().charmap()
  }

  // Create the transient font reference for accessing this crate's
  // functionality.
  pub fn as_ref(&self) -> swash::FontRef {
      // Note that you'll want to initialize the struct directly here as
      // using any of the FontRef constructors will generate a new key which,
      // while completely safe, will nullify the performance optimizations of
      // the caching mechanisms used in this crate.
      swash::FontRef {
          data: &self.data,
          offset: self.offset,
          key: self.key
      }
  }
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct GlyphCacheKey (FontId, FontSize, swash::GlyphId);

// ABGR buffer
// TODO: cow ptr
#[derive(Clone)]
pub struct ImageBuffer {
  pub data: Vec<u8>,
  pub width: u32,
  pub height: u32
}

// Manages loaded fonts, and keeps a glyph cache
pub struct FontManager {
  //fonts: HashMap<FontId, FontData>,
  // TODO: will glyphs die if evicted from the cache because of the lifetime?
  // stores cached subpixel masks
  glyph_cache: lru::LruCache<GlyphCacheKey, ImageBuffer>,
  // TODO: does scaler get dropped if its evicted from the cache?
  scale_context: swash::scale::ScaleContext
}

//TODO: move to FontRefs everywhere
impl FontManager {
  pub fn default() -> Self {
    FontManager {
      //fonts: HashMap::new(),
      glyph_cache: lru::LruCache::new(GLYPH_CACHE_SIZE),
      scale_context: scale::ScaleContext::new()
    }
  }

  /*
  pub fn add_font_from_file(&mut self, path: &str) -> Option<swash::FontRef> {
    let font = FontData::from_file(path, 0)?;
    let fref = font.as_ref();
    self.fonts.insert(font.key, font);
    Some(fref)
  }*/

  // TODO: maybe user provided buffer?
  pub fn render_glyph(&mut self, font: swash::FontRef, size: FontSize, glyph: swash::GlyphId, textcolor: [u8; 4], destcolor: [u8; 4]) -> ImageBuffer {
    let mut img = self.get_glyph_mask(font, size, glyph);
    Self::apply_mask(&mut img, textcolor, destcolor);
    img
  }

  // TODO: make this return an immutable reference to the glyph mask
  fn get_glyph_mask<'a>(&mut self, font: swash::FontRef, size: FontSize, glyph_id: swash::GlyphId) -> ImageBuffer {
    let key = GlyphCacheKey (font.key, size, glyph_id);
    let x = match self.glyph_cache.get(&key) {
        Some(img) => img,
        None => {
          let mut scaler = self.scale_context.builder(font)
            .size(size as f32)
            .hint(false)
            .build();
          let img = Self::scale_glyph(&mut scaler, glyph_id);
          // TODO: fix this. so stupid
          self.glyph_cache.put(key.clone(), img);
          self.glyph_cache.get(&key).unwrap()
        }
    };
    x.clone()
  }

  fn scale_glyph(scaler: &mut swash::scale::Scaler<'_>, glyph_id: swash::GlyphId) -> ImageBuffer {
    let offset = zeno::Vector::new(0., 0.);
    let img = scale::Render::new(&[
      // list of sources in the font for the renderer to try to find
      scale::Source::ColorOutline(0),
      scale::Source::ColorBitmap(scale::StrikeWith::BestFit),
      scale::Source::Outline,
    ])
    .format(zeno::Format::Subpixel)
    //.format(zeno::Format::CustomSubpixel([0., -0.6, 0.3]))
    .offset(offset)
    .default_color([255, 255, 255, 255])
    .render(scaler, glyph_id).unwrap();

    // TODO: align this array for SIMD
    let mut img_data = img.data.to_vec();
    // set alpha channel to the green channel, which is the original outline
    for px in img_data.chunks_mut(4) {
      px[3] = px[1];
    }

    ImageBuffer {
      data: img_data,
      width: img.placement.width,
      height: img.placement.height
    }
  }

  // TODO: better type than array of 4 u8s for color
  // TODO: avx2
  // TODO: feature detection
  fn apply_mask(img: &mut ImageBuffer, textcolor: [u8; 4], destcolor: [u8; 4]) {
    fn composite_color(textcolor: u8, textalpha: u8, maskcolor: u8, destcolor: u8) -> u8 {
      /*
        https://github.com/servo/webrender/blob/master/webrender/doc/text-rendering.md
        Calculate the following equation using fixed point:
          textcolor * maskcolor + (1.0 - textalpha * maskcolor) * destcolor * 255.0
    
          TODO: cache based on destcolor and alpha so that the right side of the equation can be completely cached
          (textcolor * maskcolor) + (destcolor * (0xff00 - ((textalpha * maskcolor) >> 8))) >> 8
    
      */
      
      (
        (textcolor as u16 * maskcolor as u16) + 
        (destcolor as u16 * 
          ((0xff00 - (textalpha as u16 * maskcolor as u16)) >> 8))
         >> 8) as u8
    }

    // TODO: document simd code and make it feature detecting
    unsafe {
      #[repr(C)]
      union Simd16x8 {
        array: [u8; 16],
        simd: __m128i
      }

      // constants
      let vzero = _mm_setzero_si128();

      let vtext_color = _mm_set1_epi32(i32::from_ne_bytes(textcolor));
      let vtext_color = _mm_unpacklo_epi8(vtext_color, vzero);

      let vdest_color = _mm_set1_epi32(i32::from_ne_bytes(textcolor));
      let vdest_color = _mm_unpacklo_epi8(vtext_color, vzero);

      let vtext_alpha = _mm_set1_epi8(textcolor[3] as i8);

      // fixed point representation of one
      // 0xff00 == -256
      let fixpt_one = _mm_set1_epi16(-256);

      for px in img.data.chunks_mut(16) {
        //if px.len() < 16 {
            for p in px.chunks_mut(4) {

              p[3] = composite_color(textcolor[3], textcolor[3], p[3], destcolor[3]);

              p[0] = composite_color(textcolor[0], textcolor[3], p[0], destcolor[0]);
              p[1] = composite_color(textcolor[1], textcolor[3], p[1], destcolor[1]);
              p[2] = composite_color(textcolor[2], textcolor[3], p[2], destcolor[2]);
            }
            /*
        } else {
          let vsubpx_mask = _mm_loadu_si128(&px[0] as *const _ as *const __m128i);
          let vsubpx_mask = _mm_unpacklo_epi8(vsubpx_mask, vzero);

          let left = _mm_mullo_epi16(vsubpx_mask, vtext_color);

          let results = _mm_mullo_epi16(vtext_alpha, vsubpx_mask);
          let results = _mm_sub_epi16(fixpt_one, results);
          let results = _mm_srli_epi16(results, 8);
          let results = _mm_mullo_epi16(results, vdest_color);
          let results = _mm_add_epi16(results, left);
          let results_lo = _mm_srli_epi16(results, 8);

          // repeat of above with the high bits of the mask
          let vsubpx_mask = _mm_loadu_si128(&px[0] as *const _ as *const __m128i);
          let vsubpx_mask = _mm_unpackhi_epi8(vsubpx_mask, vzero);

          let left = _mm_mullo_epi16(vsubpx_mask, vtext_color);

          let results = _mm_mullo_epi16(vtext_alpha, vsubpx_mask);
          let results = _mm_sub_epi16(fixpt_one, results);
          let results = _mm_srli_epi16(results, 8);
          let results = _mm_mullo_epi16(results, vdest_color);
          let results = _mm_add_epi16(results, left);
          let results_hi = _mm_srli_epi16(results, 8);

          let results = _mm_packus_epi16(results_lo, results_hi);

          _mm_storeu_si128(&mut px[0] as *mut _ as *mut __m128i, results);
        }*/
      }
    }
  }
  
  /*
  fn composite_glyph<'a>(scaler: &mut swash::scale::Scaler<'_>, glyph_id: swash::GlyphId) -> sdl2::surface::Surface<'a> {
    // TODO: remove unwraps
    // TODO: clean up this functions
    // TODO: take text color and destcolor
    let offset = zeno::Vector::new(0., 0.);
    let img = scale::Render::new(&[
      // list of sources in the font for the renderer to try to find
      scale::Source::ColorOutline(0),
      scale::Source::ColorBitmap(scale::StrikeWith::BestFit),
      scale::Source::Outline,
    ])
    .format(zeno::Format::Subpixel)
    //.format(zeno::Format::CustomSubpixel([0., -0.6, 0.3]))
    .offset(offset)
    .default_color([255, 255, 255, 255])
    .render(scaler, glyph_id).unwrap();

    let glyph_img = &mut
    let mut imgdata = &mut img.data;


    // set alpha channel to the green channel
    for px in imgdata.chunks_mut(4) {
      px[3] = px[1];
    }

    fn composite_color(textcolor: u8, textalpha: u8, maskcolor: u8, destcolor: u8) -> u8 {
      /*
        https://github.com/servo/webrender/blob/master/webrender/doc/text-rendering.md
        Calculate the following equation using fixed point:
          textcolor * maskcolor + (1.0 - textalpha * maskcolor) * destcolor * 255.0
    
          TODO: cache based on destcolor and alpha so that the right side of the equation can be completely cached
          (textcolor * maskcolor) + (destcolor * (0xff00 - ((textalpha * maskcolor) >> 8))) >> 8
    
      */
      
      (
        (textcolor as u16 * maskcolor as u16) + 
        (destcolor as u16 * 
          ((0xff00 - (textalpha as u16 * maskcolor as u16)) >> 8))
         >> 8) as u8
    }

    // TODO: document simd code and make it feature detecting
    unsafe {
      #[repr(C)]
      union Simd16x8 {
        array: [u8; 16],
        simd: __m128i
      }

      // constants
      let vzero = _mm_setzero_si128();

      let vtext_color = _mm_set1_epi32(i32::from_ne_bytes(textcolor));
      let vtext_color = _mm_unpacklo_epi8(vtext_color, vzero);

      let vdest_color = _mm_set1_epi32(i32::from_ne_bytes(textcolor));
      let vdest_color = _mm_unpacklo_epi8(vtext_color, vzero);

      let vtext_alpha = _mm_set1_epi8(textcolor[3] as i8);

      // fixed point representation of one
      // 0xff00 == -256
      let fixpt_one = _mm_set1_epi16(-256);

      for px in imgdata.chunks_mut(16) {
        if px.len() < 16 {
            for p in px.chunks_mut(4) {

              p[3] = composite_color(textcolor[3], textcolor[3], p[3], destcolor[3]);

              p[0] = composite_color(textcolor[0], textcolor[3], p[0], destcolor[0]);
              p[1] = composite_color(textcolor[1], textcolor[3], p[1], destcolor[1]);
              p[2] = composite_color(textcolor[2], textcolor[3], p[2], destcolor[2]);
            }
        } else {
          let vsubpx_mask = _mm_loadu_si128(&px[0] as *const _ as *const __m128i);
          let vsubpx_mask = _mm_unpacklo_epi8(vsubpx_mask, vzero);

          let left = _mm_mullo_epi16(vsubpx_mask, vtext_color);

          let results = _mm_mullo_epi16(vtext_alpha, vsubpx_mask);
          let results = _mm_sub_epi16(fixpt_one, results);
          let results = _mm_srli_epi16(results, 8);
          let results = _mm_mullo_epi16(results, vdest_color);
          let results = _mm_add_epi16(results, left);
          let results_lo = _mm_srli_epi16(results, 8);

          // repeat of above with the high bits of the mask
          let vsubpx_mask = _mm_loadu_si128(&px[0] as *const _ as *const __m128i);
          let vsubpx_mask = _mm_unpackhi_epi8(vsubpx_mask, vzero);

          let left = _mm_mullo_epi16(vsubpx_mask, vtext_color);

          let results = _mm_mullo_epi16(vtext_alpha, vsubpx_mask);
          let results = _mm_sub_epi16(fixpt_one, results);
          let results = _mm_srli_epi16(results, 8);
          let results = _mm_mullo_epi16(results, vdest_color);
          let results = _mm_add_epi16(results, left);
          let results_hi = _mm_srli_epi16(results, 8);

          let results = _mm_packus_epi16(results_lo, results_hi);

          _mm_storeu_si128(&mut px[0] as *mut _ as *mut __m128i, results);
        }
      
    }
    
    let surface = sdl2::surface::Surface::from_data(
      &mut imgdata, 
      img.placement.width, 
      img.placement.height, 
      img.placement.width*4,
      // NOTE: if font rendering looks weird, it is probably meant to be ABGR
      sdl2::pixels::PixelFormatEnum::ABGR8888).unwrap();

      surface
    }
  }*/
}


/*

*/
pub fn write_text() {
    println!("test");
}