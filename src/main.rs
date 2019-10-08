extern crate libc;

use std::cmp::min;
use std::ffi::CString;
use std::fmt::format;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::str::FromStr;
use std::string::String;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

extern crate portaudio_rs as portaudio;

// use portaudio as pa;
use portaudio::{device, hostapi, stream};

extern crate clap;
use clap::{App, Arg, SubCommand};

extern crate tinyosc;
use tinyosc as osc;

const CHANNELS: i32 = 2;
const NUM_SECONDS: i32 = 5;
const SAMPLE_RATE: f64 = 44100.0;
// const FRAMES_PER_BUFFER: u32 = 64;
// const FRAMES_PER_BUFFER: u32 = 2048;
const FRAMES_PER_BUFFER: u32 = 4096;

extern "C" {
  pub fn fraust_init(samplerate: i32);
  pub fn fraust_compute(count: i32, input: *const libc::c_float, output: *mut libc::c_float);
  pub fn fraust_setval(label: *const libc::c_char, val: libc::c_float);
}

enum SeType {
  SliderPress,
  SliderMove,
  SliderUnpress,
}

enum SeWhat {
  Millisecond,
  Feedback,
}

pub struct SliderEvt {
  evttype: SeType,
  what: SeWhat,
  position: f32,
}

fn main() {
  let matches = App::new("fraust-echo")
    .version("0.1")
    .author("Ben Burdette <bburdette@gmail.com>")
    .about("echo in rust using faust")
    .arg(
      Arg::with_name("input_device")
        .short("i")
        .long("input")
        .help("Sets the input device index.")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("output_device")
        .short("o")
        .long("output")
        .help("Sets the output device index.")
        .takes_value(true),
    )
    .arg(Arg::with_name("list").help("List audio devices"))
    .get_matches();

  let input_device = matches.value_of("input_device");
  let output_device = matches.value_of("output_device");
  let list = matches.value_of("list");

  // println!("args are: {:?} {:?} {:?}", input_device, output_device, list);

  portaudio::initialize().unwrap();
  if list.is_some() {
    print_devs();
  } else {
    callback_demo(
      input_device.map(|x: &str| u32::from_str(x).unwrap()),
      output_device.map(|x: &str| u32::from_str(x).unwrap()),
    );
  }
  portaudio::terminate().unwrap();
}

fn print_devs() {
  for i in 0..portaudio::device::get_count().unwrap() {
    match portaudio::device::get_info(i) {
      None => {}
      Some(info) => {
        println!("-------------------------------------------");
        println!("Device {}: ", i);
        print_device(info);
      }
    }
  }
  println!("-------------------------------------------");

  if let Some(i) = device::get_default_input_index() {
    println!("default input device index: {}", i);
  } else {
    println!("no default input device!");
  }
  if let Some(i) = device::get_default_output_index() {
    println!("default output device index: {}", i);
  } else {
    println!("no default output device!");
  }
}

fn as_millis(d: Duration) -> u64 {
  let s = u64::from(d.as_secs());
  let n = u64::from(d.subsec_nanos());
  (s * 1000) + (n / 1000000)
}

fn print_device(info: device::DeviceInfo) {
  println!("name: {}", info.name);
  println!("host_api: {}", info.host_api);
  println!("max_input_channels: {}", info.max_input_channels);
  println!("max_output_channels: {}", info.max_output_channels);
  println!(
    "default_low_input_latency: {} ms",
    as_millis(info.default_low_input_latency)
  );
  println!(
    "default_low_output_latency: {} ms",
    as_millis(info.default_low_output_latency)
  );
  println!(
    "default_high_input_latency: {} ms",
    as_millis(info.default_high_input_latency)
  );
  println!(
    "default_high_output_latency: {} ms",
    as_millis(info.default_high_output_latency)
  );
  println!("default_sample_rate: {}", info.default_sample_rate);
}

