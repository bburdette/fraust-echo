extern crate libc;

use std::ffi::CString;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::io::{Error,ErrorKind};
use std::string::String;
use std::fmt::format;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::str::FromStr;
use std::cmp::min;

extern crate alsa;
use alsa::Direction;
use alsa::ValueOr;
use alsa::pcm::{PCM, HwParams, Format, Access};

// extern crate portaudio;
// use portaudio as pa;

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

type Sample = f32;

fn main() {
  match run() { 
    Err(e) => println!("error: {:?}", e),
    _ => println!("its over!"),
  }
}

fn run() -> Result<(), Box<std::error::Error> > {
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
    // init alsa 
    // ---------------------------------------------

    /*
    let config = default_config();
    let mut phases: Vec<Phase> = (*config.pitches).iter().map(|&p| phase(&config, p)).collect();
    phases.sort();
    let phases = phases;

    let phase_min = phases[0];
    let phase_max = phases[phases.len()-1];
    let sample_rate = config.sample_rate;
    let samples = phase_max * 2;

    let mut backing_vector: Vec<Sample> = Vec::with_capacity(samples);
    // Should probably use Vec::from_elem(samples, 0) but that is not in stable yet
    unsafe { backing_vector.set_len(samples); }
    let mut data = &mut backing_vector[..];
    */
    // input, output buffers.
    // let sample_count = 10000;
    let sample_count = 64;
    let period_size_try = 2048;
    let buffer_size_try = 2048;
    let sample_rate = 44100;
    // let sample_rate = 48000;

    let default = CString::new("default").unwrap();
    let nonblock = false; 

    let pcm_in = PCM::open(&*default, Direction::Capture, nonblock).unwrap();
    {
      let hwp = HwParams::any(&pcm_in).unwrap();
      hwp.set_rate(sample_rate, ValueOr::Nearest).unwrap();
      // hwp.set_period_size(period_size, ValueOr::Nearest);
      hwp.set_channels(1).unwrap();
      hwp.set_buffer_size_near(buffer_size_try).unwrap();
      hwp.set_period_size_near(period_size_try,ValueOr::Nearest).unwrap();
      hwp.set_format(Format::float()).unwrap();
      hwp.set_access(Access::RWInterleaved).unwrap();
      pcm_in.hw_params(&hwp).unwrap();
    }
    let io_in = pcm_in.io_f32().unwrap();
    pcm_in.prepare().unwrap();

    match pcm_in.hw_params_current() {
      Ok(params) => println!("hwparams: {:?}", params),
      _ => println!("failed to get params"),
    }

    let pcm_out = PCM::open(&*default, Direction::Playback, nonblock).unwrap();
    {
      let hwp = HwParams::any(&pcm_out).unwrap();
      hwp.set_rate(sample_rate, ValueOr::Nearest).unwrap();
      hwp.set_channels(1).unwrap();
      hwp.set_buffer_size_near(buffer_size_try).unwrap();
      hwp.set_period_size_near(period_size_try,ValueOr::Nearest).unwrap();
      hwp.set_format(Format::float()).unwrap();
      hwp.set_access(Access::RWInterleaved).unwrap();
      pcm_out.hw_params(&hwp).unwrap();
    }
    let io_out = pcm_out.io_f32().unwrap();
    pcm_out.prepare().unwrap();

    match pcm_out.hw_params_current() {
      Ok(params) => println!("hwparams: {:?}", params),
      _ => println!("failed to get params"),
    }
         
    let period_size = 
	match pcm_out.hw_params_current() {
	      Ok(params) => { match params.get_period_size() {
		Ok(ps) => ps, //  as i32, 
		_ => period_size_try as i32,
	 	}}
	      _ => period_size_try as i32, 
	    };
    
    let vecsize = period_size as usize;
      
    let mut input_vector: Vec<Sample> = Vec::with_capacity(vecsize);
    // Should probably use Vec::from_elem(samples, 0) but that is not in stable yet
    unsafe { input_vector.set_len(vecsize); }
    let mut inputdata = &mut input_vector[..];

    let mut output_vector: Vec<Sample> = Vec::with_capacity(vecsize);
    // Should probably use Vec::from_elem(period_size, 0) but that is not in stable yet
    unsafe { output_vector.set_len(vecsize); }
    let mut outputdata = &mut output_vector[..];

    let oscrecvip = std::net::SocketAddr::from_str("0.0.0.0:8000").expect("Invalid IP");
    // spawn the osc receiver thread. 
    thread::spawn(move || {
      match oscthread(oscrecvip, tx) {
        Ok(s) => println!("oscthread exited ok"),
        Err(e) => println!("oscthread error: {} ", e),
      }
    });

 
    // copy vals into output array.
    let mut idx = 0;
    for _ in 0..sample_count {
        outputdata[idx] = outflts[idx];
        idx += 1;
    }

    /*
    let mut val = 1;
    let period = 10;
    for _ in 0..sample_count {
      if (val > period)
      {
        val = 0;
      }

      if (val < 5)
      {
        outputdata[idx] = -1.0;
      }
      else
      {
        outputdata[idx] = 1.0;
      }
      idx += 1;
      val += 1;
    }
    */

    
     // try!(io_out.writei(inputdata));
     // try!(io_out.writei(inputdata));

    println!("instate: {:?}", pcm_in.state());
    println!("outstate: {:?}", pcm_out.state());

    // try!(io_out.writei(inputdata));
    try!(io_out.writei(inputdata));

    // let frames = 64;
    let frames = period_size;

    loop {
      // let samps = try!(io_in.readi(&mut inputdata));
      try!(io_in.readi(&mut inputdata));

      unsafe { fraust_compute(frames as i32, inputdata.as_ptr(), outputdata.as_mut_ptr()); }

      try!(io_out.writei(outputdata));
      
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

      // println!("{} {} {} {} {}", inputdata[0], inputdata[1], inputdata[2], inputdata[3], inputdata[4]);
      
      // let samps = assert_eq!(io_in.readi(&mut inputdata).unwrap(), sample_count);
      // let phase = autocorrelate(phase_min, phase_max, &data);
      // let closest_index = closest(phase, &phases);
      // VT100 escape magic to clear the current line and reset the cursor
      // print!("\x1B[2K\r");
      // print!("phase:{:>4}, freq:{:>8.3}, pitch:{:>8.3}, note: {}, string: {}", phase, sample_rate as f64 / phase as f64, frequency(&config, phase as f64), pprint_pitch(frequency(&config, phase as f64).round() as isize), closest_index + 1);

      // io_out.writei(inputdata);
      // try!(io_out.writei(outputdata));
      // try!(io_out.writei(outputdata));
      // std::io::stdout().flush().unwrap();
    }
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


