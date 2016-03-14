extern crate libc;

use std::ffi::CString;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::io::{Error,ErrorKind};
use std::string::String;
use std::fmt::format;
use std::sync::mpsc;
use std::thread;
use std::str::FromStr;
use std::cmp::min;

extern crate portaudio;

use portaudio as pa;

extern crate tinyosc;
use tinyosc as osc;

extern {
  pub fn fraust_init(samplerate: i32);
  pub fn fraust_compute(count: i32, input: *const libc::c_float, output: *mut libc::c_float );
  pub fn fraust_setval(label: *const libc::c_char , val: libc::c_float); 
}


enum SeType { 
  SliderPress,
  SliderMove,
  SliderUnpress,
} 

enum SeWhat { 
  Millisecond,
  Feedback
}

pub struct SliderEvt { 
  evttype: SeType
, what: SeWhat
, position: f32
}

const CHANNELS: i32 = 2;
const NUM_SECONDS: i32 = 5;
const SAMPLE_RATE: f64 = 44100.0;
// const FRAMES_PER_BUFFER: u32 = 64;
const FRAMES_PER_BUFFER: u32 = 2048;

fn main() {
    run().unwrap()
}


fn run() -> Result<(), pa::Error> {


    // ---------------------------------------------
    // start the osc receiver thread
    // ---------------------------------------------

    // make a channel to receive updates from the osc.
    let (tx, rx) = mpsc::channel::<SliderEvt>();

    // we'll do osc receive below, in the main thread.

    // ---------------------------------------------
    // init fraust 
    // ---------------------------------------------
    println!("initting with sample rate: {}", SAMPLE_RATE);

    unsafe { fraust_init(SAMPLE_RATE as i32); }

    let bufmax = 10000;
    let mut inflts = [0.0;10000];
    inflts[0] = 1.0;

    let mut outflts = [0.0;10000];

    let volstring = CString::new("Volume").unwrap();
    let millisecond = CString::new("millisecond").unwrap();
    let feedback = CString::new("feedback").unwrap();

    unsafe { fraust_setval(feedback.as_ptr(), 50.0); }
    unsafe { fraust_setval(millisecond.as_ptr(), 70.0); }
    // unsafe { fraust_setval(volstring.as_ptr(), 0.05); }

    let mut loopcount = 0;
    let mut buflen = 0;
    let bufmaxu = bufmax as usize;
    let mut bufidx = bufmaxu - 1;

    // make a full buffer to begin with.
    // unsafe { fraust_compute(bufmax, flts.as_mut_ptr(), outflts.as_mut_ptr()); }

    // ---------------------------------------------
    // start the portaudio process!
    // ---------------------------------------------

    let pa = try!(pa::PortAudio::new());

/*
    let mut settings = try!(pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER));
    // we won't output out of range samples so don't bother clipping them.
    settings.flags = pa::stream_flags::CLIP_OFF;
*/

    let id = pa::DeviceIndex(0);
    let inparams = pa::StreamParameters::<f32>::new(id, 2, true, 0.0);
    let outparams = pa::StreamParameters::<f32>::new(id, 2, true, 0.0);
    let mut settings = 
      pa::DuplexStreamSettings::new(inparams, outparams, SAMPLE_RATE, FRAMES_PER_BUFFER);
    settings.flags = pa::stream_flags::CLIP_OFF;

    printPaDev(id, &pa);


    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.
    let callback = move |pa::DuplexStreamCallbackArgs { in_buffer, out_buffer, frames, .. }| {
        // println!("in the callback! frames: {}", frames);
        // any events to update the DSP with?? 
        match rx.try_recv() { 
          Ok(se) => {
            match se.what { 
              SeWhat::Millisecond => { 
                  // println!("setting vol to 0.3!");
                   unsafe { fraust_setval(millisecond.as_ptr(), se.position); }
                }
              SeWhat::Feedback => { 
                  // println!("setting vol to 0.001!");
                  unsafe { fraust_setval(feedback.as_ptr(), se.position); }
                }
            }
          }
          _ => {}
        }

        if frames * 2 > bufmax
        {
          pa::Abort
        }
        else
        {
          // do dsp!
          let mut idx = 0;
          let mut ifidx = 0;

          // just get one input channel.
          for _ in 0..frames {
              inflts[idx] = in_buffer[ifidx];
              idx += 1;
              ifidx += 2;
          }
           // compute 'frames' number of samples.
          // unsafe { fraust_compute(frames as i32, in_buffer.as_ptr(), out_buffer.as_mut_ptr()); }
          unsafe { fraust_compute(frames as i32, inflts.as_ptr(), outflts.as_mut_ptr()); }
          
          idx = 0;
          let mut ofidx = 0;
          // stereo output.
          for _ in 0..frames {
              out_buffer[idx] = outflts[ofidx];
              idx += 1;
              out_buffer[idx] = outflts[ofidx];
              idx += 1;
              ofidx += 1;
          }

          /*
          // passthrough!
          let mut idx = 0;
          for i in 0..frames { 
            out_buffer[idx] = in_buffer[idx];
            idx = idx + 1;
            out_buffer[idx] = in_buffer[idx];
            idx = idx + 1;
          }
          */



          pa::Continue
        }
    };

    let mut stream = try!(pa.open_non_blocking_stream(settings, callback));

    try!(stream.start());

    let oscrecvip = std::net::SocketAddr::from_str("0.0.0.0:8000").expect("Invalid IP");
    // spawn the osc receiver thread. 
    match oscthread(oscrecvip, tx) {
      Ok(s) => println!("oscthread exited ok"),
      Err(e) => println!("oscthread error: {} ", e),
    };

    /*
    loop {
      println!("Play for {} seconds.", NUM_SECONDS);
      pa.sleep(NUM_SECONDS * 1_000);
    }
    */

    try!(stream.stop());
    try!(stream.close());

    println!("its over!");

    Ok(())
}