fn callback_demo(input_device: Option<u32>, output_device: Option<u32>) {
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

  unsafe {
    fraust_init(SAMPLE_RATE as i32);
  }

  let bufmax = 10000;
  let mut inflts = [0.0; 10000];
  inflts[0] = 1.0;

  let mut outflts = [0.0; 10000];

  let volstring = CString::new("Volume").unwrap();
  let millisecond = CString::new("millisecond").unwrap();
  let feedback = CString::new("feedback").unwrap();

  unsafe {
    fraust_setval(feedback.as_ptr(), 50.0);
  }
  unsafe {
    fraust_setval(millisecond.as_ptr(), 70.0);
  }
  // unsafe { fraust_setval(volstring.as_ptr(), 0.05); }

  let mut loopcount = 0;
  let mut buflen = 0;
  let bufmaxu = bufmax as usize;
  let mut bufidx = bufmaxu - 1;

  // ---------------------------------------------
  // set up portaudio callback ftn.
  // ---------------------------------------------
  let callback = Box::new(
    |input: &[f32],
     output: &mut [f32],
     _time: stream::StreamTimeInfo,
     _flags: stream::StreamCallbackFlags|
     -> stream::StreamCallbackResult {
      // println!("in the callback! frames: {}", frames);
      // any events to update the DSP with??
      match rx.try_recv() {
        Ok(se) => match se.what {
          SeWhat::Millisecond => {
            println!("setting vol to {}!", se.position);
            unsafe {
              fraust_setval(millisecond.as_ptr(), se.position);
            }
          }
          SeWhat::Feedback => {
            println!("setting vol to {}!", se.position);
            unsafe {
              fraust_setval(feedback.as_ptr(), se.position);
            }
          }
        },
        _ => {}
      }

      // do dsp!

      // TO DO: verify input buflen too, in case it has fewer or no channels, thats a segfault.

      unsafe {
        fraust_compute(output.len() as i32, input.as_ptr(), output.as_mut_ptr());
      }

      stream::StreamCallbackResult::Continue
    },
  );

  // ---------------------------------------------
  // start portaudio
  // ---------------------------------------------
  let in_idx = input_device
    .unwrap_or_else(|| device::get_default_input_index().expect("no default input device found."));
  let in_lat = match device::get_info(in_idx) {
    None => {
      println!("input device not found for index: {}", in_idx);
      return;
    }
    Some(d) => d.default_low_input_latency,
  };
  let inparams = stream::StreamParameters {
    device: in_idx,
    channel_count: 1,
    suggested_latency: in_lat,
    data: 0f32,
  };

  let out_idx = output_device.unwrap_or_else(|| {
    device::get_default_output_index().expect("no default output device found.")
  });
  let out_lat = match device::get_info(out_idx) {
    None => {
      println!("output device not found for index: {}", out_idx);
      return;
    }
    Some(d) => d.default_low_output_latency,
  };
  let outparams = stream::StreamParameters {
    device: out_idx,
    channel_count: 2,
    suggested_latency: out_lat,
    data: 0f32,
  };

  let mut stream = match stream::Stream::open(
    Some(inparams),
    Some(outparams),
    44100f64,
    stream::FRAMES_PER_BUFFER_UNSPECIFIED,
    stream::StreamFlags::empty(),
    Some(callback),
  ) {
    Err(v) => {
      println!("Err({:?})", v);
      return;
    }
    Ok(stream) => stream,
  };

  let finished_callback = Box::new(|| println!("Finshed callback called"));
  println!(
    "finished_callback: {:?}",
    stream.set_finished_callback(finished_callback)
  );
  println!("start: {:?}", stream.start());

  let oscrecvip = std::net::SocketAddr::from_str("0.0.0.0:8000").expect("Invalid IP");
  // spawn the osc receiver thread.
  match oscthread(oscrecvip, tx) {
    Ok(s) => println!("oscthread exited ok"),
    Err(e) => println!("oscthread error: {} ", e),
  };
}

fn find_location(args: &Vec<osc::Argument>) -> Option<f32> {
  let mut arg_iter = args.into_iter();
  let mut arg = arg_iter.next();

  let mut ret = None; // Option<f32>;

  while arg.is_some() {
    match arg {
      Some(&osc::Argument::s("location")) => {
        arg = arg_iter.next();
        match arg {
          Some(&osc::Argument::f(loc)) => {
            ret = Some(loc);
            break;
          }
          _ => {
            continue;
          }
        }
      }
      _ => {
        arg = arg_iter.next();
        continue;
      }
    }
  }
  ret
}

fn find_etype(args: &Vec<osc::Argument>) -> Option<SeType> {
  if args.len() >= 1 {
    match &args[0] {
      &osc::Argument::s(etype) => match etype {
        "pressed" => Some(SeType::SliderPress),
        "unpressed" => Some(SeType::SliderUnpress),
        _ => Some(SeType::SliderMove),
      },
      _ => Some(SeType::SliderMove),
    }
  } else {
    None
  }
}

fn oscthread(oscrecvip: SocketAddr, sender: mpsc::Sender<SliderEvt>) -> Result<String, Error> {
  let socket = try!(UdpSocket::bind(oscrecvip));
  let mut buf = [0; 1000];

  loop {
    let (amt, src) = try!(socket.recv_from(&mut buf));

    let inmsg = match osc::Message::deserialize(&buf[..amt]) {
      Ok(m) => m,
      Err(e) => {
        return Err(Error::new(ErrorKind::Other, "oh no!"));
      }
    };

    println!("message received {} {:?}", inmsg.path, inmsg.arguments);
    match inmsg {
      osc::Message {
        path: ref path,
        arguments: ref args,
      } => {
        let what = match path {
          &"millisecond" => Some(SeWhat::Millisecond),
          &"feedback" => Some(SeWhat::Feedback),
          _ => None,
        };

        match (what, find_etype(args), find_location(args)) {
          (Some(what), Some(sevt), Some(pos)) => {
            let amt = match what {
              SeWhat::Millisecond => pos * 500.0,
              SeWhat::Feedback => pos * 100.0,
            };

            let se = SliderEvt {
              evttype: sevt,
              what: what,
              position: amt,
            };
            sender.send(se);
          }
          _ => {}
        }
      }
    };
  }
}