fn oscthread(oscrecvip: SocketAddr, sender: mpsc::Sender<SliderEvt>) -> Result<String, Error> { 
  let socket = try!(UdpSocket::bind(oscrecvip));
  let mut buf = [0; 1000];

  loop { 
    let (amt, src) = try!(socket.recv_from(&mut buf));

    // println!("length: {}", amt);
    let inmsg = match osc::Message::deserialize(&buf[.. amt]) {
      Ok(m) => m,
      Err(e) => {
          return Err(Error::new(ErrorKind::Other, "oh no!"));
        },
      };

    // println!("message received {} {:?}", inmsg.path, inmsg.arguments );
    match inmsg {
      osc::Message { path: ref path, arguments: ref args } => {
        if args.len() > 1 {
          match (&args[0], &args[1]) {
            (&osc::Argument::s(etype), &osc::Argument::f(pos)) => {
            let what = match path { 
              &"millisecond" => Some(SeWhat::Millisecond),
              &"feedback" => Some(SeWhat::Feedback),
              _ => None,
              };

            let setype = match etype { 
              "s_pressed" => Some(SeType::SliderPress),
              "s_unpressed" => Some(SeType::SliderUnpress),
              "s_moved" => Some(SeType::SliderMove),
              _ => None,
              };

            match (what, setype) { 
              (Some(what), Some(sevt)) => { 
                let amt = match what { 
                  SeWhat::Millisecond => pos * 500.0,
                  SeWhat::Feedback => pos * 100.0,
                  };
                
                let se = SliderEvt{ evttype: sevt, what: what, position: amt };
                sender.send(se)
              }
              _ => Ok(())
            }
            },
            _ => Ok(())
          } 
        }
        else {
          Ok(())
        }
      },
      };
    };

  // drop(socket); // close the socket
  // Ok(String::from("meh"))
}

const INTERLEAVED: bool = true;
const LATENCY: pa::Time = 0.0; // Ignored by PortAudio::is_*_format_supported.
const STANDARD_SAMPLE_RATES: [f64; 13] = [
    8000.0, 9600.0, 11025.0, 12000.0, 16000.0, 22050.0, 24000.0, 32000.0,
    44100.0, 48000.0, 88200.0, 96000.0, 192000.0,
];

fn printPaDev(idx: pa::DeviceIndex, pado: &pa::PortAudio) -> Result<(), pa::Error> {
  let info = try!(pado.device_info(idx));
  println!("--------------------------------------- {:?}", idx);
  println!("{:#?}", &info);

  let in_channels = info.max_input_channels;
  let input_params = 
    pa::StreamParameters::<i16>::new(idx, in_channels, INTERLEAVED, LATENCY);
  let out_channels = info.max_output_channels;
  let output_params = 
    pa::StreamParameters::<i16>::new(idx, out_channels, INTERLEAVED, LATENCY);

  println!("Supported standard sample rates for half-duplex 16-bit {} channel input:", 
    in_channels);
  for &sample_rate in &STANDARD_SAMPLE_RATES {
    if pado.is_input_format_supported(input_params, sample_rate).is_ok() {
        println!("\t{}hz", sample_rate);
    }
  }

  println!("Supported standard sample rates for half-duplex 16-bit {} channel output:", 
    out_channels);
  for &sample_rate in &STANDARD_SAMPLE_RATES {
    if pado.is_output_format_supported(output_params, sample_rate).is_ok() {
        println!("\t{}hz", sample_rate);
    }
  }

  println!("Supported standard sample rates for full-duplex 16-bit {} channel input, {} channel output:",
     in_channels, out_channels);
  for &sample_rate in &STANDARD_SAMPLE_RATES {
    if pado.is_duplex_format_supported(input_params, output_params, sample_rate).is_ok() {
        println!("\t{}hz", sample_rate);
    }
  }

  Ok(())
}

